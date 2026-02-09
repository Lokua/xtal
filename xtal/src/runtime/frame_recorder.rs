//! Realtime frame recording pipeline for Xtal.
//!
//! # Recording Report Guide
//!
//! When the `recording-report` feature is enabled (default), the recorder logs a
//! multi-line summary after recording stops:
//!
//! ```text
//! Recording report:
//! frames(...)
//! duration(...)
//! fps_rolling90(...)
//! waits(...)
//! ```
//!
//! Read it top-to-bottom:
//!
//! - `frames(captured, dropped)`: number of frames successfully enqueued for
//!   encoding, and dropped frames. For sync-sensitive capture, `dropped=0` is
//!   the goal.
//! - `duration(wall, expected, slowdown)`: real elapsed time vs ideal time
//!   (`captured / target_fps`). `slowdown=1.00x` is perfect realtime; values
//!   above `1.00x` mean recording fell behind.
//! - `fps_rolling90(avg, min, max)`: FPS stats over the last 90 captured frame
//!   intervals. This matches the smoothing horizon used in frame controller
//!   style monitoring and avoids misleading whole-run min/max outliers.
//! - `waits(count, total_ms, max_ms)`: times the render thread had to wait for
//!   a free readback buffer. Non-zero waits indicate capture backpressure.
//!
//! Practical rule of thumb:
//!
//! - Stable recording: `slowdown` near `1.00x`, `dropped=0`, and `waits=0`.
//! - If waits rise or slowdown drifts upward, use a faster preset, reduce
//!   resolution/fps, or increase `XTAL_RECORDING_NUM_BUFFERS` for bursty loads.
//!
use std::collections::VecDeque;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use log::{error, info, warn};
use nannou::prelude::*;
use nannou::wgpu;

const DEFAULT_NUM_BUFFERS: usize = 6;
const DST_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

// Blocking Map Wait vs Sleep + Poll is essentially a tie, but Blocking Map Wait
// is simpler to reason about.
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
    device_queue_pair: Arc<wgpu::DeviceQueuePair>,
    buffers: Vec<Arc<wgpu::Buffer>>,
    ffmpeg_stdin: std::process::ChildStdin,
    frame_rx: mpsc::Receiver<WriterMessage>,
    buffer_return_tx: mpsc::Sender<usize>,
    has_padding: bool,
    unpadded_bytes_per_row: u32,
    padded_bytes_per_row: u32,
    height: u32,
}

#[cfg(feature = "recording-report")]
struct RecordingReport {
    recording_start: Instant,
    target_fps: f32,

    // Per-frame intervals (main thread) — used for rolling-90 FPS stats
    last_capture_time: Option<Instant>,
    frame_intervals_ms: VecDeque<f64>,

    // Buffer wait times (main thread) — when all buffers are in-flight
    buffer_wait_count: u32,
    buffer_wait_total_ms: f64,
    buffer_wait_max_ms: f64,
}

#[cfg(feature = "recording-report")]
impl RecordingReport {
    const ROLLING_WINDOW: usize = 90;
    fn new(target_fps: f32) -> Self {
        Self {
            recording_start: Instant::now(),
            target_fps,
            last_capture_time: None,
            frame_intervals_ms: VecDeque::with_capacity(Self::ROLLING_WINDOW),
            buffer_wait_count: 0,
            buffer_wait_total_ms: 0.0,
            buffer_wait_max_ms: 0.0,
        }
    }

    fn on_capture(&mut self) {
        let now = Instant::now();
        if let Some(last) = self.last_capture_time {
            self.frame_intervals_ms
                .push_back(now.duration_since(last).as_secs_f64() * 1000.0);
            if self.frame_intervals_ms.len() > Self::ROLLING_WINDOW {
                self.frame_intervals_ms.pop_front();
            }
        }
        self.last_capture_time = Some(now);
    }

    fn on_buffer_wait(&mut self, waited_ms: f64) {
        self.buffer_wait_count += 1;
        self.buffer_wait_total_ms += waited_ms;
        self.buffer_wait_max_ms = self.buffer_wait_max_ms.max(waited_ms);
    }

    fn print(self, frames_captured: u32, frames_dropped: u32) {
        let wall_clock_s = self.recording_start.elapsed().as_secs_f64();
        let expected_s = frames_captured as f64 / self.target_fps as f64;

        let (rolling_avg_fps, rolling_min_fps, rolling_max_fps) =
            rolling_fps_stats(&self.frame_intervals_ms);

        let slowdown = if expected_s > 0.0 {
            wall_clock_s / expected_s
        } else {
            0.0
        };

        let frames_status = if frames_dropped == 0 { "PASS" } else { "FAIL" };
        let duration_status = if slowdown <= 1.02 {
            "PASS"
        } else if slowdown <= 1.05 {
            "WARN"
        } else {
            "FAIL"
        };

        let fps_avg_target = self.target_fps as f64 - 1.0;
        let fps_min_target = self.target_fps as f64 * 0.92;
        let fps_status = if rolling_avg_fps >= fps_avg_target
            && rolling_min_fps >= fps_min_target
        {
            "PASS"
        } else {
            "FAIL"
        };

        let waits_status = if self.buffer_wait_count == 0 {
            "PASS"
        } else if self.buffer_wait_max_ms <= 16.67 {
            "WARN"
        } else {
            "FAIL"
        };

        info!(
            "Recording report:\n\
             frames(captured={}, dropped={}) target(dropped=0) status({})\n\
             duration(wall={:.2}s, expected={:.2}s, slowdown={:.2}x) target(pass<=1.02x, warn<=1.05x) status({})\n\
             fps_rolling90(avg={:.1}, min={:.1}, max={:.1}) target(avg>={:.1}, min>={:.1}) status({})\n\
             waits(count={}, total_ms={:.1}, max_ms={:.2}) target(count=0) status({})",
            frames_captured,
            frames_dropped,
            frames_status,
            wall_clock_s,
            expected_s,
            slowdown,
            duration_status,
            rolling_avg_fps,
            rolling_min_fps,
            rolling_max_fps,
            fps_avg_target,
            fps_min_target,
            fps_status,
            self.buffer_wait_count,
            self.buffer_wait_total_ms,
            self.buffer_wait_max_ms,
            waits_status,
        );
    }
}

#[cfg(feature = "recording-report")]
fn rolling_fps_stats(intervals_ms: &VecDeque<f64>) -> (f64, f64, f64) {
    if intervals_ms.is_empty() {
        return (0.0, 0.0, 0.0);
    }

    let fps_values: Vec<f64> =
        intervals_ms.iter().map(|ms| 1000.0 / ms).collect();
    let sum: f64 = fps_values.iter().sum();
    let avg = sum / fps_values.len() as f64;
    let min = fps_values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = fps_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (avg, min, max)
}

pub struct FrameRecorder {
    // GPU resources (created lazily)
    reshaper: Option<wgpu::TextureReshaper>,
    dst_texture: Option<wgpu::Texture>,
    dst_texture_view: Option<wgpu::TextureView>,
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

    #[cfg(feature = "recording-report")]
    report: Option<RecordingReport>,
}

impl FrameRecorder {
    pub fn new(
        device_queue_pair: &Arc<wgpu::DeviceQueuePair>,
        output_path: &str,
        width: u32,
        height: u32,
        fps: f32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
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

        // Channel for writer thread to return used buffers
        let (buffer_return_tx, buffer_return_rx) = mpsc::channel();

        // Bounded channel for frame messages (backpressure)
        let (writer_tx, writer_rx) =
            mpsc::sync_channel::<WriterMessage>(num_buffers);

        // Calculate row padding
        let bytes_per_pixel = 4u32;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let padding = wgpu::compute_row_padding(unpadded_bytes_per_row);
        let padded_bytes_per_row = unpadded_bytes_per_row + padding;

        // Create readback buffers
        let device = device_queue_pair.device();
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

        // Spawn writer thread
        let writer_buffers = buffers.clone();
        let writer_dqp = device_queue_pair.clone();
        let has_padding = padding > 0;
        let w_unpadded = unpadded_bytes_per_row;
        let w_padded = padded_bytes_per_row;
        let h = height;

        let writer_thread_args = WriterThreadArgs {
            device_queue_pair: writer_dqp,
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
            reshaper: None,
            dst_texture: None,
            dst_texture_view: None,
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
            #[cfg(feature = "recording-report")]
            report: Some(RecordingReport::new(fps)),
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
        if self.reshaper.is_some() && self.width == w && self.height == h {
            return;
        }

        // Get the resolved (non-MSAA) texture view
        let src_view: &wgpu::TextureViewHandle = match frame.resolve_target() {
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
        self.dst_texture_view = self
            .dst_texture
            .as_ref()
            .map(|texture| texture.view().build());
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
        #[cfg(feature = "recording-report")]
        if let Some(report) = &mut self.report {
            report.on_capture();
        }

        // Reclaim returned buffers
        while let Ok(idx) = self.buffer_return_rx.try_recv() {
            self.available_buffers.push_back(idx);
        }

        // Get an available buffer. Block until one is free to avoid
        // dropping frames and preserve sync with audio.
        let buffer_index = if let Some(idx) = self.available_buffers.pop_front()
        {
            idx
        } else {
            let start = Instant::now();
            let idx = match self.buffer_return_rx.recv() {
                Ok(idx) => idx,
                Err(_) => {
                    error!("Recording: writer thread disconnected");
                    return;
                }
            };
            let waited_ms = start.elapsed().as_secs_f64() * 1000.0;
            if waited_ms > 16.0 {
                warn!(
                    "Recording: waited {:.1}ms for free readback buffer",
                    waited_ms
                );
            }
            #[cfg(feature = "recording-report")]
            if let Some(report) = &mut self.report {
                report.on_buffer_wait(waited_ms);
            }
            idx
        };

        let reshaper = match &self.reshaper {
            Some(r) => r,
            None => return,
        };
        let dst_texture = match &self.dst_texture {
            Some(t) => t,
            None => return,
        };
        let dst_view = match &self.dst_texture_view {
            Some(v) => v,
            None => return,
        };
        let buffer = &self.buffers[buffer_index];

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("recording_capture"),
            });

        // Reshape: Rgba16Float intermediary -> Rgba8UnormSrgb
        reshaper.encode_render_pass(dst_view, &mut encoder);

        // Copy texture to readback buffer
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: dst_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer,
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

        // Send to writer thread. Block here for backpressure instead
        // of dropping.
        if self
            .writer_tx
            .send(WriterMessage::Frame(buffer_index))
            .is_err()
        {
            self.available_buffers.push_back(buffer_index);
            error!("Recording: writer thread disconnected");
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
                        error!("ffmpeg exited with status: {}", status);
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
        self.dst_texture_view = None;

        #[cfg(feature = "recording-report")]
        if let Some(report) = self.report.take() {
            report.print(self.frames_captured, self.frames_dropped);
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
    let device = args.device_queue_pair.device();
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
                    // A/B option B: block until mapped callbacks are serviced.
                    device.poll(wgpu::Maintain::Wait);
                    map_rx.recv().ok()
                } else {
                    // A/B option A: poll + short sleep to avoid long blocking.
                    loop {
                        match map_rx.try_recv() {
                            Ok(result) => break Some(result),
                            Err(mpsc::TryRecvError::Empty) => {
                                device.poll(wgpu::Maintain::Poll);
                                thread::sleep(Duration::from_micros(250));
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
                    Some(Err(e)) => {
                        error!("Buffer mapping failed: {:?}", e);
                    }
                    None => {
                        error!("Buffer map channel disconnected");
                    }
                }

                let _ = args.buffer_return_tx.send(buffer_index);
            }
            Ok(WriterMessage::Stop) | Err(_) => {
                // Close stdin to signal EOF to ffmpeg
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
