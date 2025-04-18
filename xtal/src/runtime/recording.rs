use std::cell::Cell;
use std::error::Error;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use super::app;
use super::storage::cache_dir;
use crate::framework::prelude::*;
use crate::runtime::app::AppEvent;
use crate::runtime::global;

#[derive(Debug)]
pub struct RecordingState {
    pub is_recording: bool,
    pub is_encoding: bool,
    pub is_queued: bool,
    pub recorded_frames: Cell<u32>,
    pub recording_dir: Option<PathBuf>,
    pub encoding_thread: Option<thread::JoinHandle<()>>,
    pub encoding_progress_rx: Option<mpsc::Receiver<EncodingMessage>>,
    pub encoding_start: Option<Instant>,
}

impl Default for RecordingState {
    fn default() -> Self {
        Self {
            is_recording: false,
            is_encoding: false,
            is_queued: false,
            recorded_frames: Cell::new(0),
            recording_dir: Some(PathBuf::from(global::videos_dir())),
            encoding_thread: None,
            encoding_progress_rx: None,
            encoding_start: None,
        }
    }
}

impl RecordingState {
    pub fn new(recording_dir: Option<PathBuf>) -> Self {
        Self {
            recording_dir,
            ..Default::default()
        }
    }

    pub fn start_recording(&mut self) -> Result<String, Box<dyn Error>> {
        if let Some(path) = &self.recording_dir {
            self.is_recording = true;
            let message =
                format!("Recording. Frames will be written to {:?}", path);
            info!("{}", message.clone());
            Ok(message)
        } else {
            Err("Unable to access recording path".into())
        }
    }

    pub fn stop_recording(
        &mut self,
        sketch_config: &SketchConfig,
        session_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        if !self.is_encoding {
            self.is_recording = false;
            self.is_queued = false;
            self.is_encoding = true;

            let (encoding_progress_tx, rx) = mpsc::channel();
            self.encoding_progress_rx = Some(rx);

            let path = self
                .recording_dir
                .as_ref()
                .ok_or("No recording directory")?
                .to_string_lossy()
                .into_owned();

            let output_path = video_output_path(session_id, sketch_config.name)
                .ok_or("Could not determine output path")?
                .to_string_lossy()
                .into_owned();

            let fps = sketch_config.fps;
            let total_frames = self.recorded_frames.get();

            info!("Preparing to encode. Output path: {}", output_path);
            debug!("Spawning encoding_thread");

            self.encoding_start = Some(Instant::now());
            self.encoding_thread = Some(thread::spawn(move || {
                if let Err(e) = frames_to_video(
                    &path,
                    fps,
                    &output_path,
                    total_frames,
                    encoding_progress_tx,
                ) {
                    error!("Error in frames_to_video: {:?}", e);
                }
            }));

            Ok(())
        } else {
            Err("Already encoding".into())
        }
    }

    pub fn on_encoding_message(
        &mut self,
        sketch_config: &SketchConfig,
        session_id: &mut String,
        event_tx: &app::AppEventSender,
    ) {
        if let Some(rx) = self.encoding_progress_rx.take() {
            while let Ok(message) = rx.try_recv() {
                match message {
                    EncodingMessage::Progress(progress) => {
                        let percentage = (progress * 100.0).round();
                        debug!("rx progress: {}%", percentage);
                        event_tx.alert(format!(
                            "Encoding progress: {}%",
                            percentage
                        ));
                    }
                    EncodingMessage::Complete => {
                        info!("Encoding complete");
                        if let Some(start_time) = self.encoding_start.take() {
                            let duration = start_time.elapsed();
                            let secs = duration.as_secs();
                            info!(
                                "Encoding duration: {}m {}s",
                                secs / 60,
                                secs % 60
                            );
                        }
                        self.is_encoding = false;
                        self.encoding_progress_rx = None;
                        let output_path =
                            video_output_path(session_id, sketch_config.name)
                                .unwrap()
                                .to_string_lossy()
                                .into_owned();
                        event_tx.alert(format!(
                            "Encoding complete. Video path: {}",
                            output_path
                        ));
                        event_tx.emit(AppEvent::EncodingComplete);
                        *session_id = generate_session_id();
                        self.recorded_frames.set(0);
                        if let Some(new_path) =
                            frames_dir(session_id, sketch_config.name)
                        {
                            self.recording_dir = Some(new_path);
                        }
                    }
                    EncodingMessage::Error(error) => {
                        let message =
                            format!("Received encoding error: {}", error);
                        event_tx.alert(message.clone());
                        error!("{}", message);
                    }
                }
            }
            self.encoding_progress_rx = Some(rx);
        }
    }
}

/// Used to differentiate multiple recordings for the same base sketch name
pub fn generate_session_id() -> String {
    uuid_5()
}

/// Location of individual, temporary frame captures that will later be stitched
/// into a single video
pub fn frames_dir(session_id: &str, sketch_name: &str) -> Option<PathBuf> {
    cache_dir().map(|config_dir| {
        config_dir
            .join("Captures")
            .join(sketch_name)
            .join(session_id)
    })
}

/// Path to the final encoded mp4 video
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

pub enum EncodingMessage {
    /// Progress updates as a percentage [0.0, 1.0]
    Progress(f32),
    Complete,
    Error(String),
}

pub fn frames_to_video(
    frame_dir: &str,
    fps: f32,
    output_path: &str,
    total_frames: u32,
    progress_sender: mpsc::Sender<EncodingMessage>,
) -> Result<(), Box<dyn std::error::Error>> {
    let process = Command::new("ffmpeg")
        .args([
            // Don't overwrite
            "-n",
            // ---
            "-loglevel",
            "level+info",
            // ---
            "-framerate",
            &fps.to_string(),
            // ---
            "-i",
            &format!("{}/frame-%06d.png", frame_dir),
            // ---
            "-c:v",
            "libx264",
            // ---
            "-crf",
            // Very high quality
            "16",
            // "18",
            // Better compression, still visually lossless (supposedly)
            // "23",
            // ---
            "-preset",
            // "medium",
            "slow",
            // "veryslow",
            // ---
            "-pix_fmt",
            "yuv420p",
            // ---
            "-progress",
            "pipe:1",
            // ---
            // -maxrate sets the maximum bitrate the encoder can use at any point.
            // -bufsize controls how strictly that limit is enforced - it's the size of the
            // buffer used for bitrate constraints.
            // Setting both to 20M (20 megabits/sec) provides high quality while
            // preventing excessive file sizes and unusual bitrate spikes
            // "-maxrate",
            // "20M",
            // "-bufsize",
            // "20M",
            // ---
            output_path,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    debug!("ffmpeg process spawned");

    let stdout = process.stdout.unwrap();
    let stdout_reader = BufReader::new(stdout);

    let stderr = process.stderr.unwrap();
    let stderr_reader = BufReader::new(stderr);
    let error_sender = progress_sender.clone();

    let error_thread = thread::spawn(move || -> Result<(), String> {
        for line in stderr_reader.lines().map_while(Result::ok) {
            debug!("stderr line: {}", line);
            if line.contains("warning") {
                warn!("Detected ffmpeg warning: {}", line);
            } else if line.contains("warning") || line.contains("fatal") {
                error!("Detected ffmpeg error: {}", line);
                let message = EncodingMessage::Error(line.clone());
                let _ = error_sender.send(message);
                return Err(line);
            }
        }
        Ok(())
    });

    for line in stdout_reader.lines().map_while(Result::ok) {
        if line.starts_with("frame=") {
            let frame_str = line
                .strip_prefix("frame=")
                .unwrap()
                .split_whitespace()
                .next();
            if let Ok(frame) = frame_str.unwrap().parse::<u32>() {
                let progress = frame as f32 / total_frames as f32;
                debug!("frames_to_video progress: {}", progress);
                let message = EncodingMessage::Progress(progress);
                progress_sender.send(message)?;
            }
        }
    }

    match error_thread.join() {
        Ok(Ok(())) => {
            if progress_sender.send(EncodingMessage::Complete).is_err() {
                warn!("Completion receiver dropped");
            }
        }
        Ok(Err(_)) => {}
        Err(err) => {
            error!("Error thread panicked: {:?}", err);
        }
    }

    Ok(())
}
