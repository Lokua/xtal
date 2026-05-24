use std::path::Path;
use std::sync::OnceLock;

use gstreamer as gst;
use gstreamer::glib;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;
use log::{debug, warn};

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
        })
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
        self.pipeline
            .seek_simple(
                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                gst::ClockTime::ZERO,
            )
            .map_err(|err| format!("failed to seek video to start: {}", err))?;
        self.pipeline
            .set_state(gst::State::Playing)
            .map_err(|err| format!("failed to restart video playback: {}", err))?;
        Ok(())
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
                    if let Err(err) = self.pipeline.seek_simple(
                        gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                        gst::ClockTime::ZERO,
                    ) {
                        warn!("failed to loop video pipeline: {}", err);
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
        .map_err(|_| "GStreamer appsink factory returned wrong type".to_string())?;

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
    let ghost_pad = gst::GhostPad::with_target(&sink_pad)
        .map_err(|err| format!("failed to create video sink ghost pad: {}", err))?;
    bin.add_pad(&ghost_pad)
        .map_err(|err| format!("failed to add video sink ghost pad: {}", err))?;

    Ok((bin, appsink))
}

impl Drop for VideoSource {
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}

fn init_gstreamer() -> Result<(), String> {
    GST_INIT
        .get_or_init(|| {
            gst::init()
                .map_err(|err| format!("failed to initialize GStreamer: {}", err))
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
