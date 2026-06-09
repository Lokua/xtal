//! Main application runtime for xtal.
//!
//! This module owns the process-level application loop:
//!
//! - choose and instantiate the active sketch;
//! - create the winit window, wgpu surface, render context, and graph;
//! - build the `ControlHub` and timing source for the active sketch;
//! - route `RuntimeEvent` commands from the UI, keyboard, timing, and
//!   background callbacks;
//! - emit runtime-to-UI notifications through the web view bridge;
//! - save and restore global settings plus per-sketch control state.
//!
//! `RuntimeEvent` variant docs describe when each event is sent. The dispatcher
//! in this file describes the side effects of those events.

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::Utc;
use log::{debug, error, info, trace, warn};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowAttributes, WindowId};

use super::events::{
    RuntimeCommandReceiver, RuntimeCommandSender, RuntimeEvent,
    RuntimeEventSender, command_channel, event_channel,
};
use super::monitor_preview::{
    MonitorPreview, RenderResult as MonitorRenderResult, preview_size_for_main,
};
use super::projector::{self, ProjectorQuality};
use super::recording::{self, RecordingState};
use super::registry::RuntimeRegistry;
use super::serialization::{GlobalSettings, TransitorySketchState};
use super::storage;
use super::web_view;
use super::web_view_bridge::WebViewBridge;
use crate::context::Context;
use crate::control::map_mode::MapMode;
use crate::control::{ControlCollection, ControlHub, ControlValue};
use crate::core::logging;
use crate::core::util::{HashMap, uuid_5};
use crate::frame::Frame;
use crate::gpu::compute_row_padding;
use crate::gpu::{CompiledGraph, ExecuteCtx};
use crate::graph::GraphBuilder;
use crate::io::audio::list_audio_devices;
use crate::io::midi;
use crate::io::osc::SHARED_OSC_RECEIVER;
use crate::motion::{Bpm, MidiTransportEvent, Timing};
use crate::sketch::{PlayMode, Sketch, SketchConfig, TimingMode};
use crate::time::frame_clock;
use crate::time::tap_tempo::TapTempo;
use crate::uniforms::UniformBanks;

const DEFAULT_OSC_PORT: u16 = 2346;
const CONTINUE_HANDLING: bool = false;
const QUIT_REQUESTED: bool = true;

/// UI-only state that should survive sketch switches in this runtime session.
///
/// The control hub stores live control values and MIDI/OSC/audio mappings. This
/// cache stores the web-view shape of per-sketch mapping and randomize
/// exclusion state so switching away from a sketch and back does not lose local
/// UI edits before they are saved.
#[derive(Clone, Default)]
struct SketchUiState {
    mappings: web_view::Mappings,
    exclusions: web_view::Exclusions,
}

/// GPU readback state for a PNG capture requested from the UI or shortcut.
///
/// The render pass fills the buffer, then `queue_png_capture_save` maps it on a
/// worker thread so the winit event loop does not block on PNG encoding.
struct PendingPngCapture {
    path: PathBuf,
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
    source_format: wgpu::TextureFormat,
}

/// Long-lived application state owned by the winit application handler.
///
/// `XtalRuntime` is intentionally the place where process-wide concerns meet:
/// windowing, GPU resources, active sketch state, the web view bridge, global
/// settings, and background event callbacks. Sketch-specific behavior should
/// still live in sketches, controls, timing, and graph code.
struct XtalRuntime {
    // Active sketch identity and factory source.
    registry: RuntimeRegistry,
    active_sketch_name: String,
    timing_mode_override: Option<TimingMode>,
    config: &'static SketchConfig,
    sketch: Box<dyn Sketch>,
    render_requested: bool,

    // Runtime command ingress used for cross-component async handoff.
    // Best practice:
    // - Use direct helper calls for immediate local state changes.
    // - Use command enqueue when callbacks/watchers/background paths need to
    //   hand work back to the main runtime dispatcher.
    command_tx: RuntimeCommandSender,
    command_rx: RuntimeCommandReceiver,
    event_tx: Option<RuntimeEventSender>,

    // Window, surface, device, and render context resources.
    instance: Option<wgpu::Instance>,
    adapter: Option<wgpu::Adapter>,
    window: Option<Arc<Window>>,
    window_id: Option<WindowId>,
    monitor_preview: Option<MonitorPreview>,
    monitor_preview_size_hint: Option<winit::dpi::PhysicalSize<u32>>,
    windowed_size_before_fullscreen: Option<winit::dpi::PhysicalSize<u32>>,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    context: Option<Context>,
    uniforms: Option<UniformBanks>,
    graph: Option<CompiledGraph>,

    // Runtime control, timing, and performance state.
    control_hub: Option<ControlHub<Timing>>,
    bpm: Bpm,
    tap_tempo: TapTempo,
    tap_tempo_enabled: bool,
    perf_mode: bool,
    projector_mode_enabled: bool,
    projector_quality: ProjectorQuality,
    transition_time: f32,
    mappings_enabled: bool,
    map_mode: MapMode,
    sketch_ui_state: HashMap<String, SketchUiState>,
    recording_state: RecordingState,
    session_id: String,

    // External IO selections and live output handles.
    audio_device: String,
    audio_devices: Vec<String>,
    hrcc: bool,
    midi_out: Option<midi::MidiOut>,
    midi_clock_port: String,
    midi_input_port: String,
    midi_output_port: String,
    midi_input_ports: Vec<(usize, String)>,
    midi_output_ports: Vec<(usize, String)>,
    osc_port: u16,

    // Storage locations and cross-frame bookkeeping.
    images_dir: String,
    user_data_dir: String,
    videos_dir: String,
    image_index: Option<storage::ImageIndex>,
    last_average_fps_emit: Instant,
    shutdown_signaled: bool,
    pending_png_capture_path: Option<PathBuf>,
    modifiers: ModifiersState,
}

impl XtalRuntime {
    /// Builds runtime state before window and GPU initialization.
    ///
    /// This loads global settings from the sketch storage root, normalizes
    /// persisted device/port selections against currently available devices,
    /// starts shared OSC and MIDI output listeners, and leaves GPU/window
    /// resources empty until the winit `resumed` hook runs.
    fn new(
        registry: RuntimeRegistry,
        initial_sketch: Option<&str>,
        timing_mode_override: Option<TimingMode>,
        command_tx: RuntimeCommandSender,
        command_rx: RuntimeCommandReceiver,
        event_tx: Option<RuntimeEventSender>,
    ) -> Result<Self, String> {
        let active_name =
            select_initial_sketch_name(&registry, initial_sketch)?;

        let (config, sketch) = instantiate_sketch(&registry, &active_name)
            .map_err(|err| {
                format!(
                    "failed to initialize sketch '{}': {}",
                    active_name, err
                )
            })?;

        let bpm = Bpm::new(config.bpm);

        let sketch_storage_dir = default_user_data_dir_for_sketch(
            sketch.as_ref(),
        )
        .unwrap_or_else(|| {
            env::current_dir()
                .unwrap_or_default()
                .join("storage")
                .display()
                .to_string()
        });

        let mut global_settings = GlobalSettings {
            user_data_dir: sketch_storage_dir.clone(),
            ..GlobalSettings::default()
        };
        if let Ok(Some(saved)) =
            storage::load_global_state_if_exists(&sketch_storage_dir)
        {
            global_settings = saved;
        }
        if global_settings.osc_port == 0 {
            global_settings.osc_port = DEFAULT_OSC_PORT;
        }

        let image_index =
            storage::load_image_index(&global_settings.user_data_dir)
                .inspect_err(|e| error!("Error in runtime init: {}", e))
                .ok();

        let mut sketch_ui_state = HashMap::default();
        sketch_ui_state.insert(active_name.clone(), SketchUiState::default());

        let mut runtime = Self {
            registry,
            active_sketch_name: active_name,
            timing_mode_override,
            config,
            sketch,
            render_requested: false,
            command_tx,
            command_rx,
            event_tx,
            instance: None,
            adapter: None,
            window: None,
            window_id: None,
            monitor_preview: None,
            monitor_preview_size_hint: None,
            windowed_size_before_fullscreen: None,
            surface: None,
            surface_config: None,
            context: None,
            uniforms: None,
            graph: None,
            control_hub: None,
            bpm: bpm.clone(),
            tap_tempo: TapTempo::new(config.bpm),
            tap_tempo_enabled: false,
            perf_mode: false,
            projector_mode_enabled: global_settings.projector_mode_enabled,
            projector_quality: global_settings.projector_quality,
            transition_time: global_settings.transition_time,
            mappings_enabled: global_settings.mappings_enabled,
            map_mode: MapMode::default(),
            sketch_ui_state,
            recording_state: RecordingState::default(),
            session_id: recording::generate_session_id(),
            audio_device: global_settings.audio_device_name,
            audio_devices: list_audio_devices().unwrap_or_default(),
            hrcc: global_settings.hrcc,
            midi_out: None,
            midi_clock_port: global_settings.midi_clock_port,
            midi_input_port: global_settings.midi_control_in_port,
            midi_output_port: global_settings.midi_control_out_port,
            midi_input_ports: midi::list_input_ports().unwrap_or_default(),
            midi_output_ports: midi::list_output_ports().unwrap_or_default(),
            osc_port: global_settings.osc_port,
            images_dir: global_settings.images_dir,
            user_data_dir: global_settings.user_data_dir,
            videos_dir: global_settings.videos_dir,
            image_index,
            last_average_fps_emit: Instant::now(),
            shutdown_signaled: false,
            pending_png_capture_path: None,
            modifiers: ModifiersState::default(),
        };

        let audio_device_updated = runtime.normalize_audio_device_selection();
        let midi_ports_updated = runtime.normalize_midi_port_selections();
        let osc_port_updated = runtime.normalize_osc_port_selection();
        runtime.restart_osc_receiver();
        runtime.connect_midi_out();
        runtime.log_midi_startup_state();
        if audio_device_updated || midi_ports_updated || osc_port_updated {
            runtime.save_global_state();
        }

        Ok(runtime)
    }

    /// Applies one runtime command on the main application thread.
    ///
    /// `RuntimeEvent` documents when each event is sent. This dispatcher owns
    /// the actual side effects: mutating runtime state, restarting listeners,
    /// updating the control hub, requesting redraws, saving state, and
    /// notifying the web view.
    ///
    /// Returns `QUIT_REQUESTED` when handling should terminate early.
    fn on_runtime_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: RuntimeEvent,
    ) -> bool {
        match event {
            RuntimeEvent::AdvanceSingleFrame => {
                frame_clock::advance_single_frame();
            }
            RuntimeEvent::CaptureFrame => {
                if let Err(err) = fs::create_dir_all(&self.images_dir) {
                    self.alert_and_log(
                        format!(
                            "Failed to create images directory '{}': {}",
                            self.images_dir, err
                        ),
                        log::Level::Error,
                    );
                    return false;
                }

                let filename =
                    format!("{}-{}.png", self.active_sketch_name, uuid_5());
                let file_path = PathBuf::from(&self.images_dir).join(&filename);
                self.pending_png_capture_path = Some(file_path);
                self.render_requested = true;

                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }

                if let Some(image_index) = &mut self.image_index {
                    image_index.items.push(storage::ImageIndexItem {
                        filename,
                        created_at: Utc::now().to_rfc3339().to_string(),
                    });
                    if let Err(e) = storage::save_image_index(
                        &self.user_data_dir,
                        image_index,
                    ) {
                        error!("{}", e);
                    }
                }
            }
            RuntimeEvent::ChangeAudioDevice(name) => {
                self.audio_device = name.clone();
                if !self.audio_devices.contains(&name) {
                    self.audio_devices.push(name);
                }
                if let Some(hub) = self.control_hub.as_mut() {
                    hub.audio_controls
                        .set_device_name(self.audio_device.clone());
                    hub.audio_controls
                        .restart()
                        .inspect_err(|err| {
                            error!("Error in ChangeAudioDevice: {}", err)
                        })
                        .ok();
                }
                self.save_global_state();
            }
            RuntimeEvent::ChangeMidiClockPort(port) => {
                info!("Changing MIDI clock port to '{}'", port);
                self.midi_clock_port = port;
                let timing = self.build_timing();
                if let Some(hub) = self.control_hub.as_mut() {
                    hub.animation.timing = timing;
                }
                self.save_global_state();
            }
            RuntimeEvent::ChangeMidiControlInputPort(port) => {
                info!("Changing MIDI control input port to '{}'", port);
                self.midi_input_port = port.clone();
                if !self
                    .midi_input_ports
                    .iter()
                    .any(|(_, existing)| existing == &port)
                {
                    let idx = self.midi_input_ports.len();
                    self.midi_input_ports.push((idx, port));
                }
                if let Some(hub) = self.control_hub.as_mut() {
                    hub.midi_controls.set_port(self.midi_input_port.clone());
                    hub.midi_controls
                        .restart()
                        .inspect_err(|err| {
                            error!(
                                "Error in ChangeMidiControlInputPort: {}",
                                err
                            );
                        })
                        .ok();
                }
                self.save_global_state();
            }
            RuntimeEvent::ChangeMidiControlOutputPort(port) => {
                info!("Changing MIDI control output port to '{}'", port);
                self.midi_output_port = port.clone();
                if !self
                    .midi_output_ports
                    .iter()
                    .any(|(_, existing)| existing == &port)
                {
                    let idx = self.midi_output_ports.len();
                    self.midi_output_ports.push((idx, port));
                }
                self.connect_midi_out();
                self.save_global_state();
            }
            RuntimeEvent::ChangeOscPort(port) => {
                info!("Changing OSC port to {}", port);
                self.osc_port = port;
                self.restart_osc_receiver();
                self.save_global_state();
            }
            RuntimeEvent::ClearBuffer => {
                self.alert(
                    "ClearBuffer is not yet implemented in xtal runtime.",
                );
            }
            RuntimeEvent::CommitMappings => {
                // Committing from Settings -> Controls should also end live
                // learn so subsequent MIDI movement does not keep remapping.
                self.map_mode.stop();
                let mappings = self.map_mode.mappings();
                let mut missing_slider_ranges = Vec::new();

                {
                    let Some(hub) = self.control_hub.as_mut() else {
                        return false;
                    };

                    hub.midi_override_configs.retain(|name, _| {
                        mappings.contains_key(name) && hub.ui_controls.has(name)
                    });
                    hub.midi_overrides.lock().unwrap().retain(|name, _| {
                        hub.midi_override_configs.contains_key(name)
                    });

                    for (name, (ch, cc)) in mappings {
                        let slider_range =
                            match hub.ui_controls.slider_range(&name) {
                                Some(range) => range,
                                None => {
                                    missing_slider_ranges.push(name.clone());
                                    continue;
                                }
                            };

                        let existing_value = hub
                            .midi_overrides
                            .lock()
                            .unwrap()
                            .get(&name)
                            .copied()
                            .unwrap_or(0.0);

                        hub.midi_override_configs.insert(
                            name,
                            crate::control::MidiControlConfig::new(
                                (ch as u8, cc as u8),
                                slider_range,
                                existing_value,
                            ),
                        );
                    }

                    hub.midi_controls.set_override_configs(
                        hub.midi_override_configs.clone(),
                    );

                    if let Err(err) = hub.midi_controls.restart() {
                        error!("{}", err);
                    }
                }

                self.current_sketch_ui_state_mut().mappings =
                    self.map_mode.mappings();
                for name in missing_slider_ranges {
                    self.alert_and_log(
                        format!("No slider range for {}", name),
                        log::Level::Error,
                    );
                }
            }
            RuntimeEvent::CurrentlyMapping(name) => {
                if name.is_empty() {
                    self.map_mode.stop();
                    return false;
                }

                self.map_mode.remove(&name);
                if let Some(hub) = self.control_hub.as_mut() {
                    hub.midi_override_configs.remove(&name);
                    hub.midi_overrides.lock().unwrap().remove(&name);
                    hub.midi_controls.set_override_configs(
                        hub.midi_override_configs.clone(),
                    );
                }

                self.map_mode.currently_mapping = Some(name.clone());

                let command_tx = self.command_tx.clone();
                self.map_mode
                    .start(
                        &name,
                        &self.midi_input_port,
                        self.hrcc,
                        move |result| {
                            if let Err(err) = result {
                                let _ = command_tx.send(
                                    RuntimeEvent::MapModeError(format!(
                                        "Error: {}",
                                        err
                                    )),
                                );
                            }
                            let _ = command_tx.send(RuntimeEvent::SendMappings);
                        },
                    )
                    .inspect_err(|err| {
                        error!("Error in CurrentlyMapping: {}", err)
                    })
                    .ok();
            }
            RuntimeEvent::MapModeError(message) => {
                self.alert_and_log(message, log::Level::Error);
            }
            RuntimeEvent::HubPopulated => {
                let Some(hub) = self.control_hub.as_ref() else {
                    return false;
                };

                let controls = web_view::controls_from_hub(hub);
                let bypassed = hub.bypassed();
                let snapshot_sequence_enabled = hub.snapshot_sequence_enabled();

                self.emit_web_view_event(web_view::Event::HubPopulated((
                    controls, bypassed,
                )));
                self.emit_web_view_event(
                    web_view::Event::SnapshotSequenceEnabled(
                        snapshot_sequence_enabled,
                    ),
                );
                self.alert("Hub repopulated");
            }
            RuntimeEvent::MidiContinue | RuntimeEvent::MidiStart => {
                info!("Received MIDI Start/Continue. Resetting transport.");
                self.reset_transport();

                if self.recording_state.is_queued {
                    let _ = self.on_runtime_event(
                        event_loop,
                        RuntimeEvent::StartRecording,
                    );
                }
            }
            RuntimeEvent::MidiStop => {
                let _ = self
                    .on_runtime_event(event_loop, RuntimeEvent::StopRecording);
            }
            RuntimeEvent::OpenOsDir(kind) => {
                let path = self.os_dir_path(&kind);
                if let Err(err) = fs::create_dir_all(&path) {
                    self.alert_and_log(
                        format!(
                            "Failed to create {:?} directory '{}': {}",
                            kind,
                            path.display(),
                            err
                        ),
                        log::Level::Error,
                    );
                    return false;
                }

                let result = if cfg!(target_os = "macos") {
                    Command::new("open").arg(&path).spawn().map(|_| ())
                } else if cfg!(target_os = "windows") {
                    Command::new("explorer").arg(&path).spawn().map(|_| ())
                } else {
                    Command::new("xdg-open").arg(&path).spawn().map(|_| ())
                };

                if let Err(err) = result {
                    self.alert_and_log(
                        format!(
                            "Failed to open {:?} directory '{}': {}",
                            kind,
                            path.display(),
                            err
                        ),
                        log::Level::Error,
                    );
                }
            }
            RuntimeEvent::Pause(paused) => {
                frame_clock::set_paused(paused);
            }
            RuntimeEvent::QueueRecord => {
                self.recording_state.is_queued =
                    !self.recording_state.is_queued;
                if self.recording_state.is_queued {
                    self.alert_and_log(
                        "Recording queued. Awaiting MIDI start message.",
                        log::Level::Info,
                    );
                }
            }
            RuntimeEvent::Quit => {
                self.shutdown(event_loop);
                return QUIT_REQUESTED;
            }
            RuntimeEvent::Randomize(exclusions) => {
                self.alert_and_log("Transition started", log::Level::Info);

                if let Some(hub) = self.control_hub.as_mut() {
                    hub.randomize(exclusions);
                }
            }
            RuntimeEvent::ReceiveDir(kind, dir) => {
                if dir.is_empty() {
                    warn!("received empty directory update for {:?}", kind);
                    return false;
                }

                match kind {
                    web_view::UserDir::Images => self.images_dir = dir.clone(),
                    web_view::UserDir::UserData => {
                        self.user_data_dir = dir.clone();
                        if let Some(image_index) = &self.image_index
                            && !storage::image_metadata_exists(
                                &self.user_data_dir,
                            )
                            && !image_index.items.is_empty()
                        {
                            storage::save_image_index(
                                &self.user_data_dir,
                                image_index,
                            )
                            .inspect_err(|e| {
                                error!("Error saving image index: {}", e)
                            })
                            .ok();
                        }
                    }
                    web_view::UserDir::Videos => self.videos_dir = dir.clone(),
                }
                self.save_global_state();
            }
            RuntimeEvent::ReceiveMappings(mappings) => {
                self.map_mode.set_mappings(mappings.clone());
                self.current_sketch_ui_state_mut().mappings = mappings.clone();
            }
            RuntimeEvent::ReloadControls => {
                if let Some(hub) = self.control_hub.as_ref() {
                    hub.request_reload();
                }
            }
            RuntimeEvent::RemoveMapping(name) => {
                self.map_mode.remove(&name);
                self.map_mode.currently_mapping = None;

                if let Some(hub) = self.control_hub.as_mut() {
                    hub.midi_override_configs.remove(&name);
                    hub.midi_overrides.lock().unwrap().remove(&name);
                    hub.midi_controls.set_override_configs(
                        hub.midi_override_configs.clone(),
                    );
                }

                let mappings = self.map_mode.mappings();
                self.current_sketch_ui_state_mut().mappings = mappings.clone();
                self.emit_web_view_event(web_view::Event::Mappings(mappings));
            }
            RuntimeEvent::Reset => {
                self.reset_transport();
                self.alert("Reset");
            }
            RuntimeEvent::Save(exclusions) => {
                let stored = self.current_sketch_ui_state().exclusions;
                let next = if !exclusions.is_empty() || stored.is_empty() {
                    exclusions
                } else {
                    stored
                };
                self.set_exclusions(next);
                let exclusions_to_save =
                    self.current_sketch_ui_state().exclusions;
                let mappings_to_save = self.map_mode.mappings();
                self.current_sketch_ui_state_mut().mappings =
                    mappings_to_save.clone();
                let Some(hub) = self.control_hub.as_ref() else {
                    self.alert_and_log(
                        "Unable to save controls (no hub)",
                        log::Level::Error,
                    );
                    return false;
                };

                match storage::save_sketch_state(
                    &self.user_data_dir,
                    &self.active_sketch_name,
                    hub,
                    mappings_to_save,
                    exclusions_to_save,
                ) {
                    Ok(path) => {
                        self.alert_and_log(
                            format!("Controls saved to {:?}", path),
                            log::Level::Info,
                        );
                    }
                    Err(err) => {
                        self.alert_and_log(
                            format!("Failed to save controls: {}", err),
                            log::Level::Error,
                        );
                    }
                }
            }
            RuntimeEvent::SendMappings => {
                let mappings = self.map_mode.mappings();
                self.current_sketch_ui_state_mut().mappings = mappings.clone();
                self.emit_web_view_event(web_view::Event::Mappings(mappings));
            }
            RuntimeEvent::SendMidi => {
                let messages = self
                    .control_hub
                    .as_ref()
                    .map(|hub| {
                        if self.hrcc {
                            hub.midi_controls.messages_hrcc()
                        } else {
                            hub.midi_controls.messages()
                        }
                    })
                    .unwrap_or_default();

                if messages.is_empty() {
                    return false;
                }

                let Some(midi_out) = &mut self.midi_out else {
                    self.alert_and_log(
                        "Unable to send MIDI; no MIDI out connection",
                        log::Level::Warn,
                    );
                    return false;
                };

                let mut any_sent = false;
                for message in messages {
                    if let Err(err) = midi_out.send(&message) {
                        self.alert_and_log(
                            format!(
                                "Error sending MIDI message: {:?}; error: {}",
                                message, err
                            ),
                            log::Level::Error,
                        );
                        return false;
                    }
                    any_sent = true;
                }

                if any_sent {
                    self.alert_and_log("MIDI Sent", log::Level::Debug);
                }
            }
            RuntimeEvent::SetHrcc(enabled) => {
                self.hrcc = enabled;
                info!("Setting HRCC mode to {}", self.hrcc);
                if let Some(hub) = self.control_hub.as_mut() {
                    hub.midi_controls.hrcc = self.hrcc;
                    hub.midi_controls
                        .restart()
                        .inspect_err(|err| error!("Error in Hrcc: {}", err))
                        .ok();
                }
                self.save_global_state();
                self.alert_and_log(
                    if self.hrcc {
                        "Expecting 14bit MIDI CCs for channels 0-31"
                    } else {
                        "Expecting standard 7bit MIDI messages for all CCs"
                    },
                    log::Level::Info,
                );
            }
            RuntimeEvent::SetMappingsEnabled(enabled) => {
                info!("Setting mappings_enabled to {}", enabled);
                self.mappings_enabled = enabled;
                if let Some(hub) = self.control_hub.as_mut() {
                    hub.midi_overrides_enabled = enabled;
                }
                self.save_global_state();
            }
            RuntimeEvent::SetMonitorPreview(enabled) => {
                self.set_monitor_preview_enabled(event_loop, enabled);
            }
            RuntimeEvent::SetBpm(bpm) => {
                if self.tap_tempo_enabled && bpm.is_finite() {
                    self.bpm.set(bpm);
                    self.tap_tempo = TapTempo::new(self.bpm.get());
                    self.emit_web_view_event(web_view::Event::Bpm(
                        self.bpm.get(),
                    ));
                }
            }
            RuntimeEvent::SetPerfMode(perf_mode) => {
                self.set_perf_mode(perf_mode);
            }
            RuntimeEvent::SetProjectorMode(enabled) => {
                self.set_projector_mode(enabled);
            }
            RuntimeEvent::SetProjectorQuality(quality) => {
                self.set_projector_quality(quality);
            }
            RuntimeEvent::SetTransitionTime(transition_time) => {
                self.transition_time = transition_time;
                if let Some(hub) = self.control_hub.as_mut() {
                    hub.set_transition_time(self.transition_time);
                }
                self.save_global_state();
            }
            RuntimeEvent::SnapshotDelete(id) => {
                if let Some(hub) = self.control_hub.as_mut() {
                    hub.delete_snapshot(&id);
                    self.alert_and_log(
                        format!("Snapshot {:?} deleted", id),
                        log::Level::Info,
                    );
                }
            }
            RuntimeEvent::SnapshotEnded => {
                if let Some(hub) = self.control_hub.as_ref() {
                    self.emit_web_view_event(web_view::Event::SnapshotEnded(
                        web_view::controls_from_hub(hub),
                    ));
                }
                self.alert_and_log(
                    "Snapshot/Transition ended",
                    log::Level::Debug,
                );
                let _ = self.command_tx.send(RuntimeEvent::SendMidi);
            }
            RuntimeEvent::SnapshotRecall(id) => {
                if let Some(hub) = self.control_hub.as_mut() {
                    if let Err(err) = hub.recall_snapshot(&id) {
                        self.alert_and_log(err, log::Level::Error);
                    } else {
                        self.alert_and_log(
                            format!("Snapshot {:?} recalled", id),
                            log::Level::Info,
                        );
                    }
                }
            }
            RuntimeEvent::SnapshotStore(id) => {
                if let Some(hub) = self.control_hub.as_mut() {
                    hub.take_snapshot(&id);
                    self.alert_and_log(
                        format!("Snapshot {:?} saved", id),
                        log::Level::Info,
                    );
                } else {
                    self.alert_and_log(
                        "Unable to store snapshot (no hub)",
                        log::Level::Error,
                    );
                }
            }
            RuntimeEvent::StartRecording => {
                let Some(context) = self.context.as_ref() else {
                    self.alert_and_log(
                        "Failed to start recording: \
                            runtime context unavailable",
                        log::Level::Error,
                    );
                    return false;
                };
                let Some(graph) = self.graph.as_ref() else {
                    self.alert_and_log(
                        "Failed to start recording: render graph unavailable",
                        log::Level::Error,
                    );
                    return false;
                };
                let source_format =
                    graph.recording_source_format().or_else(|| {
                        self.surface_config.as_ref().map(|config| config.format)
                    });
                let Some(source_format) = source_format else {
                    self.alert_and_log(
                        "Failed to start recording: \
                            no capture source format available",
                        log::Level::Error,
                    );
                    return false;
                };

                if let Err(err) = fs::create_dir_all(&self.videos_dir) {
                    self.alert_and_log(
                        format!(
                            "Failed to create videos directory '{}': {}",
                            self.videos_dir, err
                        ),
                        log::Level::Error,
                    );
                    return false;
                }

                let [width, height] = context.resolution_u32();
                let output_path = recording::video_output_path(
                    &self.videos_dir,
                    &self.session_id,
                    self.config.name,
                )
                .to_string_lossy()
                .into_owned();

                match self.recording_state.start_recording(
                    context.device.clone(),
                    &output_path,
                    width,
                    height,
                    self.config.fps,
                    source_format,
                ) {
                    Ok(message) => {
                        self.recording_state.is_queued = false;
                        self.alert(message);
                        self.emit_web_view_event(
                            web_view::Event::StartRecording,
                        );
                    }
                    Err(err) => {
                        self.alert_and_log(
                            format!("Failed to start recording: {}", err),
                            log::Level::Error,
                        );
                    }
                }
            }
            RuntimeEvent::StopRecording => {
                if self.recording_state.is_recording
                    && !self.recording_state.is_encoding
                {
                    match self.recording_state.stop_recording() {
                        Ok(()) => {
                            self.emit_web_view_event(
                                web_view::Event::StopRecording,
                            );
                            self.emit_web_view_event(
                                web_view::Event::Encoding(true),
                            );
                        }
                        Err(err) => {
                            self.alert_and_log(
                                format!("Failed to stop recording: {}", err),
                                log::Level::Error,
                            );
                        }
                    }
                }
            }
            RuntimeEvent::SwitchSketch(name) => {
                if let Err(err) = self.switch_sketch(&name) {
                    error!("failed to switch sketch '{}': {}", name, err);
                }
            }
            RuntimeEvent::Tap => {
                if self.tap_tempo_enabled {
                    let bpm = self.tap_tempo.tap();
                    self.bpm.set(bpm);
                    self.emit_web_view_event(web_view::Event::Bpm(bpm));
                }
            }
            RuntimeEvent::TapTempoEnabled(enabled) => {
                self.tap_tempo_enabled = enabled;
                self.bpm.set(self.config.bpm);
                self.emit_web_view_event(web_view::Event::Bpm(self.bpm.get()));
                self.alert_and_log(
                    if enabled {
                        "Tap `Space` key to set BPM"
                    } else {
                        "Sketch BPM has been restored"
                    },
                    log::Level::Info,
                );
            }
            RuntimeEvent::ToggleFullScreen => {
                let Some(window) = self.window.as_ref() else {
                    return false;
                };

                if window.fullscreen().is_some() {
                    window.set_fullscreen(None);
                    if let Some(size) = self.windowed_size_before_fullscreen {
                        let _ = window.request_inner_size(size);
                    }
                } else {
                    self.windowed_size_before_fullscreen =
                        Some(window.inner_size());
                    let monitor = window.current_monitor();
                    window
                        .set_fullscreen(Some(Fullscreen::Borderless(monitor)));
                }
            }
            RuntimeEvent::ToggleMainFocus => {
                let Some(window) = self.window.as_ref() else {
                    return false;
                };

                if window.fullscreen().is_some() {
                    window.set_fullscreen(None);
                    if let Some(size) = self.windowed_size_before_fullscreen {
                        let _ = window.request_inner_size(size);
                    }
                }
                window.set_visible(true);
                window.focus_window();
            }
            RuntimeEvent::UpdateExclusions(exclusions) => {
                self.set_exclusions(exclusions);
            }
            RuntimeEvent::UpdateUiControl((name, value)) => {
                let should_emit_updated_controls = matches!(
                    value,
                    ControlValue::Bool(_) | ControlValue::String(_)
                );

                self.apply_control_update(name, value);

                if should_emit_updated_controls
                    && let Some(hub) = self.control_hub.as_ref()
                {
                    self.emit_web_view_event(web_view::Event::UpdatedControls(
                        web_view::controls_from_hub(hub),
                    ));
                }
            }
            RuntimeEvent::FrameSkipped
            | RuntimeEvent::SketchSwitched(_)
            | RuntimeEvent::Stopped
            | RuntimeEvent::WebView(_) => {}
        }

        CONTINUE_HANDLING
    }

    /// Drains pending commands from the async command channel.
    ///
    /// The web view bridge, timing callbacks, control hub callbacks, and
    /// background tasks cannot safely mutate runtime state directly. They send
    /// `RuntimeEvent` values here so all side effects happen on the main winit
    /// thread through `on_runtime_event`.
    fn process_commands(&mut self, event_loop: &ActiveEventLoop) {
        while let Ok(event) = self.command_rx.try_recv() {
            if self.on_runtime_event(event_loop, event) == QUIT_REQUESTED {
                return;
            }
        }
    }

    /// Broadcasts average FPS to the UI at most once per second.
    fn emit_average_fps_if_due(&mut self, now: Instant) {
        if now.duration_since(self.last_average_fps_emit)
            < Duration::from_secs(1)
        {
            return;
        }

        self.last_average_fps_emit = now;
        self.emit_web_view_event(web_view::Event::AverageFps(
            frame_clock::average_fps(),
        ));
    }

    /// Runs one update/render pass when a redraw is requested.
    ///
    /// Order matters:
    ///
    /// 1. Update sketch + hub, then write uniforms.
    /// 2. Acquire the surface frame and execute the graph.
    /// 3. Encode recording/capture readback copies before submit.
    /// 4. Submit once, then run post-submit host-side work.
    fn render(&mut self, event_loop: &ActiveEventLoop) {
        if !self.render_requested {
            return;
        }

        self.render_requested = false;

        let (
            pending_png_capture,
            pending_png_capture_error,
            capture_device,
            capture_submission_index,
            monitor_render_result,
        ) = {
            // 1) Resolve runtime resources for this frame.
            let Some(context) = self.context.as_mut() else {
                return;
            };
            let Some(uniforms) = self.uniforms.as_mut() else {
                return;
            };
            let Some(graph) = self.graph.as_mut() else {
                return;
            };
            let Some(surface_config) = self.surface_config.as_ref() else {
                return;
            };

            let render_size = projector::internal_render_size(
                [surface_config.width, surface_config.height],
                self.projector_mode_enabled,
                self.projector_quality,
            );
            context.set_render_size(render_size);

            // 2) Let sketch mutate runtime state before uniform upload.
            self.sketch.update(context);

            // 3) Runtime-owned uniforms: resolution + beat source + hub vars.
            let [w, h] = context.resolution();
            uniforms.set_resolution(w, h);
            let current_beats;
            let video_transports;

            if let Some(hub) = self.control_hub.as_mut() {
                hub.update();
                video_transports = hub.video_transports();

                for (id, value) in hub.var_values() {
                    if let Err(err) = uniforms.set(&id, value) {
                        warn!(
                            "ignoring control var '{}' for sketch '{}': {}",
                            id, self.config.name, err
                        );
                    }
                }

                current_beats = hub.beats();
            } else {
                current_beats = context.elapsed_seconds();
                video_transports = HashMap::default();
            }

            uniforms.set_beats(current_beats);
            uniforms.upload(context.queue.as_ref());

            // 4) Acquire current presentation surface texture.
            let Some(surface) = self.surface.as_mut() else {
                return;
            };

            let output = match surface.get_current_texture() {
                Ok(output) => output,
                Err(
                    wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
                ) => {
                    surface.configure(context.device.as_ref(), surface_config);
                    return;
                }
                Err(wgpu::SurfaceError::Timeout) => {
                    warn!("surface timeout while acquiring frame");
                    return;
                }
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    error!("surface out of memory; exiting");
                    self.shutdown(event_loop);
                    return;
                }
                Err(wgpu::SurfaceError::Other) => {
                    warn!("surface error while acquiring frame");
                    return;
                }
            };

            // 5) Build frame command context and execute graph.
            let mut frame = Frame::new(
                context.device.as_ref(),
                context.queue.clone(),
                output,
            );

            self.sketch.view(&mut frame, context);

            let execute_ctx = ExecuteCtx {
                device: context.device.as_ref(),
                queue: context.queue.as_ref(),
                uniforms,
                video_transports: &video_transports,
                beats: current_beats,
                bpm: self.bpm.get(),
                render_size: context.resolution_u32(),
                surface_size: [surface_config.width, surface_config.height],
            };

            if let Err(err) = graph.execute(&mut frame, execute_ctx) {
                error!("graph execution error: {}", err);
                event_loop.exit();
                return;
            }

            // 6) Recording readback copy is encoded pre-submit.
            if self.recording_state.is_recording
                && let Some(recorder) = self.recording_state.recorder.as_mut()
            {
                if let Some(source_texture) = graph.recording_source_texture() {
                    let encoder = frame.encoder();
                    let _ =
                        recorder.capture_surface_frame(encoder, source_texture);
                } else {
                    let (encoder, source_texture) =
                        frame.encoder_and_output_texture();
                    let _ =
                        recorder.capture_surface_frame(encoder, source_texture);
                }
            }

            // 7) Optional still-image capture readback copy is also pre-submit.
            let pending_png_capture_error: Option<String> = None;
            let pending_png_capture =
                if let Some(path) = self.pending_png_capture_path.take() {
                    let source_format = surface_config.format;
                    let (encoder, source_texture) =
                        frame.encoder_and_output_texture();
                    let width = source_texture.size().width.max(1);
                    let height = source_texture.size().height.max(1);
                    let bytes_per_pixel = 4u32;
                    let unpadded_bytes_per_row = width * bytes_per_pixel;
                    let padded_bytes_per_row = unpadded_bytes_per_row
                        + compute_row_padding(unpadded_bytes_per_row);
                    let buffer_size =
                        (padded_bytes_per_row as u64) * (height as u64);
                    let buffer =
                        context.device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("xtal-capture-readback"),
                            size: buffer_size,
                            usage: wgpu::BufferUsages::COPY_DST
                                | wgpu::BufferUsages::MAP_READ,
                            mapped_at_creation: false,
                        });

                    encoder.copy_texture_to_buffer(
                        wgpu::TexelCopyTextureInfo {
                            texture: source_texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyBufferInfo {
                            buffer: &buffer,
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(padded_bytes_per_row),
                                rows_per_image: Some(height),
                            },
                        },
                        wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                    );

                    Some(PendingPngCapture {
                        path,
                        buffer,
                        width,
                        height,
                        padded_bytes_per_row,
                        source_format,
                    })
                } else {
                    None
                };

            let mut monitor_fallback_texture = None;
            if self.monitor_preview.is_some()
                && graph.recording_source_texture().is_none()
            {
                let (encoder, source_texture) =
                    frame.encoder_and_output_texture();
                let size = source_texture.size();
                let fallback =
                    context.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("xtal-monitor-preview-fallback"),
                        size: wgpu::Extent3d {
                            width: size.width.max(1),
                            height: size.height.max(1),
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: surface_config.format,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING
                            | wgpu::TextureUsages::COPY_DST,
                        view_formats: &[],
                    });

                encoder.copy_texture_to_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: source_texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyTextureInfo {
                        texture: &fallback,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::Extent3d {
                        width: size.width.max(1),
                        height: size.height.max(1),
                        depth_or_array_layers: 1,
                    },
                );
                monitor_fallback_texture = Some(fallback);
            }

            // 8) Submit all encoded GPU work once.
            let submission_index = frame.submit();

            // 9) Mirror to optional monitor preview from the finalized graph
            // source texture. Do not sample from surface output textures.
            let monitor_render_result =
                if let Some(preview) = self.monitor_preview.as_mut() {
                    let source_texture = graph
                        .recording_source_texture()
                        .or(monitor_fallback_texture.as_ref());
                    source_texture.map(|texture| {
                        preview.render_if_due(context, texture, Instant::now())
                    })
                } else {
                    None
                };

            if self.recording_state.is_recording
                && let Some(recorder) = self.recording_state.recorder.as_mut()
            {
                recorder.on_submitted();
            }

            // 10) Advance local frame-time state after successful submits.
            context.next_frame();

            (
                pending_png_capture,
                pending_png_capture_error,
                context.device.clone(),
                submission_index,
                monitor_render_result,
            )
        };

        // 11) Post-submit host-side effects/events.
        if matches!(
            monitor_render_result,
            Some(MonitorRenderResult::OutOfMemory)
        ) {
            error!("monitor preview surface out of memory; exiting");
            self.shutdown(event_loop);
            return;
        }

        if let Some(message) = pending_png_capture_error {
            self.alert_and_log(message, log::Level::Error);
        }

        if self.recording_state.is_encoding
            && let Some(outcome) =
                self.recording_state.poll_finalize(&mut self.session_id)
        {
            if outcome.is_error {
                self.alert_and_log(outcome.message, log::Level::Error);
            } else {
                self.alert(outcome.message);
            }
            self.emit_web_view_event(web_view::Event::Encoding(
                self.recording_state.is_encoding,
            ));
        }

        if let Some(capture) = pending_png_capture {
            queue_png_capture_save(
                capture_device,
                capture_submission_index,
                capture,
                self.event_tx.clone(),
            );
        }
    }

    /// Converts main-window shortcuts into the same events used by the UI.
    ///
    /// The web view has its own keyboard focus, so this only handles shortcuts
    /// delivered to the render window. Keeping shortcuts routed through
    /// `RuntimeEvent` avoids a second implementation path for the same actions.
    fn handle_main_window_shortcut(
        &mut self,
        event_loop: &ActiveEventLoop,
        key_event: &KeyEvent,
    ) -> bool {
        if key_event.state != ElementState::Pressed || key_event.repeat {
            return false;
        }

        let PhysicalKey::Code(code) = key_event.physical_key else {
            return false;
        };

        let platform_mod_pressed = if cfg!(target_os = "macos") {
            self.modifiers.super_key()
        } else {
            self.modifiers.control_key()
        };
        let shift_pressed = self.modifiers.shift_key();
        let has_no_modifiers = !self.modifiers.alt_key()
            && !self.modifiers.control_key()
            && !self.modifiers.shift_key()
            && !self.modifiers.super_key();

        if let Some(digit) = digit_from_key_code(code) {
            let sequence_enabled = self
                .control_hub
                .as_ref()
                .is_some_and(|hub| hub.snapshot_sequence_enabled());
            if !sequence_enabled {
                if platform_mod_pressed {
                    return self.on_runtime_event(
                        event_loop,
                        RuntimeEvent::SnapshotRecall(digit.to_string()),
                    );
                }
                if shift_pressed {
                    return self.on_runtime_event(
                        event_loop,
                        RuntimeEvent::SnapshotStore(digit.to_string()),
                    );
                }
            }
        }

        match code {
            KeyCode::KeyA => {
                if frame_clock::paused() {
                    return self.on_runtime_event(
                        event_loop,
                        RuntimeEvent::AdvanceSingleFrame,
                    );
                }
            }
            KeyCode::KeyF => {
                return self.on_runtime_event(
                    event_loop,
                    RuntimeEvent::ToggleFullScreen,
                );
            }
            KeyCode::KeyG => {
                self.emit_web_view_event(web_view::Event::ToggleGuiFocus);
            }
            KeyCode::KeyI => {
                return self
                    .on_runtime_event(event_loop, RuntimeEvent::CaptureFrame);
            }
            KeyCode::KeyM => {
                if !platform_mod_pressed {
                    return self.on_runtime_event(
                        event_loop,
                        RuntimeEvent::ToggleMainFocus,
                    );
                }
            }
            KeyCode::KeyP => {
                let paused = !frame_clock::paused();
                let _ = self
                    .on_runtime_event(event_loop, RuntimeEvent::Pause(paused));
                self.emit_web_view_event(web_view::Event::Paused(paused));
            }
            KeyCode::KeyQ => {
                if platform_mod_pressed {
                    return self
                        .on_runtime_event(event_loop, RuntimeEvent::Quit);
                }
            }
            KeyCode::KeyR => {
                if platform_mod_pressed && shift_pressed {
                    return self.on_runtime_event(
                        event_loop,
                        RuntimeEvent::SwitchSketch(
                            self.active_sketch_name.clone(),
                        ),
                    );
                }
                if platform_mod_pressed {
                    let exclusions = self.current_sketch_ui_state().exclusions;
                    return self.on_runtime_event(
                        event_loop,
                        RuntimeEvent::Randomize(exclusions),
                    );
                }
                if has_no_modifiers {
                    return self
                        .on_runtime_event(event_loop, RuntimeEvent::Reset);
                }
            }
            KeyCode::KeyS => {
                if platform_mod_pressed || shift_pressed {
                    let exclusions = self.current_sketch_ui_state().exclusions;
                    return self.on_runtime_event(
                        event_loop,
                        RuntimeEvent::Save(exclusions),
                    );
                }
            }
            KeyCode::Space if self.tap_tempo_enabled => {
                return self.on_runtime_event(event_loop, RuntimeEvent::Tap);
            }
            _ => {}
        }

        false
    }

    /// Creates the window, GPU resources, render context, and sketch graph.
    ///
    /// Winit calls this from `resumed`, which is the first point where creating
    /// platform windows and surfaces is valid.
    fn init_runtime(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), String> {
        let attrs = WindowAttributes::default()
            .with_title(self.config.display_name)
            .with_inner_size(LogicalSize::new(self.config.w, self.config.h));

        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .map_err(|err| err.to_string())?,
        );
        anchor_window_top_left(window.as_ref());

        let instance =
            wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let surface = instance
            .create_surface(window.clone())
            .map_err(|err| err.to_string())?;

        let adapter = pollster::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            },
        ))
        .map_err(|err| err.to_string())?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("xtal-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::default(),
            },
        ))
        .map_err(|err| err.to_string())?;

        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let caps = surface.get_capabilities(&adapter);
        let format = choose_surface_format(&caps.formats)
            .ok_or_else(|| "surface has no supported formats".to_string())?;

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            // Keep swapchain queue shallow to reduce visual beat latency.
            desired_maximum_frame_latency: 1,
        };

        surface.configure(&device, &surface_config);

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let context = Context::new(
            device.clone(),
            queue.clone(),
            [width, height],
            window.scale_factor(),
        );

        self.window_id = Some(window.id());
        self.window = Some(window);
        self.instance = Some(instance);
        self.adapter = Some(adapter);
        self.surface = Some(surface);
        self.surface_config = Some(surface_config);
        self.context = Some(context);
        self.sync_context_render_size();

        self.rebuild_graph_state()?;

        Ok(())
    }

    /// Rebuilds graph, uniforms, and control hub for startup or sketch switch.
    ///
    /// Rebuilding the control hub here attaches the active sketch's YAML
    /// controls and timing source to the render graph. Saved per-sketch state
    /// is restored after the hub exists so persisted values apply to current
    /// control definitions.
    fn rebuild_graph_state(&mut self) -> Result<(), String> {
        let mut graph_builder = GraphBuilder::new();
        graph_builder.set_videos_dir(self.videos_dir.clone());
        self.sketch.setup(&mut graph_builder);
        let graph_spec = graph_builder.build();

        let Some(context) = self.context.as_ref() else {
            return Err("runtime context not initialized".to_string());
        };
        let uniforms = UniformBanks::new(
            context.device.as_ref(),
            self.config.banks.max(1),
        );

        self.control_hub = self.build_control_hub();
        self.restore_sketch_state_from_disk();

        let Some(surface_config) = self.surface_config.as_ref() else {
            return Err("surface config not initialized".to_string());
        };
        let Some(context) = self.context.as_ref() else {
            return Err("runtime context not initialized".to_string());
        };
        let graph = CompiledGraph::compile(
            context.device.as_ref(),
            context.queue.as_ref(),
            surface_config.format,
            graph_spec,
            uniforms.bind_group_layout(),
        )?;

        self.uniforms = Some(uniforms);
        self.graph = Some(graph);

        Ok(())
    }

    /// Builds the control hub from the active sketch control script.
    ///
    /// This is where sketch controls receive their timing source. MIDI, audio,
    /// snapshot, and populated callbacks are also bridged back into the runtime
    /// command channel here.
    fn build_control_hub(&self) -> Option<ControlHub<Timing>> {
        let path = self.sketch.control_script()?;

        if !path.exists() {
            warn!(
                "control script for sketch '{}' does not exist: {}",
                self.config.name,
                path.display()
            );
            return None;
        }

        let mut hub = ControlHub::from_path(path, self.build_timing());
        hub.set_transition_time(self.transition_time);
        hub.midi_overrides_enabled = self.mappings_enabled;
        hub.midi_controls.hrcc = self.hrcc;
        hub.midi_controls.set_port(self.midi_input_port.clone());
        hub.midi_controls
            .set_override_state(hub.midi_overrides.clone());
        hub.midi_controls
            .set_override_configs(hub.midi_override_configs.clone());
        hub.audio_controls
            .set_device_name(self.audio_device.clone());
        info!(
            "Configuring sketch MIDI controls: \
                input_port='{}', hrcc={}, mappings_enabled={}",
            self.midi_input_port, self.hrcc, self.mappings_enabled
        );
        hub.midi_controls
            .restart()
            .inspect_err(|err| {
                error!("Error in build_control_hub MIDI setup: {}", err)
            })
            .ok();
        info!(
            "Configuring sketch audio controls: device='{}'",
            self.audio_device
        );
        hub.audio_controls
            .restart()
            .inspect_err(|err| {
                error!("Error in build_control_hub audio setup: {}", err)
            })
            .ok();
        info!(
            "Audio startup state: device='{}', active={}",
            self.audio_device,
            hub.audio_controls.is_active()
        );
        let populated_tx = self.command_tx.clone();
        hub.register_populated_callback(move || {
            let _ = populated_tx.send(RuntimeEvent::HubPopulated);
        });
        let snapshot_ended_tx = self.command_tx.clone();
        hub.register_snapshot_ended_callback(move || {
            let _ = snapshot_ended_tx.send(RuntimeEvent::SnapshotEnded);
        });
        hub.mark_unchanged();
        Some(hub)
    }

    /// Builds the effective timing source for the active sketch.
    ///
    /// The CLI timing override wins over the sketch's configured timing mode.
    /// MIDI and hybrid timing also receive a callback that reports transport
    /// Start/Continue/Stop back to the runtime.
    fn build_timing(&self) -> Timing {
        match self.effective_timing_mode() {
            TimingMode::Frame => Timing::frame(self.bpm.clone()),
            TimingMode::Osc => Timing::osc(self.bpm.clone()),
            TimingMode::Midi => Timing::midi_with_port(
                self.bpm.clone(),
                &self.midi_clock_port,
                self.midi_transport_event_sender(),
            ),
            TimingMode::Hybrid => Timing::hybrid_with_port(
                self.bpm.clone(),
                &self.midi_clock_port,
                self.midi_transport_event_sender(),
            ),
            TimingMode::Manual => Timing::manual(self.bpm.clone()),
        }
    }

    /// Returns the timing mode after applying the optional CLI override.
    fn effective_timing_mode(&self) -> TimingMode {
        self.timing_mode_override
            .unwrap_or_else(|| self.sketch.timing_mode())
    }

    /// Creates the callback timing sources use for MIDI transport events.
    ///
    /// Timing code owns clock parsing. The runtime only needs transport
    /// lifecycle events so it can reset graph state and start/stop recording.
    fn midi_transport_event_sender(
        &self,
    ) -> impl Fn(MidiTransportEvent) + Send + Sync + 'static {
        let command_tx = self.command_tx.clone();
        move |event| {
            let event = match event {
                MidiTransportEvent::Continue => RuntimeEvent::MidiContinue,
                MidiTransportEvent::Start => RuntimeEvent::MidiStart,
                MidiTransportEvent::Stop => RuntimeEvent::MidiStop,
            };
            let _ = command_tx.send(event);
        }
    }

    /// Connects or reconnects MIDI output for sending control snapshots.
    fn connect_midi_out(&mut self) {
        if self.midi_output_port.is_empty() {
            info!("Skipping MIDI output connection; no MIDI output port.");
            self.midi_out = None;
            return;
        }

        info!("Connecting MIDI output on port '{}'", self.midi_output_port);

        let mut midi_out = midi::MidiOut::new(&self.midi_output_port);
        self.midi_out = match midi_out.connect() {
            Ok(_) => {
                info!("Connected MIDI output on '{}'", self.midi_output_port);
                Some(midi_out)
            }
            Err(err) => {
                error!("{}", err);
                None
            }
        };
    }

    /// Logs resolved MIDI and OSC startup selections.
    fn log_midi_startup_state(&self) {
        info!(
            "MIDI/OSC startup state: input_port='{}', \
                output_port='{}', clock_port='{}', osc_port={}, \
                hrcc={}, mappings_enabled={}",
            self.midi_input_port,
            self.midi_output_port,
            self.midi_clock_port,
            self.osc_port,
            self.hrcc,
            self.mappings_enabled
        );
        debug!("MIDI input ports: {:?}", self.midi_input_ports);
        debug!("MIDI output ports: {:?}", self.midi_output_ports);
    }

    /// Resolves persisted MIDI ports against currently available ports.
    ///
    /// Returns true when any persisted selection was replaced so global
    /// settings can be saved with the resolved value.
    fn normalize_midi_port_selections(&mut self) -> bool {
        let mut changed = false;

        if !self.midi_input_ports.is_empty() {
            let has_input = self
                .midi_input_ports
                .iter()
                .any(|(_, name)| *name == self.midi_input_port);
            if !has_input {
                let previous = self.midi_input_port.clone();
                self.midi_input_port = self.midi_input_ports[0].1.clone();
                if !previous.is_empty() {
                    warn!(
                        "Persisted MIDI input port '{}' not found; \
                            using '{}' for this session",
                        previous, self.midi_input_port
                    );
                }
                info!(
                    "Resolved MIDI input port from '{}' to '{}'",
                    if previous.is_empty() {
                        "<empty>"
                    } else {
                        &previous
                    },
                    self.midi_input_port
                );
                changed = true;
            }

            let has_clock = self
                .midi_input_ports
                .iter()
                .any(|(_, name)| *name == self.midi_clock_port);
            if !has_clock {
                let previous = self.midi_clock_port.clone();
                self.midi_clock_port = self.midi_input_ports[0].1.clone();
                if !previous.is_empty() {
                    warn!(
                        "Persisted MIDI clock port '{}' not found; \
                            using '{}' for this session",
                        previous, self.midi_clock_port
                    );
                }
                info!(
                    "Resolved MIDI clock port from '{}' to '{}'",
                    if previous.is_empty() {
                        "<empty>"
                    } else {
                        &previous
                    },
                    self.midi_clock_port
                );
                changed = true;
            }
        }

        if !self.midi_output_ports.is_empty() {
            let has_output = self
                .midi_output_ports
                .iter()
                .any(|(_, name)| *name == self.midi_output_port);
            if !has_output {
                let previous = self.midi_output_port.clone();
                self.midi_output_port = self.midi_output_ports[0].1.clone();
                if !previous.is_empty() {
                    warn!(
                        "Persisted MIDI output port '{}' not found; \
                            using '{}' for this session",
                        previous, self.midi_output_port
                    );
                }
                info!(
                    "Resolved MIDI output port from '{}' to '{}'",
                    if previous.is_empty() {
                        "<empty>"
                    } else {
                        &previous
                    },
                    self.midi_output_port
                );
                changed = true;
            }
        }

        changed
    }

    /// Resolves the persisted audio device against currently available devices.
    fn normalize_audio_device_selection(&mut self) -> bool {
        if self.audio_devices.is_empty() {
            return false;
        }

        if self.audio_devices.iter().any(|d| d == &self.audio_device) {
            return false;
        }

        let previous = self.audio_device.clone();
        self.audio_device = self.audio_devices[0].clone();
        info!(
            "Resolved audio device from '{}' to '{}'",
            if previous.is_empty() {
                "<empty>"
            } else {
                &previous
            },
            self.audio_device
        );
        true
    }

    /// Replaces an invalid persisted OSC port with the runtime default.
    fn normalize_osc_port_selection(&mut self) -> bool {
        if self.osc_port == 0 {
            self.osc_port = DEFAULT_OSC_PORT;
            info!("Resolved OSC port from 0 to {}", self.osc_port);
            return true;
        }
        false
    }

    /// Restarts the shared OSC receiver on the current runtime port.
    ///
    /// The receiver itself is shared by OSC controls and OSC timing; runtime
    /// only owns the configured port and restarts the shared listener when that
    /// setting changes.
    fn restart_osc_receiver(&self) {
        if let Err(err) = SHARED_OSC_RECEIVER.restart(self.osc_port) {
            error!("Failed to restart OSC receiver: {}", err);
        }
    }

    /// Applies resize to surface config and runtime context resolution.
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        let Some(surface) = self.surface.as_ref() else {
            return;
        };
        let Some(surface_config) = self.surface_config.as_mut() else {
            return;
        };
        let Some(context) = self.context.as_mut() else {
            return;
        };

        surface_config.width = new_size.width;
        surface_config.height = new_size.height;

        surface.configure(context.device.as_ref(), surface_config);
        context.set_window_size([new_size.width, new_size.height]);
        self.sync_context_render_size();
        if let Some(preview) = self.monitor_preview.as_ref() {
            self.monitor_preview_size_hint =
                Some(preview.window().inner_size());
        }
    }

    /// Emits a runtime notification to external listeners.
    ///
    /// This is the runtime-to-bridge path, not the command path. It is used for
    /// web view updates and lifecycle notifications.
    fn emit_event(&self, event: RuntimeEvent) {
        let Some(event_tx) = self.event_tx.as_ref() else {
            return;
        };

        if let Err(err) = event_tx.send(event) {
            warn!("failed to emit runtime event: {}", err);
        }
    }

    /// Wraps a web view event in `RuntimeEvent::WebView` and emits it.
    fn emit_web_view_event(&self, event: web_view::Event) {
        self.emit_event(RuntimeEvent::WebView(Box::new(event)));
    }

    /// Returns cached per-sketch UI state.
    fn current_sketch_ui_state(&self) -> SketchUiState {
        self.sketch_ui_state
            .get(&self.active_sketch_name)
            .cloned()
            .unwrap_or_default()
    }

    /// Returns mutable per-sketch UI state, creating default if needed.
    fn current_sketch_ui_state_mut(&mut self) -> &mut SketchUiState {
        self.sketch_ui_state
            .entry(self.active_sketch_name.clone())
            .or_default()
    }

    /// Derives the web-view mapping payload from hub MIDI override configs.
    fn mappings_from_hub(&self) -> web_view::Mappings {
        let Some(hub) = self.control_hub.as_ref() else {
            return HashMap::default();
        };

        let mut mappings = HashMap::default();

        for (name, config) in &hub.midi_override_configs {
            mappings.insert(
                name.to_string(),
                (config.channel as usize, config.cc as usize),
            );
        }

        mappings
    }

    /// Sends the one-time UI bootstrap payload.
    ///
    /// This initializes global settings, port/device lists, sketch catalog
    /// data, and persisted directory choices before the active sketch payload
    /// is sent.
    fn emit_web_view_init(&self) {
        let event = web_view::Event::Init {
            audio_device: self.audio_device.clone(),
            audio_devices: self.audio_devices.clone(),
            hrcc: self.hrcc,
            images_dir: self.images_dir.clone(),
            is_light_theme: true,
            mappings_enabled: self.mappings_enabled,
            midi_clock_port: self.midi_clock_port.clone(),
            midi_input_port: self.midi_input_port.clone(),
            midi_output_port: self.midi_output_port.clone(),
            midi_input_ports: self.midi_input_ports.clone(),
            midi_output_ports: self.midi_output_ports.clone(),
            monitor_preview_enabled: self.monitor_preview.is_some(),
            osc_port: self.osc_port,
            projector_mode_enabled: self.projector_mode_enabled,
            projector_quality: self.projector_quality,
            sketches_by_category: web_view::sketches_by_category(
                &self.registry,
            ),
            sketch_catalog: Some(web_view::sketch_catalog_from_registry(
                &self.registry,
            )),
            sketch_name: self.active_sketch_name.clone(),
            transition_time: self.transition_time,
            user_data_dir: self.user_data_dir.clone(),
            videos_dir: self.videos_dir.clone(),
        };

        self.emit_web_view_event(event);
    }

    /// Sends the active sketch payload to the web view.
    ///
    /// This includes controls, bypassed state, snapshot slots, mappings,
    /// exclusions, BPM, play-state toggles, and sketch dimensions.
    fn emit_web_view_load_sketch(&mut self) {
        let controls = self
            .control_hub
            .as_ref()
            .map_or_else(Vec::new, web_view::controls_from_hub);

        let bypassed = self
            .control_hub
            .as_ref()
            .map_or_else(Default::default, ControlHub::bypassed);

        let snapshot_slots = self
            .control_hub
            .as_ref()
            .map_or_else(Vec::new, ControlHub::snapshot_keys_sorted);

        let snapshot_sequence_enabled = self
            .control_hub
            .as_ref()
            .is_some_and(ControlHub::snapshot_sequence_enabled);

        let mappings = {
            let inferred_mappings = self.mappings_from_hub();
            let sketch_state = self.current_sketch_ui_state_mut();
            if sketch_state.mappings.is_empty() {
                sketch_state.mappings = inferred_mappings;
            }
            sketch_state.mappings.clone()
        };
        self.map_mode.set_mappings(mappings.clone());
        let exclusions = self.current_sketch_ui_state().exclusions;

        let event = web_view::Event::LoadSketch {
            bpm: self.bpm.get(),
            bypassed,
            controls,
            display_name: self.config.display_name.to_string(),
            fps: self.config.fps,
            mappings,
            paused: frame_clock::paused(),
            perf_mode: self.perf_mode,
            sketch_name: self.active_sketch_name.clone(),
            sketch_width: self.config.w as i32,
            sketch_height: self.config.h as i32,
            snapshot_slots,
            snapshot_sequence_enabled,
            tap_tempo_enabled: self.tap_tempo_enabled,
            exclusions,
        };

        self.emit_web_view_event(event);
    }

    /// Applies one UI control mutation into the hub.
    ///
    /// In advance mode, control edits should be visible immediately even though
    /// the frame clock is paused, so this advances and redraws one frame.
    fn apply_control_update(&mut self, name: String, value: ControlValue) {
        let Some(hub) = self.control_hub.as_mut() else {
            warn!(
                "ignoring control update for '{}' because no control hub \
                    is active",
                name
            );
            return;
        };

        hub.ui_controls.set(&name, value);

        if self.config.play_mode == PlayMode::Advance && frame_clock::paused() {
            frame_clock::advance_single_frame();
            self.request_render_now();
        }
    }

    /// Resets frame timing plus graph-owned video/animation transport state.
    ///
    /// MIDI Start/Continue, manual reset, and sketch changes all converge here
    /// so the frame clock, graph, and video transports return to beat zero
    /// together.
    fn reset_transport(&mut self) {
        frame_clock::reset();
        let video_transports = self
            .control_hub
            .as_ref()
            .map_or_else(HashMap::default, ControlHub::video_transports);
        if let (Some(graph), Some(context)) =
            (self.graph.as_mut(), self.context.as_ref())
        {
            graph.reset(
                context.device.as_ref(),
                context.queue.as_ref(),
                &video_transports,
                self.bpm.get(),
            );
        }
        self.request_render_now();
    }

    /// Swaps sketch instance/config, rebuilds graph state, and updates the UI.
    ///
    /// Tap-tempo BPM is preserved across sketches while tap tempo is active;
    /// otherwise the new sketch's configured BPM becomes the runtime BPM.
    fn switch_sketch(&mut self, name: &str) -> Result<(), String> {
        self.map_mode.stop();

        let preserved_bpm = self.bpm.get();
        let (config, sketch) = instantiate_sketch(&self.registry, name)?;

        self.active_sketch_name = name.to_string();
        self.config = config;
        self.sketch = sketch;
        let next_bpm = if self.tap_tempo_enabled {
            preserved_bpm
        } else {
            self.config.bpm
        };
        self.bpm.set(next_bpm);
        self.tap_tempo = TapTempo::new(next_bpm);
        frame_clock::set_fps(self.config.fps);
        frame_clock::reset_timing(Instant::now());
        self.apply_play_mode();

        if let Some(window) = self.window.as_ref() {
            window.set_title(self.config.display_name);
            if !self.perf_mode {
                anchor_window_top_left(window.as_ref());
                let _ = window.request_inner_size(LogicalSize::new(
                    self.config.w,
                    self.config.h,
                ));
            }
        }
        self.rebuild_graph_state()?;

        info!(
            "switched sketch to '{}' ({})",
            self.active_sketch_name, self.config.display_name
        );

        self.emit_event(RuntimeEvent::SketchSwitched(
            self.active_sketch_name.clone(),
        ));
        self.emit_web_view_load_sketch();
        self.alert(format!("Switched to {}", self.config.display_name));
        // Ensure frame 0 is visible even when play mode starts paused.
        self.request_render_now();

        Ok(())
    }

    /// Requests a redraw on the next event loop cycle.
    fn request_render_now(&mut self) {
        self.render_requested = true;
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    /// Applies the active sketch's play mode to the frame clock.
    fn apply_play_mode(&self) {
        let paused = match self.config.play_mode {
            PlayMode::Loop => false,
            PlayMode::Pause | PlayMode::Advance => true,
        };
        frame_clock::set_paused(paused);
    }

    /// Toggles performance-mode window policy.
    ///
    /// Leaving performance mode restores the sketch's configured window size
    /// and anchor. Entering performance mode leaves the user's current layout
    /// alone.
    fn set_perf_mode(&mut self, perf_mode: bool) {
        if self.perf_mode == perf_mode {
            return;
        }

        self.perf_mode = perf_mode;
        info!("performance mode set to {}", self.perf_mode);

        if let Some(window) = self.window.as_ref() {
            if !self.perf_mode {
                anchor_window_top_left(window.as_ref());
                let _ = window.request_inner_size(LogicalSize::new(
                    self.config.w,
                    self.config.h,
                ));
            }

            window.request_redraw();
        }

        if let Some(preview) = self.monitor_preview.as_ref() {
            self.monitor_preview_size_hint =
                Some(preview.window().inner_size());
        }
    }

    /// Enables or disables projector render sizing.
    fn set_projector_mode(&mut self, enabled: bool) {
        if self.projector_mode_enabled == enabled {
            return;
        }

        self.projector_mode_enabled = enabled;
        info!("projector mode set to {}", enabled);
        self.sync_context_render_size();
        self.save_global_state();
        self.request_render_now();
    }

    /// Changes projector render quality and recomputes internal render size.
    fn set_projector_quality(&mut self, quality: ProjectorQuality) {
        if self.projector_quality == quality {
            return;
        }

        self.projector_quality = quality;
        info!("projector quality set to {:?}", quality);
        self.sync_context_render_size();
        self.save_global_state();
        self.request_render_now();
    }

    /// Synchronizes context render size with surface and projector settings.
    fn sync_context_render_size(&mut self) {
        let Some(surface_config) = self.surface_config.as_ref() else {
            return;
        };
        let Some(context) = self.context.as_mut() else {
            return;
        };

        let render_size = projector::internal_render_size(
            [surface_config.width, surface_config.height],
            self.projector_mode_enabled,
            self.projector_quality,
        );
        context.set_render_size(render_size);
    }

    /// Opens or closes the monitor preview window.
    fn set_monitor_preview_enabled(
        &mut self,
        event_loop: &ActiveEventLoop,
        enabled: bool,
    ) {
        if self.monitor_preview.is_some() == enabled {
            return;
        }

        if enabled {
            if let Err(err) = self.create_monitor_preview(event_loop) {
                self.alert_and_log(
                    format!("Failed to open monitor preview: {}", err),
                    log::Level::Error,
                );
                self.emit_web_view_event(web_view::Event::MonitorPreview(
                    false,
                ));
                return;
            }
            self.request_render_now();
        } else {
            if let Some(preview) = self.monitor_preview.as_ref() {
                self.monitor_preview_size_hint =
                    Some(preview.window().inner_size());
            }
            self.monitor_preview = None;
        }

        self.emit_web_view_event(web_view::Event::MonitorPreview(enabled));
    }

    /// Creates a monitor preview using the existing GPU instance and adapter.
    fn create_monitor_preview(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), String> {
        let Some(instance) = self.instance.as_ref() else {
            return Err("wgpu instance is not initialized".to_string());
        };
        let Some(adapter) = self.adapter.as_ref() else {
            return Err("wgpu adapter is not initialized".to_string());
        };
        let Some(context) = self.context.as_ref() else {
            return Err("runtime context is not initialized".to_string());
        };
        let Some(surface_config) = self.surface_config.as_ref() else {
            return Err("surface config is not initialized".to_string());
        };

        let initial_size = if self.perf_mode {
            self.monitor_preview_size_hint.unwrap_or_else(|| {
                preview_size_for_main(
                    surface_config.width,
                    surface_config.height,
                )
            })
        } else {
            preview_size_for_main(surface_config.width, surface_config.height)
        };

        let preview = MonitorPreview::create(
            event_loop,
            instance,
            adapter,
            context.device.as_ref(),
            initial_size,
        )?;

        preview.window().focus_window();
        preview.window().request_redraw();
        self.monitor_preview_size_hint = Some(preview.window().inner_size());
        self.monitor_preview = Some(preview);

        Ok(())
    }

    /// Sends a UI alert message.
    fn alert(&self, message: impl Into<String>) {
        self.emit_web_view_event(web_view::Event::Alert(message.into()));
    }

    /// Sends a UI alert and emits a log entry with the same message.
    fn alert_and_log(&self, message: impl Into<String>, level: log::Level) {
        let message = message.into();
        self.alert(message.clone());
        match level {
            log::Level::Error => error!("{}", message),
            log::Level::Warn => warn!("{}", message),
            log::Level::Info => info!("{}", message),
            log::Level::Debug => debug!("{}", message),
            log::Level::Trace => trace!("{}", message),
        }
    }

    /// Resolves a requested OS directory kind to an absolute path.
    fn os_dir_path(&self, kind: &web_view::OsDir) -> PathBuf {
        match kind {
            web_view::OsDir::Cache => storage::cache_dir()
                .unwrap_or_else(|| env::temp_dir().join("Xtal")),
            web_view::OsDir::Config => PathBuf::from(&self.user_data_dir),
        }
    }

    /// Updates cached randomize/save exclusions for the active sketch.
    fn set_exclusions(&mut self, exclusions: web_view::Exclusions) {
        self.current_sketch_ui_state_mut().exclusions = exclusions;
    }

    /// Persists global runtime settings to the user data directory.
    ///
    /// Per-sketch controls are saved separately through `RuntimeEvent::Save`.
    fn save_global_state(&self) {
        let settings = GlobalSettings {
            version: super::serialization::GLOBAL_SETTINGS_VERSION.to_string(),
            audio_device_name: self.audio_device.clone(),
            hrcc: self.hrcc,
            images_dir: self.images_dir.clone(),
            mappings_enabled: self.mappings_enabled,
            midi_clock_port: self.midi_clock_port.clone(),
            midi_control_in_port: self.midi_input_port.clone(),
            midi_control_out_port: self.midi_output_port.clone(),
            osc_port: self.osc_port,
            projector_mode_enabled: self.projector_mode_enabled,
            projector_quality: self.projector_quality,
            transition_time: self.transition_time,
            user_data_dir: self.user_data_dir.clone(),
            videos_dir: self.videos_dir.clone(),
        };

        match storage::save_global_state(&self.user_data_dir, settings) {
            Ok(()) => {
                let path = PathBuf::from(&self.user_data_dir)
                    .join("global_settings.json");
                info!("Global settings saved to {}", path.display());
            }
            Err(err) => {
                self.alert_and_log(
                    format!("Failed to persist global settings: {}", err),
                    log::Level::Error,
                );
            }
        }
    }

    /// Loads per-sketch controls, snapshots, mappings, and exclusions.
    ///
    /// This intentionally restores persisted values into the freshly populated
    /// hub instead of replacing current UI control definitions, so YAML schema
    /// changes remain authoritative.
    fn restore_sketch_state_from_disk(&mut self) {
        let current = self.current_sketch_ui_state();
        self.map_mode.set_mappings(current.mappings.clone());
        let Some(hub) = self.control_hub.as_mut() else {
            return;
        };

        let mut state = TransitorySketchState::from_hub(
            hub,
            current.mappings,
            current.exclusions,
        );

        let result = storage::load_sketch_state(
            &self.user_data_dir,
            &self.active_sketch_name,
            &mut state,
        );

        match result {
            Ok(state) => {
                let mappings = state.mappings.clone();
                let exclusions = state.exclusions.clone();
                // Preserve live UI control configs (including disabled fns),
                // and only restore persisted values.
                for (name, value) in state.ui_controls.values() {
                    hub.ui_controls.set(&name, value);
                }
                hub.midi_controls = state.midi_controls.clone();
                hub.midi_controls.hrcc = self.hrcc;
                hub.midi_controls.set_port(self.midi_input_port.clone());
                hub.midi_overrides =
                    Arc::new(Mutex::new(state.midi_overrides.clone()));
                hub.midi_override_configs = state.midi_override_configs.clone();
                hub.midi_controls
                    .set_override_state(hub.midi_overrides.clone());
                hub.midi_controls
                    .set_override_configs(hub.midi_override_configs.clone());
                hub.osc_controls = state.osc_controls.clone();
                hub.snapshots = state.snapshots.clone();
                hub.midi_controls
                    .restart()
                    .inspect_err(|err| {
                        error!(
                            "Error in restore_sketch_state_from_disk: {}",
                            err
                        )
                    })
                    .ok();
                self.current_sketch_ui_state_mut().mappings = mappings;
                self.current_sketch_ui_state_mut().exclusions = exclusions;
                self.map_mode
                    .set_mappings(self.current_sketch_ui_state().mappings);
                self.alert_and_log("Controls restored", log::Level::Info);
            }
            Err(err) => {
                if err
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound)
                {
                    return;
                }
                self.alert_and_log(
                    format!("Failed to restore controls: {}", err),
                    log::Level::Error,
                );
            }
        }
    }

    /// Emits one-time shutdown events to peers.
    fn signal_shutdown(&mut self) {
        if self.shutdown_signaled {
            return;
        }

        self.shutdown_signaled = true;
        self.emit_event(RuntimeEvent::WebView(Box::new(web_view::Event::Quit)));
        self.emit_event(RuntimeEvent::Stopped);
    }

    /// Requests graceful exit of the event loop.
    fn shutdown(&mut self, event_loop: &ActiveEventLoop) {
        self.signal_shutdown();
        event_loop.exit();
    }
}

impl ApplicationHandler for XtalRuntime {
    // Winit lifecycle hook: initialize runtime resources once.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        if let Err(err) = self.init_runtime(event_loop) {
            error!("failed to initialize xtal runtime: {}", err);
            event_loop.exit();
            return;
        }

        frame_clock::set_fps(self.config.fps);
        self.apply_play_mode();
        // Always draw the first frame, even in Pause/Advance modes.
        self.request_render_now();
        self.emit_web_view_init();
        self.emit_web_view_load_sketch();
    }

    // Winit window event hook: route input, resize, redraw, and close events.
    //
    // Monitor preview window events are handled first because they belong to a
    // separate window id but share the same application handler.
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self
            .monitor_preview
            .as_ref()
            .is_some_and(|preview| preview.window_id() == window_id)
        {
            match event {
                WindowEvent::CloseRequested => {
                    self.set_monitor_preview_enabled(event_loop, false);
                }
                WindowEvent::Resized(new_size) => {
                    let Some(context) = self.context.as_ref() else {
                        return;
                    };
                    if let Some(preview) = self.monitor_preview.as_mut() {
                        preview.on_window_resized(
                            context.device.as_ref(),
                            new_size,
                        );
                        self.monitor_preview_size_hint =
                            Some(preview.window().inner_size());
                    }
                }
                WindowEvent::RedrawRequested => {}
                _ => {}
            }
            return;
        }

        if self.window_id != Some(window_id) {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                self.shutdown(event_loop);
            }
            WindowEvent::Destroyed => {
                self.shutdown(event_loop);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_main_window_shortcut(event_loop, &event);
            }
            WindowEvent::Resized(new_size) => self.resize(new_size),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(context) = self.context.as_mut() {
                    context.set_scale_factor(scale_factor);
                }
            }
            WindowEvent::RedrawRequested => {
                self.render(event_loop);
            }
            _ => {}
        }
    }

    // Winit idle hook: drain commands and schedule frames through the clock.
    //
    // Runtime commands are processed before frame scheduling so UI, MIDI, OSC,
    // and background callbacks can affect the next frame deterministically.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.process_commands(event_loop);
        let now = Instant::now();
        self.emit_average_fps_if_due(now);

        if self.render_requested {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                frame_clock::next_deadline(),
            ));
            return;
        }

        let tick = frame_clock::tick(now);

        if tick.should_render {
            self.render_requested = true;
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        } else {
            self.emit_event(RuntimeEvent::FrameSkipped);
        }

        event_loop.set_control_flow(ControlFlow::WaitUntil(
            frame_clock::next_deadline(),
        ));
    }

    // Winit final lifecycle hook.
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.signal_shutdown();
    }
}

/// Runs a registry-backed xtal app with the production web view bridge.
///
/// `initial_sketch` selects the first active sketch. The optional timing mode
/// override is parsed from the second process argument so launchers can choose
/// frame, OSC, MIDI, hybrid, or manual timing at runtime.
pub fn run_registry(
    registry: RuntimeRegistry,
    initial_sketch: Option<&str>,
) -> Result<(), String> {
    let timing_mode_override = parse_timing_mode_arg()?;
    let (command_tx, command_rx) = command_channel();
    let (event_tx, event_rx) = event_channel();

    let _bridge = WebViewBridge::launch(command_tx.clone(), event_rx)?;

    run_registry_with_channels(
        registry,
        initial_sketch,
        timing_mode_override,
        command_tx,
        command_rx,
        Some(event_tx),
    )
}

/// Runs the app with caller-provided command and notification channels.
///
/// This keeps the production launcher small while allowing tests or embedding
/// code to provide their own bridge around the same runtime loop.
fn run_registry_with_channels(
    registry: RuntimeRegistry,
    initial_sketch: Option<&str>,
    timing_mode_override: Option<TimingMode>,
    command_tx: RuntimeCommandSender,
    command_rx: RuntimeCommandReceiver,
    event_tx: Option<RuntimeEventSender>,
) -> Result<(), String> {
    logging::init_logger();

    let event_loop = EventLoop::new().map_err(|err| err.to_string())?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut runner = XtalRuntime::new(
        registry,
        initial_sketch,
        timing_mode_override,
        command_tx,
        command_rx,
        event_tx,
    )?;

    event_loop
        .run_app(&mut runner)
        .map_err(|err| err.to_string())
}

/// Parses the optional timing mode override from argv position 2.
fn parse_timing_mode_arg() -> Result<Option<TimingMode>, String> {
    let Some(value) = env::args().nth(2) else {
        return Ok(None);
    };

    value.parse().map(Some)
}

/// Chooses the startup sketch by requested name or registry fallback.
fn select_initial_sketch_name(
    registry: &RuntimeRegistry,
    initial_sketch: Option<&str>,
) -> Result<String, String> {
    if let Some(initial_sketch) = initial_sketch {
        if registry.get(initial_sketch).is_some() {
            return Ok(initial_sketch.to_string());
        }

        warn!(
            "requested initial sketch '{}' does not exist; falling back",
            initial_sketch
        );
    }

    registry
        .first_sketch_name()
        .map(ToOwned::to_owned)
        .ok_or_else(|| "runtime registry is empty".to_string())
}

/// Instantiates one registered sketch and returns its static config.
fn instantiate_sketch(
    registry: &RuntimeRegistry,
    name: &str,
) -> Result<(&'static SketchConfig, Box<dyn Sketch>), String> {
    let Some(entry) = registry.get(name) else {
        return Err(format!("sketch '{}' is not registered", name));
    };

    let config = entry.config;
    let sketch = (entry.factory)();

    Ok((config, sketch))
}

/// Chooses the preferred surface format for the main render target.
fn choose_surface_format(
    formats: &[wgpu::TextureFormat],
) -> Option<wgpu::TextureFormat> {
    formats
        .iter()
        .copied()
        .find(|f| *f == wgpu::TextureFormat::Bgra8UnormSrgb)
        .or_else(|| formats.first().copied())
}

/// Places a non-performance-mode window at the top-left of its monitor.
fn anchor_window_top_left(window: &Window) {
    let Some(monitor) = window.current_monitor() else {
        return;
    };

    let monitor_origin = monitor.position();
    let x = monitor_origin.x;
    let y = monitor_origin.y;

    window.set_outer_position(winit::dpi::PhysicalPosition::new(x, y));
}

/// Maps a GPU readback buffer and writes it as an RGBA PNG.
fn save_png_capture(
    device: &wgpu::Device,
    submission_index: wgpu::SubmissionIndex,
    capture: PendingPngCapture,
) -> Result<(), String> {
    let PendingPngCapture {
        path,
        buffer,
        width,
        height,
        padded_bytes_per_row,
        source_format,
    } = capture;

    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    let _ =
        device.poll(wgpu::PollType::WaitForSubmissionIndex(submission_index));
    let map_result = rx
        .recv()
        .map_err(|err| format!("map channel recv failed: {}", err))?;
    map_result.map_err(|err| format!("map failed: {:?}", err))?;

    let data = slice.get_mapped_range();
    // Recording/capture source formats are 8-bit RGBA/BGRA, so 4 bytes/pixel.
    let unpadded_bytes_per_row = (width * 4) as usize;
    let padded_bytes_per_row = padded_bytes_per_row as usize;
    let mut rgba = vec![0u8; unpadded_bytes_per_row * (height as usize)];

    for row in 0..(height as usize) {
        let src_start = row * padded_bytes_per_row;
        let src_end = src_start + unpadded_bytes_per_row;
        let dst_start = row * unpadded_bytes_per_row;
        let dst_end = dst_start + unpadded_bytes_per_row;
        rgba[dst_start..dst_end].copy_from_slice(&data[src_start..src_end]);
    }

    drop(data);
    buffer.unmap();

    if matches!(
        source_format,
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    ) {
        for px in rgba.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
    }

    let file = fs::File::create(&path).map_err(|err| {
        format!("failed to create '{}': {}", path.display(), err)
    })?;
    let mut writer = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(&mut writer, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    encoder.set_filter(png::Filter::Sub);
    let mut png_writer = encoder
        .write_header()
        .map_err(|err| format!("png header failed: {}", err))?;
    png_writer
        .write_image_data(&rgba)
        .map_err(|err| format!("png write failed: {}", err))?;
    drop(png_writer);
    writer
        .flush()
        .map_err(|err| format!("png flush failed: {}", err))?;

    Ok(())
}

/// Saves a completed PNG capture on a worker thread and alerts the UI.
fn queue_png_capture_save(
    device: Arc<wgpu::Device>,
    submission_index: wgpu::SubmissionIndex,
    capture: PendingPngCapture,
    event_tx: Option<RuntimeEventSender>,
) {
    std::thread::spawn(move || {
        let path = capture.path.clone();
        match save_png_capture(device.as_ref(), submission_index, capture) {
            Ok(()) => {
                let message = format!("Image saved to {:?}", path);
                info!("{}", message);
                if let Some(tx) = event_tx.as_ref() {
                    let _ = tx.send(RuntimeEvent::WebView(Box::new(
                        web_view::Event::Alert(message),
                    )));
                }
            }
            Err(err) => {
                let message = format!("Failed to save image capture: {}", err);
                error!("{}", message);
                if let Some(tx) = event_tx.as_ref() {
                    let _ = tx.send(RuntimeEvent::WebView(Box::new(
                        web_view::Event::Alert(message),
                    )));
                }
            }
        }
    });
}

/// Derives the default storage directory from a sketch control script path.
fn default_user_data_dir_for_sketch(sketch: &dyn Sketch) -> Option<String> {
    let control_script = sketch.control_script()?;
    let crate_root = find_crate_root(control_script.as_path())?;
    Some(crate_root.join("storage").display().to_string())
}

/// Finds the containing crate root for a file path.
fn find_crate_root(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|ancestor| ancestor.join("Cargo.toml").exists())
        .map(Path::to_path_buf)
}

/// Converts top-row digit keys into snapshot slot identifiers.
fn digit_from_key_code(code: KeyCode) -> Option<char> {
    match code {
        KeyCode::Digit0 => Some('0'),
        KeyCode::Digit1 => Some('1'),
        KeyCode::Digit2 => Some('2'),
        KeyCode::Digit3 => Some('3'),
        KeyCode::Digit4 => Some('4'),
        KeyCode::Digit5 => Some('5'),
        KeyCode::Digit6 => Some('6'),
        KeyCode::Digit7 => Some('7'),
        KeyCode::Digit8 => Some('8'),
        KeyCode::Digit9 => Some('9'),
        _ => None,
    }
}
