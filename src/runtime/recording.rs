use std::{
    cell::Cell,
    error::Error,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Command, Stdio},
    str,
    sync::mpsc,
    thread,
};

use super::prelude::*;
use crate::framework::prelude::*;

#[derive(Default)]
pub struct RecordingState {
    pub is_recording: bool,
    pub is_encoding: bool,
    pub is_queued: bool,
    pub recorded_frames: Cell<u32>,
    pub recording_dir: Option<PathBuf>,
    pub encoding_thread: Option<thread::JoinHandle<()>>,
    pub encoding_progress_rx: Option<mpsc::Receiver<EncodingMessage>>,
}

impl RecordingState {
    pub fn new(recording_dir: Option<PathBuf>) -> Self {
        Self {
            recording_dir,
            recorded_frames: Cell::new(0),
            ..Default::default()
        }
    }

    pub fn start_recording(
        &mut self,
        alert_text: &mut String,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(path) = &self.recording_dir {
            self.is_recording = true;
            let message =
                format!("Recording. Frames will be written to {:?}", path);
            *alert_text = message;
            info!("Recording started, path: {:?}", path);
            Ok(())
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

    pub fn toggle_recording(
        &mut self,
        sketch_config: &SketchConfig,
        session_id: &str,
        alert_text: &mut String,
    ) -> Result<(), Box<dyn Error>> {
        if self.is_recording {
            self.stop_recording(sketch_config, session_id)
        } else {
            self.start_recording(alert_text)
        }
    }

    pub fn on_encoding_message(
        &mut self,
        session_id: &mut String,
        sketch_config: &SketchConfig,
        alert_text: &mut String,
    ) {
        if let Some(rx) = self.encoding_progress_rx.take() {
            while let Ok(message) = rx.try_recv() {
                match message {
                    EncodingMessage::Progress(progress) => {
                        let percentage = (progress * 100.0).round();
                        debug!("rx progress: {}%", percentage);
                        *alert_text =
                            format!("Encoding progress: {}%", percentage)
                                .into();
                    }
                    EncodingMessage::Complete => {
                        info!("Encoding complete");
                        self.is_encoding = false;
                        self.encoding_progress_rx = None;
                        let output_path =
                            video_output_path(session_id, sketch_config.name)
                                .unwrap()
                                .to_string_lossy()
                                .into_owned();
                        *alert_text = format!(
                            "Encoding complete. Video path {}",
                            output_path
                        )
                        .into();
                        *session_id = generate_session_id();
                        self.recorded_frames.set(0);
                        if let Some(new_path) =
                            frames_dir(session_id, sketch_config.name)
                        {
                            self.recording_dir = Some(new_path);
                        }
                    }
                    EncodingMessage::Error(error) => {
                        error!("Received child process error: {}", error);
                        *alert_text =
                            format!("Received encoding error: {}", error);
                    }
                }
            }
            self.encoding_progress_rx = Some(rx);
        }
    }
}

pub fn frames_dir(session_id: &str, sketch_name: &str) -> Option<PathBuf> {
    lattice_config_dir().map(|config_dir| {
        config_dir
            .join("Captures")
            .join(sketch_name)
            .join(session_id)
    })
}

pub fn video_output_path(
    session_id: &str,
    sketch_name: &str,
) -> Option<PathBuf> {
    dirs::video_dir().map(|video_dir| {
        video_dir
            .join(format!("{}-{}", sketch_name, session_id))
            .with_extension("mp4")
    })
}

pub enum EncodingMessage {
    /// Progress updates as a percentage (0.0 to 1.0)
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
            "-n", // Don't overwrite
            "-loglevel",
            "level+info",
            "-framerate",
            &fps.to_string(),
            "-i",
            &format!("{}/frame-%06d.png", frame_dir),
            "-c:v",
            "libx264",
            "-crf",
            "16",
            "-preset",
            "veryslow",
            "-pix_fmt",
            "yuv420p",
            "-progress",
            "pipe:1",
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
        for line in stderr_reader.lines() {
            if let Ok(line) = line {
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
        }
        Ok(())
    });

    for line in stdout_reader.lines() {
        if let Ok(line) = line {
            if line.starts_with("frame=") {
                let frame_str = line[6..].split_whitespace().next();
                if let Ok(frame) = frame_str.unwrap().parse::<u32>() {
                    let progress = frame as f32 / total_frames as f32;
                    debug!("frames_to_video progress: {}", progress);
                    let message = EncodingMessage::Progress(progress);
                    progress_sender.send(message)?;
                }
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

pub fn frames_to_video_stub(
    frame_dir: &str,
    fps: f32,
    output_path: &str,
    total_frames: u32,
    progress_sender: mpsc::Sender<EncodingMessage>,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::{thread, time};

    debug!(
        "Simulating: frame_dir = {}, fps = {}, output_path = {}, total_frames = {}",
        frame_dir, fps, output_path, total_frames
    );

    let duration = time::Duration::from_millis(10);
    let steps = 100; // Total steps to simulate
    let step_progress = 1.0 / steps as f32;

    for step in 0..=steps {
        let progress = step as f32 * step_progress;
        progress_sender.send(EncodingMessage::Progress(progress))?;
        debug!("frames_to_video_stub progress: {}", progress);
        thread::sleep(duration);
    }

    debug!("Simulated video encoding complete");

    if progress_sender.send(EncodingMessage::Complete).is_err() {
        warn!("Completion receiver dropped");
    }

    Ok(())
}
