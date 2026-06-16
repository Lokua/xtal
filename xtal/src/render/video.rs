//! GStreamer-backed video textures for render graphs.
//!
//! A graph video resource can contain one or more files. `VideoSource` keeps a
//! `VideoPipeline` per file, selects the active one from `VideoTransport`, and
//! returns RGBA frames that the GPU executor uploads into a texture.
//!
//! Transport is beat-based, not wall-time-based. Runtime controls define the
//! source name, file index, normalized start point, loop length in beats,
//! playback speed, and direction. This module translates those controls into
//! GStreamer play, pause, and seek operations while avoiding repeated seeks for
//! equivalent transport state.

use std::path::Path;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use gstreamer as gst;
use gstreamer::glib;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;
use log::{debug, warn};

use crate::control::{VideoDirection, VideoTransport};
use crate::warn_once;

static GST_INIT: OnceLock<Result<(), String>> = OnceLock::new();
const START_SEEK_THROTTLE: Duration = Duration::from_millis(50);
const NANOSECONDS_PER_SECOND: f32 = 1_000_000_000.0;

/// Decoded RGBA frame ready for upload into a `wgpu` texture.
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Concrete seek command planned from beat transport state.
///
/// GStreamer reverse playback requires both a start and stop bound. Forward
/// playback can be unbounded, or bounded when a ping-pong segment needs a
/// precise turnaround point.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PlannedSeek {
    rate: f64,
    start: gst::ClockTime,
    stop: Option<gst::ClockTime>,
    accurate_forward: bool,
}

impl PlannedSeek {
    /// Creates an unbounded forward seek.
    fn forward(
        rate: f64,
        start: gst::ClockTime,
        accurate_forward: bool,
    ) -> Self {
        Self {
            rate,
            start,
            stop: None,
            accurate_forward,
        }
    }

    /// Creates a bounded forward seek for the first half of ping-pong motion.
    fn forward_segment(
        rate: f64,
        start: gst::ClockTime,
        stop: gst::ClockTime,
    ) -> Self {
        Self {
            rate,
            start,
            stop: Some(stop),
            accurate_forward: true,
        }
    }

    /// Creates a bounded backward seek for reverse playback.
    fn backward(
        rate: f64,
        start: gst::ClockTime,
        stop: gst::ClockTime,
    ) -> Self {
        Self {
            rate,
            start,
            stop: Some(stop),
            accurate_forward: false,
        }
    }

    /// Rejects seek plans GStreamer cannot execute safely.
    fn validate(self) -> Result<Self, String> {
        if let Some(stop) = self.stop {
            if self.start >= stop {
                return Err(format!(
                    "invalid seek bounds: start={:?}, stop={:?}",
                    self.start, stop
                ));
            }
        } else if self.rate < 0.0 {
            return Err("backward seek missing stop time".to_string());
        }

        Ok(self)
    }
}

/// Multi-file video source bound to one graph video resource.
pub struct VideoSource {
    sources: Vec<VideoPipeline>,
    active_index: usize,
}

/// GStreamer pipeline and cached transport state for one video file.
struct VideoPipeline {
    pipeline: gst::Element,
    appsink: gst_app::AppSink,
    logged_frame_info: bool,
    duration: Option<gst::ClockTime>,
    last_loop_index: Option<i64>,
    last_ping_second_half: Option<bool>,
    last_transport: Option<VideoTransport>,
    last_transport_seek_at: Option<Instant>,
    current_rate: f64,
}

impl VideoSource {
    /// Opens all file paths and starts the first pipeline.
    pub fn new(paths: &[std::path::PathBuf]) -> Result<Self, String> {
        if paths.is_empty() {
            return Err("video source requires at least one path".to_string());
        }

        let mut sources = paths
            .iter()
            .map(|path| VideoPipeline::new(path))
            .collect::<Result<Vec<_>, _>>()?;
        for source in sources.iter_mut().skip(1) {
            source.pause()?;
        }

        Ok(Self {
            sources,
            active_index: 0,
        })
    }

    /// Selects a file index and applies beat-synced transport to it.
    pub fn apply_transport(
        &mut self,
        transport: &VideoTransport,
        beats: f32,
        bpm: f32,
    ) -> Result<(), String> {
        self.select(transport.index)?;
        self.active_mut().apply_transport(transport, beats, bpm)
    }

    /// Pulls the newest decoded RGBA frame from the active pipeline.
    pub fn next_frame(&mut self) -> Result<Option<VideoFrame>, String> {
        self.active_mut().next_frame()
    }

    /// Restarts the active pipeline from the beginning.
    pub fn restart(&mut self) -> Result<(), String> {
        self.active_mut().restart()
    }

    /// Restarts the selected pipeline and reapplies its transport at beat 0.
    pub fn restart_with_transport(
        &mut self,
        transport: &VideoTransport,
        bpm: f32,
    ) -> Result<(), String> {
        self.select(transport.index)?;
        self.active_mut().restart_with_transport(transport, bpm)
    }

    /// Switches the active file, pausing the previously active pipeline.
    fn select(&mut self, index: usize) -> Result<(), String> {
        let next_index = index.min(self.sources.len() - 1);
        if next_index == self.active_index {
            return Ok(());
        }

        self.sources[self.active_index].pause()?;
        self.active_index = next_index;
        self.sources[self.active_index].reset_transport_state();
        Ok(())
    }

    /// Returns the currently selected pipeline.
    fn active_mut(&mut self) -> &mut VideoPipeline {
        &mut self.sources[self.active_index]
    }
}

impl VideoPipeline {
    /// Builds a video-only playbin pipeline that outputs RGBA frames.
    fn new(path: &Path) -> Result<Self, String> {
        init_gstreamer()?;

        let resolved = normalize_video_path(path)?;
        let uri = glib::filename_to_uri(&resolved, None).map_err(|err| {
            format!(
                "failed to convert video path '{}' to URI: {}",
                resolved.display(),
                err
            )
        })?;

        let (video_sink, appsink) = create_video_sink()?;
        let fakesink = gst::ElementFactory::make("fakesink")
            .property("sync", false)
            .build()
            .map_err(|err| format!("failed to create fakesink: {}", err))?;

        let pipeline = gst::ElementFactory::make("playbin")
            .property("uri", uri.as_str())
            .property("video-sink", video_sink)
            .property("audio-sink", fakesink)
            .build()
            .map_err(|err| {
                format!(
                    "failed to create GStreamer playbin for '{}': {}",
                    resolved.display(),
                    err
                )
            })?;
        configure_video_only_playbin(&pipeline)?;

        pipeline.set_state(gst::State::Playing).map_err(|err| {
            format!(
                "failed to start video pipeline for '{}': {}",
                resolved.display(),
                err
            )
        })?;

        Ok(Self {
            pipeline,
            appsink,
            logged_frame_info: false,
            duration: None,
            last_loop_index: None,
            last_ping_second_half: None,
            last_transport: None,
            last_transport_seek_at: None,
            current_rate: 1.0,
        })
    }

    /// Applies beat-derived transport state to the pipeline.
    ///
    /// This method seeks only when the file index, loop, ping-pong half, or
    /// transport parameters changed enough to require it. Plain forward
    /// playback from the beginning is allowed to run without constant seeks.
    pub fn apply_transport(
        &mut self,
        transport: &VideoTransport,
        beats: f32,
        bpm: f32,
    ) -> Result<(), String> {
        let loop_beats = transport.beats.max(0.000_1);
        let loop_index = (beats / loop_beats).floor() as i64;
        let loop_beat = beats.rem_euclid(loop_beats);
        let ping_second_half = transport.direction == VideoDirection::PingPong
            && loop_beat >= loop_beats * 0.5;
        let transport_changed = self
            .last_transport
            .as_ref()
            .is_none_or(|last| last != transport);
        let start_changed = self
            .last_transport
            .as_ref()
            .is_some_and(|last| last.start != transport.start);
        let loop_changed = self.last_loop_index != Some(loop_index);
        let ping_changed = self.last_ping_second_half != Some(ping_second_half);

        if !transport_changed && !loop_changed && !ping_changed {
            return Ok(());
        }

        let rate = transport_rate(transport, ping_second_half);
        // Let GStreamer free-run for the common "play from start at 1x" case.
        if self.last_transport.is_none()
            && transport.start <= 0.0
            && transport.direction == VideoDirection::Forward
            && rate == 1.0
        {
            self.play()?;
            self.current_rate = rate;
            self.last_loop_index = Some(loop_index);
            self.last_ping_second_half = Some(ping_second_half);
            self.last_transport = Some(transport.clone());
            return Ok(());
        }

        let same_position_window = self
            .last_transport
            .as_ref()
            .is_some_and(|last| same_position_window(last, transport));

        let seek_plan = if ping_second_half {
            let Some(duration) = self.duration() else {
                return Ok(());
            };
            // The second ping-pong half plays backward between bounded points.
            match plan_ping_pong_reverse_seek(transport, bpm, duration) {
                Some(plan) => Some(plan),
                None => {
                    warn_once!(
                        "ping_pong segment wraps past EOF; skipping reverse \
                        seek"
                    );
                    self.current_rate = rate;
                    self.last_loop_index = Some(loop_index);
                    self.last_ping_second_half = Some(ping_second_half);
                    self.last_transport = Some(transport.clone());
                    return Ok(());
                }
            }
        } else if transport.direction == VideoDirection::PingPong {
            let Some(duration) = self.duration() else {
                return Ok(());
            };
            // The first ping-pong half uses a bounded forward segment so the
            // later reverse half starts from the intended turnaround point.
            let position = if loop_changed || !same_position_window {
                None
            } else {
                self.position()
            };
            match plan_ping_pong_forward_seek(
                transport, bpm, duration, position,
            ) {
                Some(plan) => Some(plan),
                None => {
                    warn_once!(
                        "ping_pong segment wraps past EOF; skipping forward \
                        seek"
                    );
                    self.current_rate = rate;
                    self.last_loop_index = Some(loop_index);
                    self.last_ping_second_half = Some(ping_second_half);
                    self.last_transport = Some(transport.clone());
                    return Ok(());
                }
            }
        } else {
            // Non-ping-pong transport can keep its current position while only
            // speed changes within the same media window.
            let position = if loop_changed || !same_position_window {
                self.normalized_time(transport.start)
            } else if let Some(position) = self.position() {
                Some(position)
            } else {
                self.normalized_time(transport.start)
            };
            position.map(|position| {
                PlannedSeek::forward(
                    rate,
                    position,
                    transport.direction == VideoDirection::PingPong,
                )
            })
        };
        let Some(seek_plan) = seek_plan else {
            return Ok(());
        };

        // Scrubbing the normalized start control can generate many tiny seek
        // changes in one gesture. Throttle only that case.
        if start_changed
            && !loop_changed
            && !ping_changed
            && self
                .last_transport_seek_at
                .is_some_and(|last| last.elapsed() < START_SEEK_THROTTLE)
        {
            return Ok(());
        }

        if rate == 0.0 {
            self.pipeline
                .set_state(gst::State::Paused)
                .map_err(|err| format!("failed to pause video: {}", err))?;
        } else if let Err(err) = self.execute_seek(seek_plan) {
            warn_once!(
                "video transport seek failed: {} ({:?})",
                err,
                seek_plan,
            );
            self.current_rate = rate;
            self.last_loop_index = Some(loop_index);
            self.last_ping_second_half = Some(ping_second_half);
            self.last_transport = Some(transport.clone());
            return Ok(());
        } else {
            self.last_transport_seek_at = Some(Instant::now());
        }

        self.current_rate = rate;
        self.last_loop_index = Some(loop_index);
        self.last_ping_second_half = Some(ping_second_half);
        self.last_transport = Some(transport.clone());

        Ok(())
    }

    /// Pulls one decoded sample and normalizes it to tightly packed RGBA rows.
    pub fn next_frame(&mut self) -> Result<Option<VideoFrame>, String> {
        self.handle_bus_messages();

        let Some(sample) = self.appsink.try_pull_sample(gst::ClockTime::ZERO)
        else {
            return Ok(None);
        };

        let caps = sample
            .caps()
            .ok_or_else(|| "video sample missing caps".to_string())?;
        let info = gst_video::VideoInfo::from_caps(caps).map_err(|err| {
            format!("failed to read video sample caps: {}", err)
        })?;

        let width = info.width().max(1);
        let height = info.height().max(1);
        let stride = info
            .stride()
            .first()
            .copied()
            .ok_or_else(|| "video sample missing row stride".to_string())?;
        if stride < 0 {
            return Err("video sample has negative row stride".to_string());
        }
        let stride = stride as usize;
        let row_bytes = width as usize * 4;

        if !self.logged_frame_info {
            debug!(
                "video frame info: {}x{}, stride={}, caps={}",
                width, height, stride, caps
            );
            self.logged_frame_info = true;
        }

        let buffer = sample
            .buffer()
            .ok_or_else(|| "video sample missing buffer".to_string())?;
        let map = buffer
            .map_readable()
            .map_err(|err| format!("failed to map video sample: {}", err))?;
        let data = map.as_slice();

        let mut rgba = vec![0; row_bytes * height as usize];
        for row in 0..height as usize {
            let src_start = row * stride;
            let src_end = src_start + row_bytes;
            let dst_start = row * row_bytes;
            let dst_end = dst_start + row_bytes;
            if src_end > data.len() {
                return Err(
                    "video sample buffer is smaller than caps".to_string()
                );
            }
            rgba[dst_start..dst_end].copy_from_slice(&data[src_start..src_end]);
        }

        Ok(Some(VideoFrame {
            width,
            height,
            rgba,
        }))
    }

    /// Seeks the active pipeline to the beginning and starts playback.
    pub fn restart(&mut self) -> Result<(), String> {
        self.last_loop_index = None;
        self.last_ping_second_half = None;
        self.last_transport = None;
        self.last_transport_seek_at = None;
        self.current_rate = 1.0;
        self.pipeline
            .seek_simple(
                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                gst::ClockTime::ZERO,
            )
            .map_err(|err| format!("failed to seek video to start: {}", err))?;
        self.pipeline
            .set_state(gst::State::Playing)
            .map_err(|err| {
                format!("failed to restart video playback: {}", err)
            })?;
        Ok(())
    }

    /// Clears cached transport state and applies transport from beat 0.
    pub fn restart_with_transport(
        &mut self,
        transport: &VideoTransport,
        bpm: f32,
    ) -> Result<(), String> {
        self.reset_transport_state();
        self.apply_transport(transport, 0.0, bpm)
    }

    /// Pauses this pipeline when another source index becomes active.
    fn pause(&mut self) -> Result<(), String> {
        self.pipeline
            .set_state(gst::State::Paused)
            .map_err(|err| format!("failed to pause video: {}", err))?;
        Ok(())
    }

    /// Starts or resumes this pipeline.
    fn play(&mut self) -> Result<(), String> {
        self.pipeline
            .set_state(gst::State::Playing)
            .map_err(|err| format!("failed to play video: {}", err))?;
        Ok(())
    }

    /// Clears transport cache so the next update will issue a fresh seek.
    fn reset_transport_state(&mut self) {
        self.last_loop_index = None;
        self.last_ping_second_half = None;
        self.last_transport = None;
        self.last_transport_seek_at = None;
        self.current_rate = 1.0;
    }

    /// Executes a validated seek and resumes playback.
    fn execute_seek(&self, seek: PlannedSeek) -> Result<(), String> {
        let seek = seek.validate()?;

        if seek.rate < 0.0 {
            let flags = gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE;
            let Some(stop) = seek.stop else {
                return Err("backward seek missing stop time".to_string());
            };
            self.pipeline
                .seek(
                    seek.rate,
                    flags,
                    gst::SeekType::Set,
                    seek.start,
                    gst::SeekType::Set,
                    stop,
                )
                .map_err(|err| {
                    format!("failed to seek video backward: {}", err)
                })?;
        } else {
            let flags = if seek.accurate_forward {
                gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE
            } else {
                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT
            };
            let (stop_type, stop) = seek
                .stop
                .map_or((gst::SeekType::None, gst::ClockTime::NONE), |stop| {
                    (gst::SeekType::Set, Some(stop))
                });
            self.pipeline
                .seek(
                    seek.rate,
                    flags,
                    gst::SeekType::Set,
                    seek.start,
                    stop_type,
                    stop,
                )
                .map_err(|err| {
                    format!("failed to seek video forward: {}", err)
                })?;
        }

        self.pipeline
            .set_state(gst::State::Playing)
            .map_err(|err| {
                format!("failed to set video pipeline playing: {}", err)
            })?;

        Ok(())
    }

    /// Converts normalized 0..1 media position to a clock time.
    fn normalized_time(&mut self, normalized: f32) -> Option<gst::ClockTime> {
        if normalized <= 0.0 {
            return Some(gst::ClockTime::ZERO);
        }

        let duration = self.duration()?;

        let nseconds = duration.nseconds() as f64;
        let position = (nseconds * normalized.clamp(0.0, 1.0) as f64) as u64;
        Some(gst::ClockTime::from_nseconds(position))
    }

    /// Returns cached media duration, querying GStreamer on first use.
    fn duration(&mut self) -> Option<gst::ClockTime> {
        if self.duration.is_none() {
            self.duration = self.pipeline.query_duration::<gst::ClockTime>();
        }
        self.duration
    }

    /// Returns the current media position when GStreamer can report it.
    fn position(&self) -> Option<gst::ClockTime> {
        self.pipeline.query_position::<gst::ClockTime>()
    }

    /// Drains end-of-stream and error messages without blocking frame render.
    fn handle_bus_messages(&self) {
        let Some(bus) = self.pipeline.bus() else {
            return;
        };

        while let Some(message) = bus.timed_pop_filtered(
            gst::ClockTime::ZERO,
            &[gst::MessageType::Eos, gst::MessageType::Error],
        ) {
            match message.view() {
                gst::MessageView::Eos(_) => {
                    let result = if self.current_rate < 0.0 {
                        let duration = self
                            .pipeline
                            .query_duration::<gst::ClockTime>()
                            .unwrap_or(gst::ClockTime::ZERO);
                        self.execute_seek(PlannedSeek::backward(
                            self.current_rate,
                            gst::ClockTime::ZERO,
                            duration,
                        ))
                    } else {
                        self.pipeline
                            .seek_simple(
                                gst::SeekFlags::FLUSH
                                    | gst::SeekFlags::KEY_UNIT,
                                gst::ClockTime::ZERO,
                            )
                            .map_err(|err| {
                                format!("failed to loop video: {}", err)
                            })
                    };
                    if let Err(err) = result {
                        warn!("{}", err);
                    }
                    if let Err(err) =
                        self.pipeline.set_state(gst::State::Playing)
                    {
                        warn!("failed to restart video pipeline: {}", err);
                    }
                }
                gst::MessageView::Error(err) => {
                    warn!(
                        "GStreamer video error from {:?}: {} ({:?})",
                        err.src().map(|src| src.path_string()),
                        err.error(),
                        err.debug()
                    );
                }
                _ => {}
            }
        }
    }
}

/// Creates a sink bin that converts decoded video into RGBA appsink samples.
fn create_video_sink() -> Result<(gst::Bin, gst_app::AppSink), String> {
    let bin = gst::Bin::with_name("xtal_video_sink");
    let convert_in = gst::ElementFactory::make("videoconvert")
        .name("xtal_video_convert_in")
        .build()
        .map_err(|err| format!("failed to create videoconvert: {}", err))?;
    let flip = gst::ElementFactory::make("videoflip")
        .name("xtal_video_flip")
        .property("video-direction", gst_video::VideoOrientationMethod::Auto)
        .build()
        .map_err(|err| format!("failed to create videoflip: {}", err))?;
    let convert_out = gst::ElementFactory::make("videoconvert")
        .name("xtal_video_convert_out")
        .build()
        .map_err(|err| format!("failed to create videoconvert: {}", err))?;
    let caps = gst::Caps::builder("video/x-raw")
        .field("format", "RGBA")
        .build();
    let capsfilter = gst::ElementFactory::make("capsfilter")
        .name("xtal_video_caps")
        .property("caps", caps)
        .build()
        .map_err(|err| format!("failed to create capsfilter: {}", err))?;
    let appsink = gst::ElementFactory::make("appsink")
        .name("xtal_sink")
        .property("sync", true)
        .property("max-buffers", 1u32)
        .property("drop", true)
        .build()
        .map_err(|err| format!("failed to create appsink: {}", err))?
        .downcast::<gst_app::AppSink>()
        .map_err(|_| {
            "GStreamer appsink factory returned wrong type".to_string()
        })?;

    bin.add_many([
        &convert_in,
        &flip,
        &convert_out,
        &capsfilter,
        appsink.upcast_ref(),
    ])
    .map_err(|err| format!("failed to add video sink elements: {}", err))?;
    gst::Element::link_many([
        &convert_in,
        &flip,
        &convert_out,
        &capsfilter,
        appsink.upcast_ref(),
    ])
    .map_err(|err| format!("failed to link video sink elements: {}", err))?;

    let sink_pad = convert_in
        .static_pad("sink")
        .ok_or_else(|| "video sink missing sink pad".to_string())?;
    let ghost_pad = gst::GhostPad::with_target(&sink_pad).map_err(|err| {
        format!("failed to create video sink ghost pad: {}", err)
    })?;
    bin.add_pad(&ghost_pad).map_err(|err| {
        format!("failed to add video sink ghost pad: {}", err)
    })?;

    Ok((bin, appsink))
}

/// Configures playbin to decode video only and discard audio.
fn configure_video_only_playbin(pipeline: &gst::Element) -> Result<(), String> {
    let flags = pipeline.property_value("flags");
    let flags_class =
        glib::FlagsClass::with_type(flags.type_()).ok_or_else(|| {
            "playbin flags property is not a flags type".to_string()
        })?;
    let video_only_flags = flags_class
        .builder()
        .set_by_nick("video")
        .build()
        .ok_or_else(video_only_flags_error)?;

    pipeline.set_property_from_value("flags", &video_only_flags);

    Ok(())
}

/// Returns an error message for failed video-only flag construction.
fn video_only_flags_error() -> String {
    "failed to build video-only playbin flags".to_string()
}

/// Converts Xtal transport direction and speed to a GStreamer playback rate.
fn transport_rate(transport: &VideoTransport, ping_second_half: bool) -> f64 {
    let speed = transport.speed.abs() as f64;
    if speed == 0.0 {
        return 0.0;
    }

    match transport.direction {
        VideoDirection::Forward => speed,
        VideoDirection::Backward => -speed,
        VideoDirection::PingPong if ping_second_half => -speed,
        VideoDirection::PingPong => speed,
    }
}

/// Plans the reverse half of a ping-pong loop.
fn plan_ping_pong_reverse_seek(
    transport: &VideoTransport,
    bpm: f32,
    duration: gst::ClockTime,
) -> Option<PlannedSeek> {
    let (start, turnaround) =
        ping_pong_segment_bounds(transport, bpm, duration)?;

    Some(PlannedSeek::backward(
        -transport.speed.abs() as f64,
        start,
        turnaround,
    ))
}

/// Plans the forward half of a ping-pong loop.
fn plan_ping_pong_forward_seek(
    transport: &VideoTransport,
    bpm: f32,
    duration: gst::ClockTime,
    position: Option<gst::ClockTime>,
) -> Option<PlannedSeek> {
    let (start, turnaround) =
        ping_pong_segment_bounds(transport, bpm, duration)?;
    let position = position
        .filter(|position| *position >= start && *position < turnaround)
        .unwrap_or(start);

    Some(PlannedSeek::forward_segment(
        transport.speed.abs() as f64,
        position,
        turnaround,
    ))
}

/// Returns bounded media positions for one ping-pong half-loop.
fn ping_pong_segment_bounds(
    transport: &VideoTransport,
    bpm: f32,
    duration: gst::ClockTime,
) -> Option<(gst::ClockTime, gst::ClockTime)> {
    let duration_ns = duration.nseconds();
    if duration_ns <= 1 {
        return None;
    }

    let start_ns = normalized_to_nseconds(transport.start, duration_ns)
        .min(duration_ns - 1);
    let half_beats = transport.beats.max(0.000_1) * 0.5;
    let seconds_per_beat = 60.0 / bpm.max(1.0);
    let media_seconds = half_beats * seconds_per_beat * transport.speed.abs();
    let media_ns = (media_seconds.max(0.0) * NANOSECONDS_PER_SECOND) as u64;
    let turnaround_ns = start_ns.checked_add(media_ns)?;

    if turnaround_ns >= duration_ns || turnaround_ns <= start_ns {
        return None;
    }

    Some((
        gst::ClockTime::from_nseconds(start_ns),
        gst::ClockTime::from_nseconds(turnaround_ns),
    ))
}

/// Converts a normalized media position into nanoseconds.
fn normalized_to_nseconds(normalized: f32, duration_ns: u64) -> u64 {
    (duration_ns as f32 * normalized.clamp(0.0, 1.0)) as u64
}

/// Returns whether two transports refer to the same media time window.
fn same_position_window(a: &VideoTransport, b: &VideoTransport) -> bool {
    a.source == b.source
        && a.start == b.start
        && a.beats == b.beats
        && a.direction == b.direction
}

impl Drop for VideoPipeline {
    /// Releases GStreamer resources by putting the pipeline into Null state.
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}

/// Initializes GStreamer once per process.
fn init_gstreamer() -> Result<(), String> {
    GST_INIT
        .get_or_init(|| {
            gst::init().map_err(|err| {
                format!("failed to initialize GStreamer: {}", err)
            })
        })
        .clone()
}

/// Resolves a video path and verifies that the file exists.
fn normalize_video_path(path: &Path) -> Result<std::path::PathBuf, String> {
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|err| format!("failed to get current directory: {}", err))?
            .join(path)
    };

    if !resolved.exists() {
        return Err(format!(
            "video file does not exist: {}",
            resolved.display()
        ));
    }

    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ping_pong_transport(start: f32) -> VideoTransport {
        VideoTransport {
            source: "a".to_string(),
            index: 0,
            start,
            beats: 4.0,
            speed: 1.0,
            direction: VideoDirection::PingPong,
        }
    }

    #[test]
    fn ping_pong_reverse_seek_uses_scrubbed_start_as_lower_bound() {
        let duration = gst::ClockTime::from_seconds(10);
        let plan = plan_ping_pong_reverse_seek(
            &ping_pong_transport(0.5),
            120.0,
            duration,
        )
        .expect("expected valid reverse seek");

        assert_eq!(plan.rate, -1.0);
        assert_eq!(plan.start, gst::ClockTime::from_seconds(5));
        assert_eq!(plan.stop, Some(gst::ClockTime::from_seconds(6)));
        assert!(plan.start < plan.stop.unwrap());
    }

    #[test]
    fn ping_pong_forward_seek_uses_bounded_segment() {
        let duration = gst::ClockTime::from_seconds(10);
        let plan = plan_ping_pong_forward_seek(
            &ping_pong_transport(0.5),
            120.0,
            duration,
            None,
        )
        .expect("expected valid forward seek");

        assert_eq!(plan.rate, 1.0);
        assert_eq!(plan.start, gst::ClockTime::from_seconds(5));
        assert_eq!(plan.stop, Some(gst::ClockTime::from_seconds(6)));
        assert!(plan.start < plan.stop.unwrap());
    }

    #[test]
    fn ping_pong_forward_seek_rejects_position_after_segment_stop() {
        let duration = gst::ClockTime::from_seconds(10);
        let plan = plan_ping_pong_forward_seek(
            &ping_pong_transport(0.5),
            120.0,
            duration,
            Some(gst::ClockTime::from_seconds(7)),
        )
        .expect("expected valid forward seek");

        assert_eq!(plan.start, gst::ClockTime::from_seconds(5));
        assert_eq!(plan.stop, Some(gst::ClockTime::from_seconds(6)));
    }

    #[test]
    fn ping_pong_reverse_seek_rejects_wrapped_segments() {
        let duration = gst::ClockTime::from_seconds(10);
        let plan = plan_ping_pong_reverse_seek(
            &ping_pong_transport(0.95),
            120.0,
            duration,
        );

        assert_eq!(plan, None);
    }

    #[test]
    fn ping_pong_reverse_seek_never_plans_invalid_bounds() {
        let duration = gst::ClockTime::from_seconds(10);

        for start in [0.0, 0.1, 0.25, 0.5, 0.75, 0.85, 0.95] {
            let plan = plan_ping_pong_reverse_seek(
                &ping_pong_transport(start),
                120.0,
                duration,
            );

            if let Some(plan) = plan {
                assert!(plan.rate < 0.0);
                assert!(plan.stop.is_some());
                assert!(plan.start < plan.stop.unwrap());
            }
        }
    }

    #[test]
    fn backward_seek_validation_rejects_invalid_bounds() {
        let seek = PlannedSeek::backward(
            -1.0,
            gst::ClockTime::from_seconds(3),
            gst::ClockTime::from_seconds(3),
        );

        assert!(seek.validate().is_err());
    }

    #[test]
    fn forward_segment_validation_rejects_invalid_bounds() {
        let seek = PlannedSeek::forward_segment(
            1.0,
            gst::ClockTime::from_seconds(3),
            gst::ClockTime::from_seconds(3),
        );

        assert!(seek.validate().is_err());
    }
}
