use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

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
use crate::framework::util::{HashMap, uuid_5};
use crate::framework::{frame_controller, logging};
use crate::gpu::CompiledGraph;
use crate::graph::GraphBuilder;
use crate::motion::{Bpm, Timing};
use crate::sketch::{Sketch, SketchConfig, TimingMode};
use crate::uniforms::UniformBanks;

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
    currently_mapping: Option<String>,
    sketch_ui_state: HashMap<String, SketchUiState>,
    recording_state: RecordingState,
    session_id: String,
    audio_device: String,
    audio_devices: Vec<String>,
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

        let mut sketch_ui_state = HashMap::default();
        sketch_ui_state.insert(active_name.clone(), SketchUiState::default());

        Ok(Self {
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
            currently_mapping: None,
            sketch_ui_state,
            recording_state: RecordingState::default(),
            session_id: recording::generate_session_id(),
            audio_device: global_settings.audio_device_name,
            audio_devices: vec![],
            midi_clock_port: global_settings.midi_clock_port,
            midi_input_port: global_settings.midi_control_in_port,
            midi_output_port: global_settings.midi_control_out_port,
            midi_input_ports: vec![],
            midi_output_ports: vec![],
            osc_port: global_settings.osc_port,
            images_dir: global_settings.images_dir,
            user_data_dir: global_settings.user_data_dir,
            videos_dir: global_settings.videos_dir,
            last_average_fps_emit: Instant::now(),
            shutdown_signaled: false,
            pending_png_capture_path: None,
            modifiers: ModifiersState::default(),
        })
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
                self.save_global_state();
            }
            RuntimeEvent::ChangeMidiClockPort(port) => {
                self.midi_clock_port = port;
                self.save_global_state();
            }
            RuntimeEvent::ChangeMidiControlInputPort(port) => {
                self.midi_input_port = port.clone();
                if !self
                    .midi_input_ports
                    .iter()
                    .any(|(_, existing)| existing == &port)
                {
                    let idx = self.midi_input_ports.len();
                    self.midi_input_ports.push((idx, port));
                }
                self.save_global_state();
            }
            RuntimeEvent::ChangeMidiControlOutputPort(port) => {
                self.midi_output_port = port.clone();
                if !self
                    .midi_output_ports
                    .iter()
                    .any(|(_, existing)| existing == &port)
                {
                    let idx = self.midi_output_ports.len();
                    self.midi_output_ports.push((idx, port));
                }
                self.save_global_state();
            }
            RuntimeEvent::ChangeOscPort(port) => {
                self.osc_port = port;
                self.save_global_state();
            }
            RuntimeEvent::ClearBuffer => {
                self.alert(
                    "ClearBuffer is not yet implemented in xtal2 runtime.",
                );
            }
            RuntimeEvent::CommitMappings => {
                self.currently_mapping = None;
            }
            RuntimeEvent::CurrentlyMapping(name) => {
                let currently_mapping =
                    if name.is_empty() { None } else { Some(name) };
                self.currently_mapping = currently_mapping.clone();
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
                self.current_sketch_ui_state_mut().mappings = mappings.clone();
            }
            RuntimeEvent::ReloadControls => {
                if let Some(hub) = self.control_hub.as_ref() {
                    hub.request_reload();
                }
            }
            RuntimeEvent::RemoveMapping(name) => {
                let mappings = {
                    let state = self.current_sketch_ui_state_mut();
                    state.mappings.remove(&name);
                    state.mappings.clone()
                };

                if let Some(hub) = self.control_hub.as_mut() {
                    hub.midi_controls.remove(&MapMode::proxy_name(&name));
                }

                self.currently_mapping = None;
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
                let mappings_to_save = self.current_sketch_ui_state().mappings;
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
            RuntimeEvent::SendMidi => {
                self.alert("SendMidi is not yet implemented in xtal2 runtime.");
            }
            RuntimeEvent::SetHrcc(enabled) => {
                if let Some(hub) = self.control_hub.as_mut() {
                    hub.hrcc(enabled);
                }
            }
            RuntimeEvent::SetMappingsEnabled(enabled) => {
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
                self.tap_tempo = TapTempo::new(self.config.bpm);
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
                let snapshot_sequence_enabled = self
                    .control_hub
                    .as_ref()
                    .is_some_and(|hub| hub.snapshot_sequence_enabled());
                self.emit_web_view_event(
                    web_view::Event::SnapshotSequenceEnabled(
                        snapshot_sequence_enabled,
                    ),
                );
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
        self.emit_event(RuntimeEvent::WebView(event));
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
        let hrcc = self
            .control_hub
            .as_ref()
            .is_some_and(|hub| hub.midi_controls.hrcc);

        let event = web_view::Event::Init {
            audio_device: self.audio_device.clone(),
            audio_devices: self.audio_devices.clone(),
            hrcc,
            images_dir: self.images_dir.clone(),
            is_light_theme: true,
            mappings_enabled: self.mappings_enabled,
            midi_clock_port: self.midi_clock_port.clone(),
            midi_input_port: self.midi_input_port.clone(),
            midi_output_port: self.midi_output_port.clone(),
            midi_input_ports: self.midi_input_ports.clone(),
            midi_output_ports: self.midi_output_ports.clone(),
            osc_port: self.osc_port,
            sketch_names: self.registry.sketch_names().to_vec(),
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

        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    // Swaps sketch instance/config, rebuilds runtime graph state, updates UI.
    fn switch_sketch(&mut self, name: &str) -> Result<(), String> {
        let (config, sketch) = instantiate_sketch(&self.registry, name)?;

        self.active_sketch_name = name.to_string();
        self.config = config;
        self.sketch = sketch;
        self.bpm.set(self.config.bpm);
        self.tap_tempo = TapTempo::new(self.config.bpm);
        frame_controller::set_fps(self.config.fps);

        if let Some(window) = self.window.as_ref() {
            window.set_title(self.config.display_name);
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

        Ok(())
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

        if let Err(err) =
            storage::save_global_state(&self.user_data_dir, settings)
        {
            self.alert_and_log(
                format!("Failed to persist global settings: {}", err),
                log::Level::Error,
            );
        }
    }

    // Loads per-sketch controls/snapshots/mappings/exclusions into runtime + hub.
    fn restore_sketch_state_from_disk(&mut self) {
        let current = self.current_sketch_ui_state();
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
                hub.ui_controls = state.ui_controls.clone();
                hub.midi_controls = state.midi_controls.clone();
                hub.osc_controls = state.osc_controls.clone();
                hub.snapshots = state.snapshots.clone();
                self.current_sketch_ui_state_mut().mappings =
                    state.mappings.clone();
                self.current_sketch_ui_state_mut().exclusions =
                    state.exclusions.clone();
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
        self.emit_event(RuntimeEvent::WebView(web_view::Event::Quit));
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
            error!("failed to initialize xtal2 runtime: {}", err);
            event_loop.exit();
            return;
        }

        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }

        frame_controller::set_fps(self.config.fps);
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
                if self.handle_main_window_shortcut(event_loop, &event) {
                    return;
                }
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
                        web_view::Event::Alert(message),
                    ));
                }
            }
            Err(err) => {
                let message = format!("Failed to save image capture: {}", err);
                error!("{}", message);
                if let Some(tx) = event_tx.as_ref() {
                    let _ = tx.send(RuntimeEvent::WebView(
                        web_view::Event::Alert(message),
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
