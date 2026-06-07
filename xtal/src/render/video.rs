use std::path::Path;
use std::sync::OnceLock;

use gstreamer as gst;
use gstreamer::glib;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;
use log::{debug, warn};

use crate::control::{VideoDirection, VideoTransport};
use crate::warn_once;

static GST_INIT: OnceLock<Result<(), String>> = OnceLock::new();

pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub struct VideoSource {
    pipeline: gst::Element,
    appsink: gst_app::AppSink,
    logged_frame_info: bool,
    duration: Option<gst::ClockTime>,
    last_loop_index: Option<i64>,
    last_ping_second_half: Option<bool>,
    last_transport: Option<VideoTransport>,
    current_rate: f64,
}

impl VideoSource {
    pub fn new(path: &Path) -> Result<Self, String> {
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
            current_rate: 1.0,
        })
    }

    pub fn apply_transport(
        &mut self,
        transport: &VideoTransport,
        beats: f32,
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
        let loop_changed = self.last_loop_index != Some(loop_index);
        let ping_changed = self.last_ping_second_half != Some(ping_second_half);

        if !transport_changed && !loop_changed && !ping_changed {
            return Ok(());
        }

        let rate = transport_rate(transport, ping_second_half);
        if self.last_transport.is_none()
            && transport.start <= 0.0
            && transport.direction == VideoDirection::Forward
            && rate == 1.0
        {
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

        let position = if loop_changed || !same_position_window {
            self.normalized_time(transport.start)
        } else if let Some(position) = self.position() {
            Some(position)
        } else {
            self.normalized_time(transport.start)
        };
        let Some(position) = position else {
            return Ok(());
        };

        if rate == 0.0 {
            self.pipeline
                .set_state(gst::State::Paused)
                .map_err(|err| format!("failed to pause video: {}", err))?;
        } else if let Err(err) = self.seek_with_rate(rate, position) {
            warn_once!("video transport seek failed: {}", err);
            return Ok(());
        }

        self.current_rate = rate;
        self.last_loop_index = Some(loop_index);
        self.last_ping_second_half = Some(ping_second_half);
        self.last_transport = Some(transport.clone());

        Ok(())
    }

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

    pub fn restart(&mut self) -> Result<(), String> {
        self.last_loop_index = None;
        self.last_ping_second_half = None;
        self.last_transport = None;
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

    pub fn restart_with_transport(
        &mut self,
        transport: &VideoTransport,
    ) -> Result<(), String> {
        self.last_loop_index = None;
        self.last_ping_second_half = None;
        self.last_transport = None;
        self.apply_transport(transport, 0.0)
    }

    fn seek_with_rate(
        &self,
        rate: f64,
        position: gst::ClockTime,
    ) -> Result<(), String> {
        let flags = gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT;

        if rate < 0.0 {
            self.pipeline
                .seek(
                    rate,
                    flags,
                    gst::SeekType::Set,
                    gst::ClockTime::ZERO,
                    gst::SeekType::Set,
                    position,
                )
                .map_err(|err| {
                    format!("failed to seek video backward: {}", err)
                })?;
        } else {
            self.pipeline
                .seek(
                    rate,
                    flags,
                    gst::SeekType::Set,
                    position,
                    gst::SeekType::None,
                    gst::ClockTime::NONE,
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

    fn normalized_time(&mut self, normalized: f32) -> Option<gst::ClockTime> {
        if normalized <= 0.0 {
            return Some(gst::ClockTime::ZERO);
        }

        let duration = self.duration();
        let Some(duration) = duration else {
            return None;
        };

        let nseconds = duration.nseconds() as f64;
        let position = (nseconds * normalized.clamp(0.0, 1.0) as f64) as u64;
        Some(gst::ClockTime::from_nseconds(position))
    }

    fn duration(&mut self) -> Option<gst::ClockTime> {
        if self.duration.is_none() {
            self.duration = self.pipeline.query_duration::<gst::ClockTime>();
        }
        self.duration
    }

    fn position(&self) -> Option<gst::ClockTime> {
        self.pipeline.query_position::<gst::ClockTime>()
    }

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
                        self.seek_with_rate(self.current_rate, duration)
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

fn same_position_window(a: &VideoTransport, b: &VideoTransport) -> bool {
    a.source == b.source
        && a.start == b.start
        && a.beats == b.beats
        && a.direction == b.direction
}

impl Drop for VideoSource {
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}

fn init_gstreamer() -> Result<(), String> {
    GST_INIT
        .get_or_init(|| {
            gst::init().map_err(|err| {
                format!("failed to initialize GStreamer: {}", err)
            })
        })
        .clone()
}

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
