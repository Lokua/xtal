use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

use crate::framework::util::uuid_5;
use crate::runtime::frame_recorder::FrameRecorder;

#[derive(Default)]
pub struct RecordingState {
    pub is_recording: bool,
    pub is_encoding: bool,
    pub is_queued: bool,
    pub frame_recorder: Option<FrameRecorder>,
    finalize_rx: Option<mpsc::Receiver<FinalizeMessage>>,
}

struct FinalizeMessage {
    frames_captured: u32,
    frames_dropped: u32,
    output_path: String,
}

pub struct FinalizeOutcome {
    pub is_error: bool,
    pub message: String,
}

impl RecordingState {
    pub fn start_recording(
        &mut self,
        device: Arc<wgpu::Device>,
        output_path: &str,
        width: u32,
        height: u32,
        fps: f32,
        source_format: wgpu::TextureFormat,
    ) -> Result<String, Box<dyn Error>> {
        let recorder = FrameRecorder::new(
            device,
            output_path,
            width,
            height,
            fps,
            source_format,
        )?;
        self.frame_recorder = Some(recorder);
        self.is_recording = true;
        let message = format!("Recording to {}", output_path);
        log::info!("{}", message);
        Ok(message)
    }

    pub fn stop_recording(&mut self) -> Result<(), Box<dyn Error>> {
        self.is_recording = false;
        self.is_queued = false;

        let recorder =
            self.frame_recorder.take().ok_or("No active recorder")?;

        self.is_encoding = true;

        let (finalize_tx, finalize_rx) = mpsc::channel();
        self.finalize_rx = Some(finalize_rx);

        thread::spawn(move || {
            let stats = recorder.stop();
            let _ = finalize_tx.send(FinalizeMessage {
                frames_captured: stats.frames_captured,
                frames_dropped: stats.frames_dropped,
                output_path: stats.output_path,
            });
        });

        Ok(())
    }

    pub fn poll_finalize(
        &mut self,
        session_id: &mut String,
    ) -> Option<FinalizeOutcome> {
        let message = if let Some(rx) = &self.finalize_rx {
            rx.try_recv().ok()
        } else {
            None
        };

        match message {
            Some(FinalizeMessage {
                frames_captured,
                frames_dropped,
                output_path,
            }) => {
                self.is_encoding = false;
                self.finalize_rx = None;
                *session_id = generate_session_id();

                let drop_info = if frames_dropped > 0 {
                    format!(" ({} frames dropped)", frames_dropped)
                } else {
                    String::new()
                };

                Some(FinalizeOutcome {
                    is_error: false,
                    message: format!(
                        "Recording complete. {} frames captured{}. Video: {}",
                        frames_captured, drop_info, output_path
                    ),
                })
            }
            None => None,
        }
    }
}

pub fn generate_session_id() -> String {
    uuid_5()
}

pub fn video_output_path(
    videos_dir: &str,
    session_id: &str,
    sketch_name: &str,
) -> PathBuf {
    PathBuf::from(videos_dir)
        .join(format!("{}-{}", sketch_name, session_id))
        .with_extension("mp4")
}
