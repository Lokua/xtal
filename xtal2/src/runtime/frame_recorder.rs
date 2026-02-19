use std::collections::VecDeque;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use log::{error, info, warn};

const DEFAULT_NUM_BUFFERS: usize = 6;
const USE_BLOCKING_MAP_WAIT: bool = true;

enum WriterMessage {
    Frame(usize),
    Stop,
}

pub struct RecordingStats {
    pub frames_captured: u32,
    pub frames_dropped: u32,
    pub output_path: String,
}

struct WriterThreadArgs {
    device: Arc<wgpu::Device>,
    buffers: Vec<Arc<wgpu::Buffer>>,
    ffmpeg_stdin: std::process::ChildStdin,
    frame_rx: mpsc::Receiver<WriterMessage>,
    buffer_return_tx: mpsc::Sender<usize>,
    has_padding: bool,
    unpadded_bytes_per_row: u32,
    padded_bytes_per_row: u32,
    height: u32,
}

pub struct FrameRecorder {
    buffers: Vec<Arc<wgpu::Buffer>>,
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
    available_buffers: VecDeque<usize>,
    buffer_return_rx: mpsc::Receiver<usize>,
    writer_tx: mpsc::SyncSender<WriterMessage>,
    writer_thread: Option<thread::JoinHandle<()>>,
    pending_submit_buffers: VecDeque<usize>,
    ffmpeg_process: Option<Child>,
    frames_captured: u32,
    frames_dropped: u32,
    output_path: String,
}

impl FrameRecorder {
    pub fn new(
        device: Arc<wgpu::Device>,
        output_path: &str,
        width: u32,
        height: u32,
        fps: f32,
        source_format: wgpu::TextureFormat,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let ffmpeg_pix_fmt = ffmpeg_input_pixel_format(source_format)
            .ok_or_else(|| {
                format!(
                    "unsupported recording source format: {:?}",
                    source_format
                )
            })?;

        let ffmpeg_preset = std::env::var("XTAL_RECORDING_PRESET")
            .unwrap_or_else(|_| "veryfast".to_string());
        let num_buffers = std::env::var("XTAL_RECORDING_NUM_BUFFERS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|&count| count >= 2)
            .unwrap_or(DEFAULT_NUM_BUFFERS);

        let mut ffmpeg = Command::new("ffmpeg")
            .args([
                "-y",
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostats",
                "-f",
                "rawvideo",
                "-pix_fmt",
                ffmpeg_pix_fmt,
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
                ffmpeg_preset.as_str(),
                "-pix_fmt",
                "yuv420p",
                output_path,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        let ffmpeg_stdin =
            ffmpeg.stdin.take().ok_or("Failed to open ffmpeg stdin")?;

        let (buffer_return_tx, buffer_return_rx) = mpsc::channel();
        let (writer_tx, writer_rx) =
            mpsc::sync_channel::<WriterMessage>(num_buffers);

        let bytes_per_pixel = 4u32;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let padded_bytes_per_row =
            unpadded_bytes_per_row + compute_row_padding(unpadded_bytes_per_row);

        let buffer_size = (padded_bytes_per_row as u64) * (height as u64);
        let buffers: Vec<Arc<wgpu::Buffer>> = (0..num_buffers)
            .map(|i| {
                Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("recording_readback_{}", i)),
                    size: buffer_size,
                    usage: wgpu::BufferUsages::COPY_DST
                        | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }))
            })
            .collect();

        let mut available_buffers = VecDeque::with_capacity(num_buffers);
        for i in 0..num_buffers {
            available_buffers.push_back(i);
        }

        let writer_buffers = buffers.clone();
        let writer_device = device.clone();
        let has_padding = padded_bytes_per_row != unpadded_bytes_per_row;
        let w_unpadded = unpadded_bytes_per_row;
        let w_padded = padded_bytes_per_row;
        let h = height;

        let writer_thread_args = WriterThreadArgs {
            device: writer_device,
            buffers: writer_buffers,
            ffmpeg_stdin,
            frame_rx: writer_rx,
            buffer_return_tx,
            has_padding,
            unpadded_bytes_per_row: w_unpadded,
            padded_bytes_per_row: w_padded,
            height: h,
        };

        let writer_thread = thread::spawn(move || {
            writer_thread_fn(writer_thread_args);
        });

        Ok(Self {
            buffers,
            width,
            height,
            padded_bytes_per_row,
            available_buffers,
            buffer_return_rx,
            writer_tx,
            writer_thread: Some(writer_thread),
            pending_submit_buffers: VecDeque::new(),
            ffmpeg_process: Some(ffmpeg),
            frames_captured: 0,
            frames_dropped: 0,
            output_path: output_path.to_string(),
        })
    }

    pub fn capture_surface_frame(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        source_texture: &wgpu::Texture,
    ) -> bool {
        while let Ok(idx) = self.buffer_return_rx.try_recv() {
            self.available_buffers.push_back(idx);
        }

        let buffer_index = if let Some(idx) = self.available_buffers.pop_front()
        {
            idx
        } else {
            let start = Instant::now();
            let idx = match self.buffer_return_rx.recv() {
                Ok(idx) => idx,
                Err(_) => {
                    error!("Recording: writer thread disconnected");
                    return false;
                }
            };
            let waited_ms = start.elapsed().as_secs_f64() * 1000.0;
            if waited_ms > 16.0 {
                warn!(
                    "Recording: waited {:.1}ms for free readback buffer",
                    waited_ms
                );
            }
            idx
        };

        let src_size = source_texture.size();
        if src_size.width != self.width || src_size.height != self.height {
            self.frames_dropped += 1;
            self.available_buffers.push_back(buffer_index);
            warn!(
                "Recording: skipping frame due to size mismatch source={}x{} recorder={}x{}",
                src_size.width, src_size.height, self.width, self.height
            );
            return false;
        }

        let buffer = &self.buffers[buffer_index];
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: source_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer,
                layout: wgpu::TexelCopyBufferLayout {
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

        self.pending_submit_buffers.push_back(buffer_index);

        self.frames_captured += 1;
        true
    }

    pub fn on_submitted(&mut self) {
        while let Some(buffer_index) = self.pending_submit_buffers.pop_front() {
            if self
                .writer_tx
                .send(WriterMessage::Frame(buffer_index))
                .is_err()
            {
                self.available_buffers.push_back(buffer_index);
                error!("Recording: writer thread disconnected");
                break;
            }
        }
    }

    pub fn stop(mut self) -> RecordingStats {
        let _ = self.writer_tx.send(WriterMessage::Stop);

        if let Some(handle) = self.writer_thread.take() {
            if let Err(err) = handle.join() {
                error!("Writer thread panicked: {:?}", err);
            }
        }

        if let Some(mut process) = self.ffmpeg_process.take() {
            match process.wait() {
                Ok(status) => {
                    if !status.success() {
                        error!("ffmpeg exited with status: {}", status);
                    }
                }
                Err(err) => {
                    error!("Failed to wait for ffmpeg: {}", err);
                }
            }
        }

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

fn writer_thread_fn(mut args: WriterThreadArgs) {
    let mut contiguous_frame = args.has_padding.then(|| {
        vec![0u8; (args.unpadded_bytes_per_row * args.height) as usize]
    });

    loop {
        match args.frame_rx.recv() {
            Ok(WriterMessage::Frame(buffer_index)) => {
                let buffer = &args.buffers[buffer_index];
                let slice = buffer.slice(..);

                let (map_tx, map_rx) = mpsc::sync_channel(1);
                slice.map_async(wgpu::MapMode::Read, move |result| {
                    let _ = map_tx.send(result);
                });

                let map_result = if USE_BLOCKING_MAP_WAIT {
                    let _ = args.device.poll(wgpu::PollType::Wait);
                    map_rx.recv().ok()
                } else {
                    loop {
                        match map_rx.try_recv() {
                            Ok(result) => break Some(result),
                            Err(mpsc::TryRecvError::Empty) => {
                                let _ =
                                    args.device.poll(wgpu::PollType::Poll);
                                thread::sleep(std::time::Duration::from_micros(
                                    250,
                                ));
                            }
                            Err(mpsc::TryRecvError::Disconnected) => {
                                break None;
                            }
                        }
                    }
                };

                match map_result {
                    Some(Ok(())) => {
                        let data = slice.get_mapped_range();

                        let write_ok = if let Some(frame_bytes) =
                            contiguous_frame.as_mut()
                        {
                            copy_padded_rows_to_contiguous(
                                &data,
                                frame_bytes,
                                args.height,
                                args.unpadded_bytes_per_row,
                                args.padded_bytes_per_row,
                            );
                            args.ffmpeg_stdin.write_all(frame_bytes).is_ok()
                        } else {
                            args.ffmpeg_stdin.write_all(&data).is_ok()
                        };

                        drop(data);
                        buffer.unmap();

                        if !write_ok {
                            error!("Failed to write frame to ffmpeg");
                            let _ = args.buffer_return_tx.send(buffer_index);
                            return;
                        }
                    }
                    Some(Err(err)) => {
                        error!("Buffer mapping failed: {:?}", err);
                    }
                    None => {
                        error!("Buffer map channel disconnected");
                    }
                }

                let _ = args.buffer_return_tx.send(buffer_index);
            }
            Ok(WriterMessage::Stop) | Err(_) => {
                drop(args.ffmpeg_stdin);
                return;
            }
        }
    }
}

fn copy_padded_rows_to_contiguous(
    data: &[u8],
    out: &mut [u8],
    height: u32,
    unpadded_bytes_per_row: u32,
    padded_bytes_per_row: u32,
) {
    let unpadded_bytes_per_row = unpadded_bytes_per_row as usize;
    let padded_bytes_per_row = padded_bytes_per_row as usize;
    debug_assert_eq!(out.len(), unpadded_bytes_per_row * height as usize);

    for row in 0..height {
        let row = row as usize;
        let src_start = row * padded_bytes_per_row;
        let src_end = src_start + unpadded_bytes_per_row;
        let dst_start = row * unpadded_bytes_per_row;
        let dst_end = dst_start + unpadded_bytes_per_row;
        out[dst_start..dst_end].copy_from_slice(&data[src_start..src_end]);
    }
}

fn compute_row_padding(unpadded_bytes_per_row: u32) -> u32 {
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let rem = unpadded_bytes_per_row % align;
    if rem == 0 { 0 } else { align - rem }
}

fn ffmpeg_input_pixel_format(
    format: wgpu::TextureFormat,
) -> Option<&'static str> {
    match format {
        wgpu::TextureFormat::Bgra8Unorm
        | wgpu::TextureFormat::Bgra8UnormSrgb => Some("bgra"),
        wgpu::TextureFormat::Rgba8Unorm
        | wgpu::TextureFormat::Rgba8UnormSrgb => Some("rgba"),
        _ => None,
    }
}
