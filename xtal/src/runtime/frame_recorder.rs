use std::collections::VecDeque;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use log::{error, info, warn};
use nannou::prelude::*;
use nannou::wgpu;

const NUM_BUFFERS: usize = 3;
const DST_FORMAT: wgpu::TextureFormat =
    wgpu::TextureFormat::Rgba8UnormSrgb;

enum WriterMessage {
    Frame(usize),
    Stop,
}

pub struct RecordingStats {
    pub frames_captured: u32,
    pub frames_dropped: u32,
    pub output_path: String,
}

pub struct FrameRecorder {
    // GPU resources (created lazily)
    reshaper: Option<wgpu::TextureReshaper>,
    dst_texture: Option<wgpu::Texture>,
    buffers: Vec<Arc<wgpu::Buffer>>,

    // Dimensions
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,

    // Buffer rotation
    available_buffers: VecDeque<usize>,
    buffer_return_rx: mpsc::Receiver<usize>,

    // Writer thread communication
    writer_tx: mpsc::SyncSender<WriterMessage>,
    writer_thread: Option<thread::JoinHandle<()>>,

    // ffmpeg process (for waiting on exit)
    ffmpeg_process: Option<Child>,

    // Stats
    frames_captured: u32,
    frames_dropped: u32,
    output_path: String,
}

impl FrameRecorder {
    pub fn new(
        device_queue_pair: &Arc<wgpu::DeviceQueuePair>,
        output_path: &str,
        width: u32,
        height: u32,
        fps: f32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut ffmpeg = Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgba",
                "-s",
                &format!("{}x{}", width, height),
                "-r",
                &fps.to_string(),
                "-i",
                "pipe:0",
                "-c:v",
                "libx264",
                "-crf",
                "16",
                "-preset",
                "medium",
                "-pix_fmt",
                "yuv420p",
                output_path,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;

        let ffmpeg_stdin = ffmpeg
            .stdin
            .take()
            .ok_or("Failed to open ffmpeg stdin")?;

        // Channel for writer thread to return used buffers
        let (buffer_return_tx, buffer_return_rx) = mpsc::channel();

        // Bounded channel for frame messages (backpressure)
        let (writer_tx, writer_rx) =
            mpsc::sync_channel::<WriterMessage>(NUM_BUFFERS);

        // Calculate row padding
        let bytes_per_pixel = 4u32;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let padding =
            wgpu::compute_row_padding(unpadded_bytes_per_row);
        let padded_bytes_per_row = unpadded_bytes_per_row + padding;

        // Create readback buffers
        let device = device_queue_pair.device();
        let buffer_size =
            (padded_bytes_per_row as u64) * (height as u64);
        let buffers: Vec<Arc<wgpu::Buffer>> = (0..NUM_BUFFERS)
            .map(|i| {
                Arc::new(device.create_buffer(
                    &wgpu::BufferDescriptor {
                        label: Some(&format!(
                            "recording_readback_{}",
                            i
                        )),
                        size: buffer_size,
                        usage: wgpu::BufferUsages::COPY_DST
                            | wgpu::BufferUsages::MAP_READ,
                        mapped_at_creation: false,
                    },
                ))
            })
            .collect();

        let mut available_buffers = VecDeque::with_capacity(NUM_BUFFERS);
        for i in 0..NUM_BUFFERS {
            available_buffers.push_back(i);
        }

        // Spawn writer thread
        let writer_buffers = buffers.clone();
        let writer_dqp = device_queue_pair.clone();
        let has_padding = padding > 0;
        let w_unpadded = unpadded_bytes_per_row;
        let w_padded = padded_bytes_per_row;
        let h = height;

        let writer_thread = thread::spawn(move || {
            writer_thread_fn(
                writer_dqp,
                writer_buffers,
                ffmpeg_stdin,
                writer_rx,
                buffer_return_tx,
                has_padding,
                w_unpadded,
                w_padded,
                h,
            );
        });

        Ok(Self {
            reshaper: None,
            dst_texture: None,
            buffers,
            width,
            height,
            padded_bytes_per_row,
            available_buffers,
            buffer_return_rx,
            writer_tx,
            writer_thread: Some(writer_thread),
            ffmpeg_process: Some(ffmpeg),
            frames_captured: 0,
            frames_dropped: 0,
            output_path: output_path.to_string(),
        })
    }

    /// Must be called while the Frame is still alive so we can
    /// access the resolved texture view for the reshaper bind group.
    pub fn ensure_gpu_resources(
        &mut self,
        device: &wgpu::Device,
        frame: &Frame,
    ) {
        let [w, h] = frame.texture_size();
        if self.reshaper.is_some() && self.width == w && self.height == h
        {
            return;
        }

        // Get the resolved (non-MSAA) texture view
        let src_view: &wgpu::TextureViewHandle =
            match frame.resolve_target() {
                Some(view) => view,
                None => frame.texture_view(),
            };

        let src_sample_type =
            wgpu::TextureSampleType::Float { filterable: true };

        let reshaper = wgpu::TextureReshaper::new(
            device,
            src_view,
            1, // resolved, not multisampled
            src_sample_type,
            1,
            DST_FORMAT,
        );

        let dst_texture = wgpu::TextureBuilder::new()
            .size([w, h])
            .format(DST_FORMAT)
            .usage(
                wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC,
            )
            .sample_count(1)
            .build(device);

        self.reshaper = Some(reshaper);
        self.dst_texture = Some(dst_texture);
        self.width = w;
        self.height = h;
    }

    /// Called after the Frame has been dropped (GPU commands
    /// submitted). Encodes a reshape + copy in a new submission and
    /// sends the buffer to the writer thread.
    pub fn capture_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        // Reclaim returned buffers
        while let Ok(idx) = self.buffer_return_rx.try_recv() {
            self.available_buffers.push_back(idx);
        }

        // Get an available buffer, blocking briefly if needed
        let buffer_index = if let Some(idx) =
            self.available_buffers.pop_front()
        {
            idx
        } else {
            // All buffers in flight — block briefly waiting for one
            let start = Instant::now();
            let timeout = Duration::from_millis(50);
            loop {
                match self.buffer_return_rx.recv_timeout(
                    Duration::from_millis(1),
                ) {
                    Ok(idx) => break idx,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if start.elapsed() > timeout {
                            self.frames_dropped += 1;
                            warn!(
                                "Recording: dropped frame \
                                 (all buffers busy for >50ms)"
                            );
                            return;
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        error!(
                            "Recording: writer thread disconnected"
                        );
                        return;
                    }
                }
            }
        };

        let reshaper = match &self.reshaper {
            Some(r) => r,
            None => return,
        };
        let dst_texture = match &self.dst_texture {
            Some(t) => t,
            None => return,
        };
        let buffer = &self.buffers[buffer_index];

        let mut encoder = device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("recording_capture"),
            },
        );

        // Reshape: Rgba16Float intermediary -> Rgba8UnormSrgb
        let dst_view = dst_texture.view().build();
        reshaper.encode_render_pass(&dst_view, &mut encoder);

        // Copy texture to readback buffer
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: dst_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(self.padded_bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        queue.submit(std::iter::once(encoder.finish()));

        // Send to writer thread
        if let Err(e) =
            self.writer_tx.try_send(WriterMessage::Frame(buffer_index))
        {
            match e {
                mpsc::TrySendError::Full(_) => {
                    self.available_buffers.push_back(buffer_index);
                    self.frames_dropped += 1;
                    warn!("Recording: dropped frame (writer backed up)");
                }
                mpsc::TrySendError::Disconnected(_) => {
                    error!("Recording: writer thread disconnected");
                }
            }
            return;
        }

        self.frames_captured += 1;
    }

    pub fn stop(mut self) -> RecordingStats {
        // Signal writer to stop
        let _ = self.writer_tx.send(WriterMessage::Stop);

        // Wait for writer thread to finish
        if let Some(handle) = self.writer_thread.take() {
            if let Err(e) = handle.join() {
                error!("Writer thread panicked: {:?}", e);
            }
        }

        // Wait for ffmpeg to finish
        if let Some(mut process) = self.ffmpeg_process.take() {
            match process.wait() {
                Ok(status) => {
                    if !status.success() {
                        error!(
                            "ffmpeg exited with status: {}",
                            status
                        );
                    }
                }
                Err(e) => {
                    error!("Failed to wait for ffmpeg: {}", e);
                }
            }
        }

        // Clean up GPU resources
        self.reshaper = None;
        self.dst_texture = None;

        info!(
            "Recording complete: {} frames captured, {} dropped",
            self.frames_captured, self.frames_dropped
        );

        RecordingStats {
            frames_captured: self.frames_captured,
            frames_dropped: self.frames_dropped,
            output_path: self.output_path.clone(),
        }
    }
}

fn writer_thread_fn(
    device_queue_pair: Arc<wgpu::DeviceQueuePair>,
    buffers: Vec<Arc<wgpu::Buffer>>,
    mut ffmpeg_stdin: std::process::ChildStdin,
    frame_rx: mpsc::Receiver<WriterMessage>,
    buffer_return_tx: mpsc::Sender<usize>,
    has_padding: bool,
    unpadded_bytes_per_row: u32,
    padded_bytes_per_row: u32,
    height: u32,
) {
    let device = device_queue_pair.device();

    loop {
        match frame_rx.recv() {
            Ok(WriterMessage::Frame(buffer_index)) => {
                let buffer = &buffers[buffer_index];
                let slice = buffer.slice(..);

                let (map_tx, map_rx) = mpsc::channel();
                slice.map_async(wgpu::MapMode::Read, move |result| {
                    let _ = map_tx.send(result);
                });

                // Poll until mapping completes
                device.poll(wgpu::Maintain::Wait);

                match map_rx.recv() {
                    Ok(Ok(())) => {
                        let data = slice.get_mapped_range();
                        let write_ok = if has_padding {
                            write_padded_rows(
                                &mut ffmpeg_stdin,
                                &data,
                                height,
                                unpadded_bytes_per_row,
                                padded_bytes_per_row,
                            )
                        } else {
                            ffmpeg_stdin.write_all(&data).is_ok()
                        };
                        drop(data);
                        buffer.unmap();

                        if !write_ok {
                            error!(
                                "Failed to write frame to ffmpeg"
                            );
                            let _ =
                                buffer_return_tx.send(buffer_index);
                            return;
                        }
                    }
                    Ok(Err(e)) => {
                        error!("Buffer mapping failed: {:?}", e);
                    }
                    Err(_) => {
                        error!("Buffer map channel disconnected");
                    }
                }

                let _ = buffer_return_tx.send(buffer_index);
            }
            Ok(WriterMessage::Stop) | Err(_) => {
                // Close stdin to signal EOF to ffmpeg
                drop(ffmpeg_stdin);
                return;
            }
        }
    }
}

fn write_padded_rows(
    writer: &mut impl Write,
    data: &[u8],
    height: u32,
    unpadded_bytes_per_row: u32,
    padded_bytes_per_row: u32,
) -> bool {
    for row in 0..height {
        let start = (row * padded_bytes_per_row) as usize;
        let end = start + unpadded_bytes_per_row as usize;
        if writer.write_all(&data[start..end]).is_err() {
            return false;
        }
    }
    true
}
