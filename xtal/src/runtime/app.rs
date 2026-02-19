use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::Once;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

use log::{debug, error, info, trace, warn};
use nannou_osc as osc;
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
use super::recording::{self, RecordingState};
use super::registry::RuntimeRegistry;
use super::serialization::{GlobalSettings, TransitorySketchState};
use super::storage;
use super::tap_tempo::TapTempo;
use super::web_view;
use super::web_view_bridge::WebViewBridge;
use crate::context::Context;
use crate::control::map_mode::MapMode;
use crate::control::{ControlCollection, ControlHub, ControlValue};
use crate::frame::Frame;
use crate::framework::audio::list_audio_devices;
use crate::framework::osc_receiver::SHARED_OSC_RECEIVER;
use crate::framework::util::{HashMap, uuid_5};
use crate::framework::{frame_controller, logging, midi};
use crate::gpu::CompiledGraph;
use crate::graph::GraphBuilder;
use crate::motion::{Bpm, Timing};
use crate::sketch::{PlayMode, Sketch, SketchConfig, TimingMode};
use crate::uniforms::UniformBanks;

const MIDI_START: u8 = 0xFA;
const MIDI_CONTINUE: u8 = 0xFB;
const MIDI_STOP: u8 = 0xFC;
const MIDI_CLOCK: u8 = 0xF8;
const MIDI_SONG_POSITION: u8 = 0xF2;
const MIDI_MTC_QUARTER_FRAME: u8 = 0xF1;
const DEFAULT_OSC_PORT: u16 = 2346;
const PULSES_PER_QUARTER_NOTE: u32 = 24;
const TICKS_PER_QUARTER_NOTE: u32 = 960;
const HYBRID_SYNC_THRESHOLD_BEATS: f32 = 0.5;

static OSC_TRANSPORT_CALLBACK_REGISTER: Once = Once::new();

#[derive(Clone, Default)]
struct SketchUiState {
    mappings: web_view::Mappings,
    exclusions: web_view::Exclusions,
}

struct PendingPngCapture {
    path: PathBuf,
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
    source_format: wgpu::TextureFormat,
}

struct XtalRuntime {
    registry: RuntimeRegistry,
    active_sketch_name: String,
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
    window: Option<Arc<Window>>,
    window_id: Option<WindowId>,
    windowed_size_before_fullscreen: Option<winit::dpi::PhysicalSize<u32>>,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    context: Option<Context>,
    uniforms: Option<UniformBanks>,
    graph: Option<CompiledGraph>,
    control_hub: Option<ControlHub<Timing>>,
    bpm: Bpm,
    tap_tempo: TapTempo,
    tap_tempo_enabled: bool,
    perf_mode: bool,
    transition_time: f32,
    mappings_enabled: bool,
    map_mode: MapMode,
    sketch_ui_state: HashMap<String, SketchUiState>,
    recording_state: RecordingState,
    session_id: String,
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
    images_dir: String,
    user_data_dir: String,
    videos_dir: String,
    last_average_fps_emit: Instant,
    shutdown_signaled: bool,
    pending_png_capture_path: Option<PathBuf>,
    modifiers: ModifiersState,
    midi_clock_count: Arc<AtomicU32>,
    midi_song_position_ticks: Arc<AtomicU32>,
    osc_transport_playing: Arc<AtomicBool>,
    osc_transport_bars: Arc<AtomicU32>,
    osc_transport_beats: Arc<AtomicU32>,
    osc_transport_ticks: Arc<AtomicU32>,
    follow_song_position: Arc<AtomicBool>,
    hybrid_mtc_sync_enabled: Arc<AtomicBool>,
    mtc_hours: Arc<AtomicU32>,
    mtc_minutes: Arc<AtomicU32>,
    mtc_seconds: Arc<AtomicU32>,
    mtc_frames: Arc<AtomicU32>,
}

impl XtalRuntime {
    // Builds runtime state from registry + persisted settings before window/GPU
    // init.
    fn new(
        registry: RuntimeRegistry,
        initial_sketch: Option<&str>,
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

        let mut sketch_ui_state = HashMap::default();
        sketch_ui_state.insert(active_name.clone(), SketchUiState::default());

        let mut runtime = Self {
            registry,
            active_sketch_name: active_name,
            config,
            sketch,
            render_requested: false,
            command_tx,
            command_rx,
            event_tx,
            window: None,
            window_id: None,
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
            last_average_fps_emit: Instant::now(),
            shutdown_signaled: false,
            pending_png_capture_path: None,
            modifiers: ModifiersState::default(),
            midi_clock_count: Arc::new(AtomicU32::new(0)),
            midi_song_position_ticks: Arc::new(AtomicU32::new(0)),
            osc_transport_playing: Arc::new(AtomicBool::new(false)),
            osc_transport_bars: Arc::new(AtomicU32::new(0)),
            osc_transport_beats: Arc::new(AtomicU32::new(0)),
            osc_transport_ticks: Arc::new(AtomicU32::new(0.0f32.to_bits())),
            follow_song_position: Arc::new(AtomicBool::new(true)),
            hybrid_mtc_sync_enabled: Arc::new(AtomicBool::new(false)),
            mtc_hours: Arc::new(AtomicU32::new(0)),
            mtc_minutes: Arc::new(AtomicU32::new(0)),
            mtc_seconds: Arc::new(AtomicU32::new(0)),
            mtc_frames: Arc::new(AtomicU32::new(0)),
        };

        let audio_device_updated = runtime.normalize_audio_device_selection();
        let midi_ports_updated = runtime.normalize_midi_port_selections();
        let osc_port_updated = runtime.normalize_osc_port_selection();
        runtime.update_timing_mode_flags();
        runtime.register_osc_transport_listener();
        runtime.start_osc_receiver();
        runtime.start_midi_clock_listener();
        runtime.connect_midi_out();
        runtime.log_midi_startup_state();
        if audio_device_updated || midi_ports_updated || osc_port_updated {
            runtime.save_global_state();
        }

        Ok(runtime)
    }

    // Single command/event dispatcher for runtime behavior changes.
    fn on_runtime_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: RuntimeEvent,
    ) -> bool {
        match event {
            RuntimeEvent::AdvanceSingleFrame => {
                frame_controller::advance_single_frame();
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
                let file_path = PathBuf::from(&self.images_dir).join(filename);
                self.pending_png_capture_path = Some(file_path);
                self.render_requested = true;

                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
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
                self.start_midi_clock_listener();
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
                self.map_mode.currently_mapping = None;
                let mappings = self.map_mode.mappings();
                let mut missing_slider_ranges = Vec::new();

                {
                    let Some(hub) = self.control_hub.as_mut() else {
                        return false;
                    };

                    for (name, _) in hub.midi_controls.configs() {
                        if MapMode::is_proxy_name(&name)
                            && !hub
                                .ui_controls
                                .has(&MapMode::unproxied_name(&name).unwrap())
                        {
                            debug!("Removing orphaned proxy: {}", name);
                            hub.midi_controls.remove(&name);
                        }
                    }

                    for (name, (ch, cc)) in mappings {
                        let proxy_name = &MapMode::proxy_name(&name);

                        if let Some(config) =
                            hub.midi_controls.config(proxy_name)
                        {
                            if config.channel == ch as u8
                                && config.cc == cc as u8
                            {
                                continue;
                            }
                        }

                        let slider_range =
                            match hub.ui_controls.slider_range(&name) {
                                Some(range) => range,
                                None => {
                                    missing_slider_ranges.push(name.clone());
                                    continue;
                                }
                            };

                        hub.midi_controls.add(
                            proxy_name,
                            crate::control::MidiControlConfig::new(
                                (ch as u8, cc as u8),
                                slider_range,
                                0.0,
                            ),
                        );
                    }

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
                    hub.midi_controls.remove(&MapMode::proxy_name(&name));
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
                info!("Received MIDI Start/Continue. Resetting frame count.");

                frame_controller::reset_frame_count();

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
                frame_controller::set_paused(paused);
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
                return true;
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
                    hub.midi_controls.remove(&MapMode::proxy_name(&name));
                }

                let mappings = self.map_mode.mappings();
                self.current_sketch_ui_state_mut().mappings = mappings.clone();
                self.emit_web_view_event(web_view::Event::Mappings(mappings));
            }
            RuntimeEvent::Reset => {
                frame_controller::reset();
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
                    hub.midi_proxies_enabled = enabled;
                }
                self.save_global_state();
            }
            RuntimeEvent::SetPerfMode(perf_mode) => {
                self.set_perf_mode(perf_mode);
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
                        "Failed to start recording: runtime context unavailable",
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
                let Some(source_format) = graph.recording_source_format()
                else {
                    self.alert_and_log(
                        "Failed to start recording: graph presents directly to surface; recording source unavailable",
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

                if should_emit_updated_controls {
                    if let Some(hub) = self.control_hub.as_ref() {
                        self.emit_web_view_event(
                            web_view::Event::UpdatedControls(
                                web_view::controls_from_hub(hub),
                            ),
                        );
                    }
                }
            }
            RuntimeEvent::FrameSkipped
            | RuntimeEvent::SketchSwitched(_)
            | RuntimeEvent::Stopped
            | RuntimeEvent::WebView(_) => {}
        }

        false
    }

    // Drains inbound command channel and routes events through the central
    // dispatcher.
    fn process_commands(&mut self, event_loop: &ActiveEventLoop) {
        while let Ok(event) = self.command_rx.try_recv() {
            if self.on_runtime_event(event_loop, event) {
                return;
            }
        }
    }

    // Throttled FPS broadcast to UI (once per second).
    fn emit_average_fps_if_due(&mut self, now: Instant) {
        if now.duration_since(self.last_average_fps_emit)
            < Duration::from_secs(1)
        {
            return;
        }

        self.last_average_fps_emit = now;
        self.emit_web_view_event(web_view::Event::AverageFps(
            frame_controller::average_fps(),
        ));
    }

    // Main render/update pipeline.
    //
    // Order matters:
    // 1) Update sketch + hub, write uniforms.
    // 2) Acquire surface frame, run sketch view + graph execution.
    // 3) Encode recording/capture readback copies before submit.
    // 4) Submit once, then run post-submit host-side work.
    fn render(&mut self, event_loop: &ActiveEventLoop) {
        if !self.render_requested {
            return;
        }

        self.render_requested = false;
        let external_beats_for_frame = self.current_external_beats_for_mode();

        let (
            pending_png_capture,
            pending_png_capture_error,
            capture_device,
            capture_submission_index,
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

            // 2) Let sketch mutate runtime state before uniform upload.
            self.sketch.update(context);

            // 3) Runtime-owned uniforms: resolution + beat source + hub vars.
            let [w, h] = context.resolution();
            uniforms.set_resolution(w, h);

            if let Some(hub) = self.control_hub.as_mut() {
                if let Some(beats) = external_beats_for_frame {
                    hub.animation.timing.set_external_beats(beats);
                }
                hub.update();

                for (id, value) in hub.var_values() {
                    if let Err(err) = uniforms.set(&id, value) {
                        warn!(
                            "ignoring control var '{}' for sketch '{}': {}",
                            id, self.config.name, err
                        );
                    }
                }

                uniforms.set_beats(hub.beats());
            } else {
                uniforms.set_beats(context.elapsed_seconds());
            }

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

            if let Err(err) = graph.execute(
                context.device.as_ref(),
                &mut frame,
                uniforms,
                context.resolution_u32(),
            ) {
                error!("graph execution error: {}", err);
                event_loop.exit();
                return;
            }

            // 6) Recording readback copy is encoded pre-submit.
            if self.recording_state.is_recording {
                if let Some(recorder) =
                    self.recording_state.frame_recorder.as_mut()
                {
                    if let Some(source_texture) =
                        graph.recording_source_texture()
                    {
                        let encoder = frame.encoder();
                        let _ = recorder
                            .capture_surface_frame(encoder, source_texture);
                    }
                }
            }

            // 7) Optional still-image capture readback copy is also pre-submit.
            let mut pending_png_capture_error = None;
            let pending_png_capture = if let Some(path) =
                self.pending_png_capture_path.take()
            {
                let source_texture = graph.recording_source_texture();
                let source_format = graph.recording_source_format();
                match (source_texture, source_format) {
                    (Some(source_texture), Some(source_format)) => {
                        let width = source_texture.size().width.max(1);
                        let height = source_texture.size().height.max(1);
                        let bytes_per_pixel = 4u32;
                        let unpadded_bytes_per_row = width * bytes_per_pixel;
                        let padded_bytes_per_row = unpadded_bytes_per_row
                            + compute_row_padding(unpadded_bytes_per_row);
                        let buffer_size =
                            (padded_bytes_per_row as u64) * (height as u64);
                        let buffer = context.device.create_buffer(
                            &wgpu::BufferDescriptor {
                                label: Some("xtal2-capture-readback"),
                                size: buffer_size,
                                usage: wgpu::BufferUsages::COPY_DST
                                    | wgpu::BufferUsages::MAP_READ,
                                mapped_at_creation: false,
                            },
                        );

                        frame.encoder().copy_texture_to_buffer(
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
                    }
                    _ => {
                        pending_png_capture_error = Some("Failed to capture frame: graph presents directly to surface; no capture source texture".to_string());
                        None
                    }
                }
            } else {
                None
            };

            // 8) Submit all encoded GPU work once.
            let submission_index = frame.submit();

            if self.recording_state.is_recording {
                if let Some(recorder) =
                    self.recording_state.frame_recorder.as_mut()
                {
                    recorder.on_submitted();
                }
            }

            // 9) Advance local frame-time state after successful submit.
            context.next_frame();

            (
                pending_png_capture,
                pending_png_capture_error,
                context.device.clone(),
                submission_index,
            )
        };

        // 10) Post-submit host-side effects/events.
        if let Some(message) = pending_png_capture_error {
            self.alert_and_log(message, log::Level::Error);
        }

        if self.recording_state.is_encoding {
            if let Some(outcome) =
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

    // Main-window keyboard handling mirroring UI shortcut semantics.
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
                if frame_controller::paused() {
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
                let paused = !frame_controller::paused();
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
            KeyCode::Space => {
                if self.tap_tempo_enabled {
                    return self
                        .on_runtime_event(event_loop, RuntimeEvent::Tap);
                }
            }
            _ => {}
        }

        false
    }

    // Creates window/surface/device/context then compiles sketch graph state.
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
                label: Some("xtal2-device"),
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
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
        self.surface = Some(surface);
        self.surface_config = Some(surface_config);
        self.context = Some(context);

        self.rebuild_graph_state()?;

        Ok(())
    }

    // Rebuilds graph + uniforms + control hub for startup/switch/reload.
    fn rebuild_graph_state(&mut self) -> Result<(), String> {
        let mut graph_builder = GraphBuilder::new();
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

    // Builds hub from sketch control script and wires callback bridges.
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

        let timing = match self.sketch.timing_mode() {
            TimingMode::Frame => Timing::frame(self.bpm.clone()),
            TimingMode::Osc => Timing::osc(self.bpm.clone()),
            TimingMode::Midi => Timing::midi(self.bpm.clone()),
            TimingMode::Hybrid => Timing::hybrid(self.bpm.clone()),
            TimingMode::Manual => Timing::manual(self.bpm.clone()),
        };

        let mut hub = ControlHub::from_path(path, timing);
        hub.set_transition_time(self.transition_time);
        hub.midi_proxies_enabled = self.mappings_enabled;
        hub.midi_controls.hrcc = self.hrcc;
        hub.midi_controls.set_port(self.midi_input_port.clone());
        hub.audio_controls
            .set_device_name(self.audio_device.clone());
        info!(
            "Configuring sketch MIDI controls: input_port='{}', hrcc={}, mappings_enabled={}",
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

    fn start_midi_clock_listener(&self) {
        if self.midi_clock_port.is_empty() {
            info!("Skipping MIDI clock listener setup; no MIDI clock port.");
            return;
        }

        info!(
            "Starting MIDI clock listener on port '{}'",
            self.midi_clock_port
        );

        let command_tx = self.command_tx.clone();
        let clock_count = self.midi_clock_count.clone();
        let song_position_ticks = self.midi_song_position_ticks.clone();
        let follow_song_position = self.follow_song_position.clone();
        let hybrid_mtc_sync_enabled = self.hybrid_mtc_sync_enabled.clone();
        let mtc_hours = self.mtc_hours.clone();
        let mtc_minutes = self.mtc_minutes.clone();
        let mtc_seconds = self.mtc_seconds.clone();
        let mtc_frames = self.mtc_frames.clone();
        let bpm = self.bpm.clone();
        let midi_handler_result = midi::on_message(
            midi::ConnectionType::Clock,
            &self.midi_clock_port,
            move |_stamp, message| {
                if message.is_empty() {
                    return;
                }

                match message[0] {
                    MIDI_CLOCK => {
                        clock_count.fetch_add(1, Ordering::SeqCst);
                    }
                    MIDI_SONG_POSITION => {
                        if !follow_song_position.load(Ordering::Acquire) {
                            return;
                        }
                        if message.len() < 3 {
                            warn!(
                                "Received malformed SONG_POSITION message: {:?}",
                                message
                            );
                            return;
                        }
                        let lsb = message[1] as u32;
                        let msb = message[2] as u32;
                        let position = (msb << 7) | lsb;
                        let tick_pos = position * (TICKS_PER_QUARTER_NOTE / 4);
                        song_position_ticks.store(tick_pos, Ordering::SeqCst);
                        clock_count.store(0, Ordering::SeqCst);
                    }
                    MIDI_START => {
                        clock_count.store(0, Ordering::SeqCst);
                        let _ = command_tx.send(RuntimeEvent::MidiStart);
                    }
                    MIDI_CONTINUE => {
                        let _ = command_tx.send(RuntimeEvent::MidiContinue);
                    }
                    MIDI_STOP => {
                        let _ = command_tx.send(RuntimeEvent::MidiStop);
                    }
                    MIDI_MTC_QUARTER_FRAME => {
                        if message.len() < 2
                            || !hybrid_mtc_sync_enabled.load(Ordering::Acquire)
                        {
                            return;
                        }

                        let data = message[1];
                        let piece_index = (data >> 4) & 0x7;
                        let value = data & 0xF;

                        match piece_index {
                            0 => {
                                let current =
                                    mtc_frames.load(Ordering::Relaxed);
                                mtc_frames.store(
                                    (current & 0xF0) | value as u32,
                                    Ordering::Relaxed,
                                );
                            }
                            1 => {
                                let current =
                                    mtc_frames.load(Ordering::Relaxed);
                                mtc_frames.store(
                                    (current & 0x0F) | ((value as u32) << 4),
                                    Ordering::Relaxed,
                                );
                            }
                            2 => {
                                let current =
                                    mtc_seconds.load(Ordering::Relaxed);
                                mtc_seconds.store(
                                    (current & 0xF0) | value as u32,
                                    Ordering::Relaxed,
                                );
                            }
                            3 => {
                                let current =
                                    mtc_seconds.load(Ordering::Relaxed);
                                mtc_seconds.store(
                                    (current & 0x0F) | ((value as u32) << 4),
                                    Ordering::Relaxed,
                                );
                            }
                            4 => {
                                let current =
                                    mtc_minutes.load(Ordering::Relaxed);
                                mtc_minutes.store(
                                    (current & 0xF0) | value as u32,
                                    Ordering::Relaxed,
                                );
                            }
                            5 => {
                                let current =
                                    mtc_minutes.load(Ordering::Relaxed);
                                mtc_minutes.store(
                                    (current & 0x0F) | ((value as u32) << 4),
                                    Ordering::Relaxed,
                                );
                            }
                            6 => {
                                let current = mtc_hours.load(Ordering::Relaxed);
                                mtc_hours.store(
                                    (current & 0xF0) | value as u32,
                                    Ordering::Relaxed,
                                );
                            }
                            7 => {
                                let hours_lsb =
                                    mtc_hours.load(Ordering::Relaxed) & 0x0F;
                                let hours_msb = value & 0x3;
                                let rate_code = (value >> 2) & 0x3;
                                let fps = match rate_code {
                                    0 => 24.0,
                                    1 => 25.0,
                                    2 => 29.97,
                                    3 => 30.0,
                                    _ => return,
                                };

                                let full_hours =
                                    ((hours_msb << 4) | hours_lsb as u8) & 0x1F;
                                mtc_hours.store(
                                    full_hours as u32,
                                    Ordering::Relaxed,
                                );

                                let mtc_time_seconds = mtc_hours
                                    .load(Ordering::Relaxed)
                                    as f32
                                    * 3600.0
                                    + mtc_minutes.load(Ordering::Relaxed)
                                        as f32
                                        * 60.0
                                    + mtc_seconds.load(Ordering::Relaxed)
                                        as f32
                                    + mtc_frames.load(Ordering::Relaxed) as f32
                                        / fps;
                                let mtc_beats =
                                    mtc_time_seconds * (bpm.get() / 60.0);
                                let midi_beats =
                                    clock_count.load(Ordering::Relaxed) as f32
                                        / PULSES_PER_QUARTER_NOTE as f32;
                                let beat_difference =
                                    (mtc_beats - midi_beats).abs();
                                if beat_difference > HYBRID_SYNC_THRESHOLD_BEATS
                                {
                                    let clock = (mtc_beats
                                        * PULSES_PER_QUARTER_NOTE as f32)
                                        as u32;
                                    clock_count.store(clock, Ordering::SeqCst);
                                    trace!(
                                        "Hybrid timing resync from MTC: mtc_beats={}, midi_beats={}, new_clock={}",
                                        mtc_beats, midi_beats, clock
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            },
        );

        if let Err(err) = midi_handler_result {
            warn!(
                "Failed to initialize {:?} MIDI connection. Error: {}",
                midi::ConnectionType::Clock,
                err
            );
        }
    }

    fn register_osc_transport_listener(&self) {
        let playing = self.osc_transport_playing.clone();
        let bars = self.osc_transport_bars.clone();
        let beats = self.osc_transport_beats.clone();
        let ticks = self.osc_transport_ticks.clone();

        OSC_TRANSPORT_CALLBACK_REGISTER.call_once(move || {
            SHARED_OSC_RECEIVER.register_callback("/transport", move |msg| {
                if msg.args.len() < 4 {
                    return;
                }

                if let (
                    osc::Type::Int(a),
                    osc::Type::Int(b),
                    osc::Type::Int(c),
                    osc::Type::Float(d),
                ) = (&msg.args[0], &msg.args[1], &msg.args[2], &msg.args[3])
                {
                    playing.store(*a != 0, Ordering::Release);
                    bars.store(
                        (*b).saturating_sub(1) as u32,
                        Ordering::Release,
                    );
                    beats.store(
                        (*c).saturating_sub(1) as u32,
                        Ordering::Release,
                    );
                    ticks.store(d.to_bits(), Ordering::Release);
                }
            });
        });
    }

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

    fn log_midi_startup_state(&self) {
        info!(
            "MIDI/OSC startup state: input_port='{}', output_port='{}', clock_port='{}', osc_port={}, hrcc={}, mappings_enabled={}",
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

    fn normalize_osc_port_selection(&mut self) -> bool {
        if self.osc_port == 0 {
            self.osc_port = DEFAULT_OSC_PORT;
            info!("Resolved OSC port from 0 to {}", self.osc_port);
            return true;
        }
        false
    }

    fn start_osc_receiver(&self) {
        if let Err(err) = SHARED_OSC_RECEIVER.restart(self.osc_port) {
            error!("Failed to restart OSC receiver: {}", err);
        }
    }

    fn restart_osc_receiver(&self) {
        if let Err(err) = SHARED_OSC_RECEIVER.restart(self.osc_port) {
            error!("Failed to restart OSC receiver: {}", err);
        }
    }

    fn current_midi_transport_beats(&self) -> f32 {
        let clock_offset = self.midi_clock_count.load(Ordering::Relaxed) as f32
            / PULSES_PER_QUARTER_NOTE as f32;
        let ticks = self.midi_song_position_ticks.load(Ordering::Relaxed);
        let beat_base = ticks as f32 / TICKS_PER_QUARTER_NOTE as f32;
        beat_base + clock_offset
    }

    fn current_hybrid_transport_beats(&self) -> f32 {
        self.midi_clock_count.load(Ordering::Relaxed) as f32
            / PULSES_PER_QUARTER_NOTE as f32
    }

    fn current_osc_transport_beats(&self) -> f32 {
        if !self.osc_transport_playing.load(Ordering::Acquire) {
            return 0.0;
        }

        let bars = self.osc_transport_bars.load(Ordering::Acquire) as f32;
        let beats = self.osc_transport_beats.load(Ordering::Acquire) as f32;
        let ticks =
            f32::from_bits(self.osc_transport_ticks.load(Ordering::Acquire));
        (bars * 4.0) + beats + ticks
    }

    fn current_external_beats_for_mode(&self) -> Option<f32> {
        match self.sketch.timing_mode() {
            TimingMode::Osc => Some(self.current_osc_transport_beats()),
            TimingMode::Midi => Some(self.current_midi_transport_beats()),
            TimingMode::Hybrid => Some(self.current_hybrid_transport_beats()),
            TimingMode::Manual | TimingMode::Frame => None,
        }
    }

    fn update_timing_mode_flags(&self) {
        let mode = self.sketch.timing_mode();
        self.follow_song_position
            .store(matches!(mode, TimingMode::Midi), Ordering::Release);
        self.hybrid_mtc_sync_enabled
            .store(matches!(mode, TimingMode::Hybrid), Ordering::Release);
    }

    // Applies resize to surface config and runtime context resolution.
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
    }

    // Internal runtime event emitter.
    fn emit_event(&self, event: RuntimeEvent) {
        let Some(event_tx) = self.event_tx.as_ref() else {
            return;
        };

        if let Err(err) = event_tx.send(event) {
            warn!("failed to emit runtime event: {}", err);
        }
    }

    // Convenience wrapper for runtime -> webview events.
    fn emit_web_view_event(&self, event: web_view::Event) {
        self.emit_event(RuntimeEvent::WebView(Box::new(event)));
    }

    // Returns cached per-sketch UI state.
    fn current_sketch_ui_state(&self) -> SketchUiState {
        self.sketch_ui_state
            .get(&self.active_sketch_name)
            .cloned()
            .unwrap_or_default()
    }

    // Returns mutable per-sketch UI state, creating default if needed.
    fn current_sketch_ui_state_mut(&mut self) -> &mut SketchUiState {
        self.sketch_ui_state
            .entry(self.active_sketch_name.clone())
            .or_default()
    }

    // Derives UI mapping payload from hub MIDI proxy configs.
    fn mappings_from_hub(&self) -> web_view::Mappings {
        let Some(hub) = self.control_hub.as_ref() else {
            return HashMap::default();
        };

        let mut mappings = HashMap::default();

        for (name, config) in hub.midi_controls.configs() {
            let Some(unproxied) = MapMode::unproxied_name(&name) else {
                continue;
            };

            mappings.insert(
                unproxied,
                (config.channel as usize, config.cc as usize),
            );
        }

        mappings
    }

    // Sends one-time UI bootstrap payload.
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
            osc_port: self.osc_port,
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

    // Sends active sketch payload (controls/snapshots/mappings/toggles).
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

        let mut sketch_state = self.current_sketch_ui_state();
        if sketch_state.mappings.is_empty() {
            sketch_state.mappings = self.mappings_from_hub();
            self.current_sketch_ui_state_mut().mappings =
                sketch_state.mappings.clone();
        }
        self.map_mode.set_mappings(sketch_state.mappings.clone());

        let event = web_view::Event::LoadSketch {
            bpm: self.bpm.get(),
            bypassed,
            controls,
            display_name: self.config.display_name.to_string(),
            fps: self.config.fps,
            mappings: sketch_state.mappings,
            paused: frame_controller::paused(),
            perf_mode: self.perf_mode,
            sketch_name: self.active_sketch_name.clone(),
            sketch_width: self.config.w as i32,
            sketch_height: self.config.h as i32,
            snapshot_slots,
            snapshot_sequence_enabled,
            tap_tempo_enabled: self.tap_tempo_enabled,
            exclusions: sketch_state.exclusions,
        };

        self.emit_web_view_event(event);
    }

    // Applies one UI control mutation into the hub and requests redraw.
    fn apply_control_update(&mut self, name: String, value: ControlValue) {
        let Some(hub) = self.control_hub.as_mut() else {
            warn!(
                "ignoring control update for '{}' because no control hub is active",
                name
            );
            return;
        };

        hub.ui_controls.set(&name, value);

        if self.config.play_mode == PlayMode::Advance
            && frame_controller::paused()
        {
            frame_controller::advance_single_frame();
            self.request_render_now();
        }
    }

    // Swaps sketch instance/config, rebuilds runtime graph state, updates UI.
    fn switch_sketch(&mut self, name: &str) -> Result<(), String> {
        self.map_mode.stop();

        let (config, sketch) = instantiate_sketch(&self.registry, name)?;

        self.active_sketch_name = name.to_string();
        self.config = config;
        self.sketch = sketch;
        self.update_timing_mode_flags();
        self.bpm.set(self.config.bpm);
        self.tap_tempo = TapTempo::new(self.config.bpm);
        frame_controller::set_fps(self.config.fps);
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

        if !self.perf_mode {
            self.resize(winit::dpi::PhysicalSize::new(
                self.config.w.max(1),
                self.config.h.max(1),
            ));
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

    fn request_render_now(&mut self) {
        self.render_requested = true;
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn apply_play_mode(&self) {
        let paused = match self.config.play_mode {
            PlayMode::Loop => false,
            PlayMode::Pause | PlayMode::Advance => true,
        };
        frame_controller::set_paused(paused);
    }

    // Toggles performance-mode window policy.
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

        if !self.perf_mode {
            self.resize(winit::dpi::PhysicalSize::new(
                self.config.w.max(1),
                self.config.h.max(1),
            ));
        }
    }

    // Sends a UI alert message.
    fn alert(&self, message: impl Into<String>) {
        self.emit_web_view_event(web_view::Event::Alert(message.into()));
    }

    // Sends UI alert and emits log entry with matching level.
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

    // Resolves requested OS directory kind to an absolute path.
    fn os_dir_path(&self, kind: &web_view::OsDir) -> PathBuf {
        match kind {
            web_view::OsDir::Cache => storage::cache_dir()
                .unwrap_or_else(|| env::temp_dir().join("Xtal")),
            web_view::OsDir::Config => PathBuf::from(&self.user_data_dir),
        }
    }

    // Updates cached randomize/save exclusions for active sketch.
    fn set_exclusions(&mut self, exclusions: web_view::Exclusions) {
        self.current_sketch_ui_state_mut().exclusions = exclusions;
    }

    // Persists global runtime settings.
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

    // Loads per-sketch controls/snapshots/mappings/exclusions into runtime + hub.
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
                hub.ui_controls = state.ui_controls.clone();
                hub.midi_controls = state.midi_controls.clone();
                hub.midi_controls.hrcc = self.hrcc;
                hub.midi_controls.set_port(self.midi_input_port.clone());
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

    // Emits one-time shutdown events to peers.
    fn signal_shutdown(&mut self) {
        if self.shutdown_signaled {
            return;
        }

        self.shutdown_signaled = true;
        self.emit_event(RuntimeEvent::WebView(Box::new(web_view::Event::Quit)));
        self.emit_event(RuntimeEvent::Stopped);
    }

    // Requests graceful exit of the event loop.
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

        frame_controller::set_fps(self.config.fps);
        self.apply_play_mode();
        // Always draw the first frame, even in Pause/Advance modes.
        self.request_render_now();
        self.emit_web_view_init();
        self.emit_web_view_load_sketch();
    }

    // Main window event router for input, resize, redraw, and close.
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
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

    // Tick hook: drain commands and schedule frames via frame controller.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.process_commands(event_loop);
        let now = Instant::now();
        self.emit_average_fps_if_due(now);

        if self.render_requested {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                frame_controller::next_deadline(),
            ));
            return;
        }

        let tick = frame_controller::tick(now);

        if tick.should_render {
            self.render_requested = true;
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        } else {
            self.emit_event(RuntimeEvent::FrameSkipped);
        }

        event_loop.set_control_flow(ControlFlow::WaitUntil(
            frame_controller::next_deadline(),
        ));
    }

    // Final lifecycle hook.
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.signal_shutdown();
    }
}

pub fn run_registry(
    registry: RuntimeRegistry,
    initial_sketch: Option<&str>,
) -> Result<(), String> {
    let (command_tx, command_rx) = command_channel();
    let (event_tx, event_rx) = event_channel();

    let _bridge = WebViewBridge::launch(command_tx.clone(), event_rx)?;

    run_registry_with_channels(
        registry,
        initial_sketch,
        command_tx,
        command_rx,
        Some(event_tx),
    )
}

fn run_registry_with_channels(
    registry: RuntimeRegistry,
    initial_sketch: Option<&str>,
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
        command_tx,
        command_rx,
        event_tx,
    )?;

    event_loop
        .run_app(&mut runner)
        .map_err(|err| err.to_string())
}

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

fn choose_surface_format(
    formats: &[wgpu::TextureFormat],
) -> Option<wgpu::TextureFormat> {
    formats
        .iter()
        .copied()
        .find(|f| *f == wgpu::TextureFormat::Bgra8UnormSrgb)
        .or_else(|| formats.first().copied())
}

fn anchor_window_top_left(window: &Window) {
    let Some(monitor) = window.current_monitor() else {
        return;
    };

    let monitor_origin = monitor.position();
    let x = monitor_origin.x;
    let y = monitor_origin.y;

    window.set_outer_position(winit::dpi::PhysicalPosition::new(x, y));
}

fn compute_row_padding(unpadded_bytes_per_row: u32) -> u32 {
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let rem = unpadded_bytes_per_row % align;
    if rem == 0 { 0 } else { align - rem }
}

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
                    let _ = tx.send(RuntimeEvent::WebView(
                        Box::new(web_view::Event::Alert(message)),
                    ));
                }
            }
            Err(err) => {
                let message = format!("Failed to save image capture: {}", err);
                error!("{}", message);
                if let Some(tx) = event_tx.as_ref() {
                    let _ = tx.send(RuntimeEvent::WebView(
                        Box::new(web_view::Event::Alert(message)),
                    ));
                }
            }
        }
    });
}

fn default_user_data_dir_for_sketch(sketch: &dyn Sketch) -> Option<String> {
    let control_script = sketch.control_script()?;
    let crate_root = find_crate_root(control_script.as_path())?;
    Some(crate_root.join("storage").display().to_string())
}

fn find_crate_root(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|ancestor| ancestor.join("Cargo.toml").exists())
        .map(Path::to_path_buf)
}

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
