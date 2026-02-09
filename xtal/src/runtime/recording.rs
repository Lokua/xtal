use std::cell::RefCell;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

use nannou::wgpu;

use super::app;
use crate::framework::prelude::*;
use crate::runtime::app::AppEvent;
use crate::runtime::frame_recorder::FrameRecorder;
use crate::runtime::global;

pub struct RecordingState {
    pub is_recording: bool,
    pub is_encoding: bool,
    pub is_queued: bool,
    pub frame_recorder: RefCell<Option<FrameRecorder>>,
    finalize_rx: Option<mpsc::Receiver<FinalizeMessage>>,
}

#[allow(dead_code)]
enum FinalizeMessage {
    Complete {
        frames_captured: u32,
        frames_dropped: u32,
        output_path: String,
    },
    Error(String),
}

impl Default for RecordingState {
    fn default() -> Self {
        Self {
            is_recording: false,
            is_encoding: false,
            is_queued: false,
            frame_recorder: RefCell::new(None),
            finalize_rx: None,
        }
    }
}

impl RecordingState {
    pub fn start_recording(
        &mut self,
        device_queue_pair: &Arc<wgpu::DeviceQueuePair>,
        output_path: &str,
        width: u32,
        height: u32,
        fps: f32,
    ) -> Result<String, Box<dyn Error>> {
        let recorder = FrameRecorder::new(
            device_queue_pair,
            output_path,
            width,
            height,
            fps,
        )?;
        *self.frame_recorder.borrow_mut() = Some(recorder);
        self.is_recording = true;
        let message = format!("Recording to {}", output_path);
        info!("{}", message);
        Ok(message)
    }

    pub fn stop_recording(
        &mut self,
        event_tx: &app::AppEventSender,
    ) -> Result<(), Box<dyn Error>> {
        self.is_recording = false;
        self.is_queued = false;

        let recorder = self
            .frame_recorder
            .borrow_mut()
            .take()
            .ok_or("No active recorder")?;

        self.is_encoding = true;

        let (finalize_tx, finalize_rx) = mpsc::channel();
        self.finalize_rx = Some(finalize_rx);

        let tx = event_tx.clone();
        thread::spawn(move || {
            let stats = recorder.stop();
            let _ = finalize_tx.send(FinalizeMessage::Complete {
                frames_captured: stats.frames_captured,
                frames_dropped: stats.frames_dropped,
                output_path: stats.output_path,
            });
            // The main thread will pick this up via poll_finalize
            drop(tx);
        });

        Ok(())
    }

    pub fn poll_finalize(
        &mut self,
        session_id: &mut String,
        event_tx: &app::AppEventSender,
    ) {
        let message = if let Some(rx) = &self.finalize_rx {
            rx.try_recv().ok()
        } else {
            None
        };

        if let Some(message) = message {
            match message {
                FinalizeMessage::Complete {
                    frames_captured,
                    frames_dropped,
                    output_path,
                } => {
                    self.is_encoding = false;
                    self.finalize_rx = None;
                    let drop_info = if frames_dropped > 0 {
                        format!(" ({} frames dropped)", frames_dropped)
                    } else {
                        String::new()
                    };
                    event_tx.alert(format!(
                        "Recording complete. {} frames captured{}. \
                         Video: {}",
                        frames_captured, drop_info, output_path
                    ));
                    event_tx.emit(AppEvent::EncodingComplete);
                    *session_id = generate_session_id();
                }
                FinalizeMessage::Error(error) => {
                    self.is_encoding = false;
                    self.finalize_rx = None;
                    let message = format!("Recording error: {}", error);
                    event_tx.alert(message.clone());
                    error!("{}", message);
                    *session_id = generate_session_id();
                }
            }
        }
    }
}

pub fn generate_session_id() -> String {
    uuid_5()
}

pub fn video_output_path(
    session_id: &str,
    sketch_name: &str,
) -> Option<PathBuf> {
    Some(
        PathBuf::from(global::videos_dir())
            .join(format!("{}-{}", sketch_name, session_id))
            .with_extension("mp4"),
    )
}
