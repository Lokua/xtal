use chrono::Utc;
use nannou::prelude::*;
use std::cell::{Cell, Ref};
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::process::Child;
use std::sync::mpsc;
use std::time::Duration;
use std::{env, str, thread};

use super::map_mode::MapMode;
use super::recording::{RecordingState, frames_dir};
use super::registry::REGISTRY;
use super::serialization::{GlobalSettings, SaveableProgramState};
use super::shared::lattice_project_root;
use super::storage;
use super::tap_tempo::TapTempo;
use super::web_view::{self as wv};
use crate::framework::osc_receiver::SHARED_OSC_RECEIVER;
use crate::framework::{frame_controller, prelude::*};

pub fn run() {
    nannou::app(model)
        .update(update)
        .view(view)
        .event(event)
        .run();
}

/// The core application event structure used to trigger [`AppModel`] updates
/// from keyboard and MIDI clock handlers as well as sending data to a web_view
/// (AppEvent -> WebView -> ipc_channel -> Frontend)
#[derive(Debug)]
pub enum AppEvent {
    AdvanceSingleFrame,
    Alert(String),
    AlertAndLog(String, log::Level),
    CaptureFrame,
    ChangeAudioDevice(String),
    ChangeMidiClockPort(String),
    ChangeMidiControlInputPort(String),
    ChangeMidiControlOutputPort(String),
    ChangeOscPort(u16),
    ClearNextFrame,
    CommitMappings,
    CurrentlyMapping(String),
    HubPopulated,
    Hrcc(bool),
    EncodingComplete,
    MidiContinue,
    MidiStart,
    MidiStop,
    Paused(bool),
    PerfMode(bool),
    QueueRecord,
    Quit,
    Randomize(Exclusions),
    ReceiveMappings(Vec<(String, ChannelAndController)>),
    Record,
    RemoveMapping(String),
    Reset,
    Resize,
    SaveProgramState(Exclusions),
    SendMidi,
    SendMappings,
    SnapshotRecall(String),
    SnapshotStore(String),
    SnapshotEnded,
    SwitchSketch(String),
    Tap,
    TapTempoEnabled(bool),
    TransitionTime(f32),
    StartRecording,
    StopRecording,
    ToggleFullScreen,
    ToggleGuiFocus,
    ToggleMainFocus,
    UpdateUiControl((String, ControlValue)),
    WebViewReady,
}

#[derive(Clone)]
pub struct AppEventSender {
    tx: mpsc::Sender<AppEvent>,
}

impl AppEventSender {
    fn new(tx: mpsc::Sender<AppEvent>) -> Self {
        Self { tx }
    }

    pub fn emit(&self, event: AppEvent) {
        self.tx.send(event).expect("Failed to send event");
    }

    pub fn alert(&self, message: impl Into<String>) {
        self.emit(AppEvent::Alert(message.into()));
    }

    pub fn alert_and_log(&self, message: impl Into<String>, level: log::Level) {
        self.emit(AppEvent::AlertAndLog(message.into(), level));
    }
}

pub type AppEventReceiver = mpsc::Receiver<AppEvent>;

struct AppModel {
    app_rx: AppEventReceiver,
    app_tx: AppEventSender,
    clear_next_frame: Cell<bool>,
    ctx: LatticeContext,
    hrcc: bool,
    image_index: Option<storage::ImageIndex>,
    main_maximized: Cell<bool>,
    main_window_id: window::Id,
    map_mode: MapMode,
    midi_out: Option<midi::MidiOut>,
    perf_mode: bool,
    recording_state: RecordingState,
    session_id: String,
    sketch: Box<dyn SketchAll>,
    sketch_config: &'static SketchConfig,
    tap_tempo: TapTempo,
    tap_tempo_enabled: bool,
    transition_time: f32,
    wv_pending_messages: VecDeque<wv::Event>,
    wv_process: Child,
    wv_ready: bool,
    wv_tx: wv::EventSender,
}

impl AppModel {
    fn main_window<'a>(&self, app: &'a App) -> Option<Ref<'a, Window>> {
        app.window(self.main_window_id)
    }

    fn sketch_name(&self) -> String {
        self.sketch_config.name.to_string()
    }

    fn control_hub(&mut self) -> Option<&ControlHub<Timing>> {
        self.sketch.controls().and_then(|provider| {
            provider.as_any().downcast_ref::<ControlHub<Timing>>()
        })
    }

    fn control_hub_mut(&mut self) -> Option<&mut ControlHub<Timing>> {
        self.sketch.controls().and_then(|provider| {
            provider.as_any_mut().downcast_mut::<ControlHub<Timing>>()
        })
    }

    fn web_view_controls(&mut self) -> Vec<wv::Control> {
        self.control_hub().map_or_else(Vec::new, |hub| {
            hub.ui_controls
                .configs()
                .iter()
                .map(|config| wv::Control::from((config, hub)))
                .collect()
        })
    }

    fn on_app_event(&mut self, app: &App, event: AppEvent) {
        match event {
            AppEvent::AdvanceSingleFrame => {
                frame_controller::advance_single_frame();
            }
            AppEvent::Alert(text) => {
                self.wv_tx.emit(wv::Event::Alert(text));
            }
            AppEvent::AlertAndLog(text, level) => {
                self.wv_tx.emit(wv::Event::Alert(text.clone()));

                match level {
                    log::Level::Error => error!("{}", text),
                    log::Level::Warn => warn!("{}", text),
                    log::Level::Info => info!("{}", text),
                    log::Level::Debug => debug!("{}", text),
                    log::Level::Trace => trace!("{}", text),
                }
            }
            AppEvent::CaptureFrame => {
                let filename =
                    format!("{}-{}.png", self.sketch_name(), uuid_5());

                let file_path =
                    lattice_project_root().join("images").join(&filename);

                self.main_window(app)
                    .unwrap()
                    .capture_frame(file_path.clone());

                if let Some(image_index) = &mut self.image_index {
                    image_index.items.push(storage::ImageIndexItem {
                        filename,
                        created_at: Utc::now().to_rfc3339().to_string(),
                    });
                    if let Err(e) = storage::save_image_index(image_index) {
                        error!("{}", e);
                    }
                }

                self.app_tx.alert_and_log(
                    format!("Image saved to {:?}", file_path),
                    log::Level::Info,
                );
            }
            AppEvent::ChangeAudioDevice(name) => {
                global::set_audio_device_name(&name);
                if let Some(hub) = self.control_hub_mut() {
                    hub.audio_controls.restart().inspect_err(log_err).ok();
                }
                self.save_global_state();
            }
            AppEvent::ChangeMidiClockPort(port) => {
                global::set_midi_clock_port(port);
                AppModel::start_midi_clock_listener(self.app_tx.tx.clone());
                self.save_global_state();
            }
            AppEvent::ChangeMidiControlInputPort(port) => {
                global::set_midi_control_in_port(port);
                if let Some(hub) = self.control_hub_mut() {
                    hub.midi_controls.restart().inspect_err(log_err).ok();
                }
                self.save_global_state();
            }
            AppEvent::ChangeMidiControlOutputPort(port) => {
                global::set_midi_control_out_port(port.clone());
                let mut midi = midi::MidiOut::new(&port);
                self.midi_out = match midi.connect() {
                    Ok(_) => Some(midi),
                    Err(e) => {
                        error!("{}", e);
                        None
                    }
                };
                self.save_global_state();
            }
            AppEvent::ChangeOscPort(port) => {
                global::set_osc_port(port);
                if let Err(e) = SHARED_OSC_RECEIVER.restart() {
                    error!("Failed to restart OSC receiver: {}", e);
                }
                self.save_global_state()
            }
            AppEvent::ClearNextFrame => {
                self.clear_next_frame.set(true);
            }
            AppEvent::CommitMappings => {
                if self.control_hub().is_none() {
                    return;
                }

                self.map_mode.currently_mapping = None;
                let mappings = self.map_mode.mappings_as_vec();
                let hub = self.control_hub_mut().unwrap();

                for (name, (ch, cc)) in mappings {
                    let proxy_name = &MapMode::proxy_name(&name);

                    if let Some(config) = hub.midi_controls.config(proxy_name) {
                        if config.channel == ch && config.cc == cc {
                            continue;
                        }
                    }

                    hub.midi_controls.add(
                        proxy_name,
                        MidiControlConfig::new(
                            (ch, cc),
                            hub.ui_controls.slider_range(&name).unwrap(),
                            0.0,
                        ),
                    );
                }

                if let Err(e) = hub.midi_controls.restart() {
                    error!("{}", e);
                }
            }
            AppEvent::CurrentlyMapping(name) => {
                if name.is_empty() {
                    self.map_mode.stop();
                    return;
                }

                self.map_mode.remove(&name);
                self.control_hub_mut()
                    .unwrap()
                    .midi_controls
                    .remove(&MapMode::proxy_name(&name));

                self.map_mode.currently_mapping = Some(name.clone());

                let app_tx = self.app_tx.clone();
                self.map_mode
                    .start(&name, self.hrcc, move || {
                        app_tx.emit(AppEvent::SendMappings);
                    })
                    .inspect_err(log_err)
                    .ok();
            }
            AppEvent::Hrcc(hrcc) => {
                self.hrcc = hrcc;
                if let Some(hub) = self.control_hub_mut() {
                    hub.midi_controls.hrcc = hrcc;
                    hub.midi_controls.restart().inspect_err(log_err).ok();
                }
                self.save_global_state();
            }
            AppEvent::HubPopulated => {
                let controls = self.web_view_controls();
                let bypassed = self
                    .control_hub()
                    .map_or_else(HashMap::default, |h| h.bypassed());
                let event = wv::Event::HubPopulated((controls, bypassed));
                self.wv_tx.emit(event);
            }
            AppEvent::EncodingComplete => {
                self.wv_tx.emit(wv::Event::Encoding(false));
            }
            AppEvent::MidiStart | AppEvent::MidiContinue => {
                info!("Received MIDI Start/Continue. Resetting frame count.");

                frame_controller::reset_frame_count();

                if self.recording_state.is_queued {
                    match self.recording_state.start_recording() {
                        Ok(message) => {
                            self.app_tx.alert(message);
                            self.wv_tx.emit(wv::Event::StartRecording);
                        }
                        Err(e) => {
                            self.app_tx.alert_and_log(
                                format!("Failed to start recording: {}", e),
                                log::Level::Error,
                            );
                        }
                    }
                }
            }
            AppEvent::MidiStop => {
                self.app_tx.emit(AppEvent::StopRecording);
            }
            AppEvent::Paused(paused) => {
                frame_controller::set_paused(paused);
            }
            AppEvent::PerfMode(perf_mode) => {
                self.perf_mode = perf_mode;
            }
            AppEvent::QueueRecord => {
                self.recording_state.is_queued =
                    !self.recording_state.is_queued;
            }
            AppEvent::Quit => {
                debug!("Quit requested");
                match self.wv_process.kill() {
                    Ok(_) => debug!("Killed ui_process"),
                    Err(e) => error!("Error killing ui_process {}", e),
                }
                thread::sleep(Duration::from_millis(50));
                debug!("Exiting main process");
                std::process::exit(0);
            }
            AppEvent::Randomize(exclusions) => {
                if let Some(hub) = self.control_hub_mut() {
                    hub.randomize(exclusions);
                }
            }
            AppEvent::ReceiveMappings(mappings) => {
                self.map_mode.update_from_vec(&mappings);
            }
            AppEvent::Record => {
                self.recording_state
                    .toggle_recording(self.sketch_config, &self.session_id)
                    .inspect(|message| self.app_tx.alert(message.clone()))
                    .inspect_err(|e| {
                        self.app_tx.alert_and_log(
                            format!("Recording error: {}", e),
                            log::Level::Error,
                        );
                    })
                    .ok();
            }
            AppEvent::RemoveMapping(name) => {
                self.map_mode.remove(&name);
                self.map_mode.currently_mapping = None;
                self.control_hub_mut()
                    .unwrap()
                    .midi_controls
                    .remove(&MapMode::proxy_name(&name));
                self.app_tx.emit(AppEvent::SendMappings);
            }
            AppEvent::Reset => {
                frame_controller::reset_frame_count();
                self.app_tx.alert("Reset");
            }
            AppEvent::Resize => {
                let window = self.main_window(app).unwrap();
                let rect = window.rect();
                let wr = &mut self.ctx.window_rect();

                if rect.w() != wr.w() || rect.h() != wr.h() {
                    wr.set_current(rect);
                }
            }
            AppEvent::SaveProgramState(exclusions) => {
                let mappings = self.map_mode.mappings();

                match storage::save_program_state(
                    self.sketch_name().as_str(),
                    self.control_hub().unwrap(),
                    mappings,
                    exclusions,
                ) {
                    Ok(path_buf) => {
                        self.app_tx.alert_and_log(
                            format!("Controls persisted at {:?}", path_buf),
                            log::Level::Info,
                        );
                    }
                    Err(e) => {
                        self.app_tx.alert_and_log(
                            format!("Failed to persist controls: {}", e),
                            log::Level::Error,
                        );
                    }
                }

                self.save_global_state();
            }
            AppEvent::SendMappings => {
                let mappings = self.map_mode.mappings_as_vec();
                self.wv_tx.emit(wv::Event::Mappings(mappings));
            }
            AppEvent::SendMidi => {
                let hrcc = self.hrcc;

                let messages = {
                    if let Some(hub) = self.control_hub() {
                        if hrcc {
                            hub.midi_controls.messages_hrcc()
                        } else {
                            hub.midi_controls.messages()
                        }
                    } else {
                        return;
                    }
                };

                let Some(midi_out) = &mut self.midi_out else {
                    self.app_tx.alert_and_log(
                        "Unable to send MIDI; no MIDI out connection",
                        log::Level::Warn,
                    );
                    return;
                };

                for message in messages {
                    if let Err(e) = midi_out.send(&message) {
                        self.app_tx.alert_and_log(
                            format!(
                                "Error sending MIDI message: {:?}; error: {}",
                                message, e
                            ),
                            log::Level::Error,
                        );
                        return;
                    }
                }

                self.app_tx.alert("MIDI Sent");
            }
            AppEvent::SnapshotEnded => {
                let controls = self.web_view_controls();
                self.wv_tx.emit(wv::Event::SnapshotEnded(controls));
                self.app_tx.emit(AppEvent::SendMidi);
            }
            AppEvent::SnapshotRecall(digit) => {
                if let Some(hub) = self.control_hub_mut() {
                    match hub.recall_snapshot(&digit) {
                        Ok(_) => {
                            self.app_tx.alert_and_log(
                                format!("Snapshot {:?} recalled", digit),
                                log::Level::Info,
                            );
                        }
                        Err(e) => {
                            self.app_tx.alert_and_log(e, log::Level::Error);
                        }
                    }
                }
            }
            AppEvent::SnapshotStore(digit) => {
                if let Some(hub) = self.control_hub_mut() {
                    hub.take_snapshot(&digit);
                    self.app_tx.alert_and_log(
                        format!("Snapshot {:?} saved", digit),
                        log::Level::Info,
                    );
                } else {
                    self.app_tx.alert_and_log(
                        "Unable to store snapshot (no hub)",
                        log::Level::Error,
                    );
                }
            }
            AppEvent::StartRecording => {
                match self.recording_state.start_recording() {
                    Ok(message) => {
                        self.app_tx.alert(message);
                    }
                    Err(e) => {
                        self.app_tx.alert_and_log(
                            format!("Failed to start recording: {}", e),
                            log::Level::Error,
                        );
                    }
                }
            }
            AppEvent::StopRecording => {
                let rs = &self.recording_state;

                if rs.is_recording && !rs.is_encoding {
                    match self
                        .recording_state
                        .stop_recording(self.sketch_config, &self.session_id)
                    {
                        Ok(_) => {
                            self.wv_tx.emit(wv::Event::Encoding(true));
                        }
                        Err(e) => {
                            error!("Failed to stop recording: {}", e);
                        }
                    }
                }
            }
            AppEvent::SwitchSketch(name) => {
                if self.sketch_name() != name {
                    self.switch_sketch(app, &name);
                }
            }
            AppEvent::Tap => {
                if self.tap_tempo_enabled {
                    self.ctx.bpm().set(self.tap_tempo.tap());
                    self.wv_tx.emit(wv::Event::Bpm(self.ctx.bpm().get()));
                }
            }
            AppEvent::TapTempoEnabled(enabled) => {
                self.tap_tempo_enabled = enabled;
                self.ctx.bpm().set(self.sketch_config.bpm);
                self.wv_tx.emit(wv::Event::Bpm(self.ctx.bpm().get()));
            }
            AppEvent::TransitionTime(transition_time) => {
                self.transition_time = transition_time;
                if let Some(hub) = self.control_hub_mut() {
                    hub.set_transition_time(transition_time);
                }
                self.save_global_state();
            }
            AppEvent::ToggleFullScreen => {
                let window = self.main_window(app).unwrap();
                if let Some(monitor) = window.current_monitor() {
                    let monitor_size = monitor.size();
                    let is_maximized = self.main_maximized.get();

                    if is_maximized {
                        window.set_inner_size_points(
                            self.sketch_config.w as f32,
                            self.sketch_config.h as f32,
                        );
                        self.main_maximized.set(false);
                    } else {
                        window.set_inner_size_pixels(
                            monitor_size.width,
                            monitor_size.height,
                        );
                        self.main_maximized.set(true);
                    }
                }
            }
            AppEvent::ToggleGuiFocus => {
                self.wv_tx.emit(wv::Event::ToggleGuiFocus);
            }
            AppEvent::ToggleMainFocus => {
                self.main_window(app).unwrap().set_visible(true);
            }
            AppEvent::UpdateUiControl((name, value)) => {
                let hub = self.control_hub_mut().unwrap();
                hub.ui_controls.update_value(&name, value.clone());

                // Revaluate disabled state
                if matches!(
                    value,
                    ControlValue::Bool(_) | ControlValue::String(_)
                ) {
                    let controls = self.web_view_controls();
                    self.wv_tx.emit(wv::Event::UpdatedControls(controls));
                }
            }
            AppEvent::WebViewReady => {
                self.wv_ready = true;

                // Not clearing the queue as this is great for live reload!
                // TODO: find a better way since this can undo some state
                for message in &self.wv_pending_messages {
                    self.wv_tx.emit(message.clone());
                }

                let registry = REGISTRY.read().unwrap();

                self.wv_tx.emit(wv::Event::Init {
                    audio_device: global::audio_device_name(),
                    audio_devices: list_audio_devices().unwrap_or_default(),
                    hrcc: self.hrcc,
                    is_light_theme: matches!(
                        dark_light::detect(),
                        dark_light::Mode::Light
                    ),
                    midi_clock_port: global::midi_clock_port(),
                    midi_input_port: global::midi_control_in_port(),
                    midi_output_port: global::midi_control_out_port(),
                    midi_input_ports: midi::list_input_ports().unwrap(),
                    midi_output_ports: midi::list_output_ports().unwrap(),
                    osc_port: global::osc_port(),
                    sketch_names: registry.names().clone(),
                    sketch_name: self.sketch_name(),
                    transition_time: self.transition_time,
                });
            }
        }
    }

    fn save_global_state(&self) {
        if let Err(e) = storage::save_global_state(GlobalSettings {
            audio_device_name: global::audio_device_name(),
            hrcc: self.hrcc,
            midi_clock_port: global::midi_clock_port(),
            midi_control_in_port: global::midi_control_in_port(),
            midi_control_out_port: global::midi_control_out_port(),
            osc_port: global::osc_port(),
            transition_time: self.transition_time,
            ..Default::default()
        }) {
            self.app_tx.alert_and_log(
                format!("Failed to persist global settings: {}", e),
                log::Level::Error,
            );
        } else {
            info!("Saved global state");
        }
    }

    fn capture_recording_frame(&self, app: &App) {
        let frame_count = self.recording_state.recorded_frames.get();
        let window = self.main_window(app).unwrap();

        let recording_dir = match &self.recording_state.recording_dir {
            Some(path) => path,
            None => {
                error!(
                    "Unable to access recording dir {:?}",
                    &self.recording_state.recording_dir
                );
                return;
            }
        };

        let filename = format!("frame-{:06}.png", frame_count);
        window.capture_frame(recording_dir.join(filename));

        self.recording_state.recorded_frames.set(frame_count + 1);
    }

    fn switch_sketch(&mut self, app: &App, name: &str) {
        let registry = REGISTRY.read().unwrap();

        let sketch_info = registry.get(name).unwrap_or_else(|| {
            error!("No sketch named `{}`. Defaulting to `template`", name);
            registry.get("template").unwrap()
        });

        frame_controller::set_fps(sketch_info.config.fps);
        let sketch = (sketch_info.factory)(app, &self.ctx);

        self.sketch = sketch;
        self.sketch_config = sketch_info.config;
        self.session_id = uuid_5();
        self.clear_next_frame.set(true);

        if let Some(hub) = self.control_hub_mut() {
            hub.clear_snapshots();
        }

        self.init_sketch_environment(app);

        self.app_tx
            .alert(format!("Switched to {}", sketch_info.config.display_name));
    }

    /// A helper to DRY-up the common needs of initializing a sketch on startup
    /// and switching sketches at runtime like window sizing, placement,
    /// persisted state recall, and sending data to the UI
    fn init_sketch_environment(&mut self, app: &App) {
        self.recording_state = RecordingState::new(frames_dir(
            &self.session_id,
            self.sketch_config.name,
        ));

        let window = self.main_window(app).unwrap();
        window.set_title(self.sketch_config.display_name);

        if !self.perf_mode {
            set_window_position(app, self.main_window_id, 0, 0);
            set_window_size(
                window.winit_window(),
                self.sketch_config.w,
                self.sketch_config.h,
            );
        }

        self.ctx.window_rect().set_current(window.rect());

        let paused = self.sketch_config.play_mode != PlayMode::Loop;
        frame_controller::set_paused(paused);

        let exclusions = self.load_program_state().unwrap_or_default();

        let tx1 = self.app_tx.clone();
        let tx2 = self.app_tx.clone();
        let transition_time = self.transition_time;
        if let Some(hub) = self.control_hub_mut() {
            hub.register_populated_callback(move || {
                tx1.emit(AppEvent::HubPopulated);
            });
            hub.register_snapshot_ended_callback(move || {
                tx2.emit(AppEvent::SnapshotEnded);
            });
            hub.set_transition_time(transition_time);
        }

        let bypassed = self
            .control_hub_mut()
            .map_or_else(HashMap::default, |hub| hub.bypassed());

        let event = wv::Event::LoadSketch {
            bpm: self.ctx.bpm().get(),
            bypassed,
            controls: self.web_view_controls(),
            display_name: self.sketch_config.display_name.to_string(),
            fps: frame_controller::fps(),
            mappings: self.map_mode.mappings_as_vec(),
            paused,
            perf_mode: self.perf_mode,
            sketch_name: self.sketch_name(),
            sketch_width: self.sketch_config.w,
            sketch_height: self.sketch_config.h,
            tap_tempo_enabled: self.tap_tempo_enabled,
            exclusions,
        };

        if self.wv_ready {
            self.wv_tx.emit(event);
        } else {
            self.wv_pending_messages.push_back(event);
        }

        self.app_tx.emit(AppEvent::SendMidi);
    }

    /// Load MIDI, OSC, and UI controls along with any snapshots MIDI Mappings
    /// the user has saved to disk
    fn load_program_state(&mut self) -> Result<Exclusions, Box<dyn Error>> {
        let app_tx = self.app_tx.clone();
        let sketch_name = self.sketch_name();
        let mappings = self.map_mode.mappings();

        let mut current_state = self.control_hub().map_or_else(
            SaveableProgramState::default,
            |hub| SaveableProgramState {
                ui_controls: hub.ui_controls.clone(),
                midi_controls: hub.midi_controls.clone(),
                osc_controls: hub.osc_controls.clone(),
                snapshots: hub.snapshots.clone(),
                mappings,
                exclusions: Vec::new(),
            },
        );

        match storage::load_program_state(&sketch_name, &mut current_state) {
            Ok(state) => {
                self.map_mode.clear();
                self.map_mode.set_mappings(state.mappings.clone());

                let Some(hub) = self.control_hub_mut() else {
                    return Ok(Vec::new());
                };

                hub.merge_program_state(state);

                // TODO: not ideal to automatically start the MIDI listener in
                // hub init phase only to restart here each time
                hub.midi_controls.restart().inspect_err(log_err).ok();

                if hub.snapshots.is_empty() {
                    app_tx.alert_and_log("Controls restored", log::Level::Info);
                } else {
                    app_tx.alert_and_log(
                        format!(
                            "Controls restored. Available snapshots: {:?}",
                            hub.snapshot_keys_sorted()
                        ),
                        log::Level::Info,
                    );
                }

                Ok(state.exclusions.clone())
            }
            Err(e) => {
                warn!("Unable to restore controls: {}", e);
                Err(e)
            }
        }
    }

    fn start_midi_clock_listener(midi_tx: mpsc::Sender<AppEvent>) {
        let midi_handler_result = midi::on_message(
            midi::ConnectionType::GlobalStartStop,
            &global::midi_clock_port(),
            move |_stamp, message| match message[0] {
                START => midi_tx.send(AppEvent::MidiStart).unwrap(),
                CONTINUE => midi_tx.send(AppEvent::MidiContinue).unwrap(),
                STOP => midi_tx.send(AppEvent::MidiStop).unwrap(),
                _ => {}
            },
        );
        if let Err(e) = midi_handler_result {
            warn!(
                "Failed to initialize {:?} MIDI connection. Error: {}",
                midi::ConnectionType::GlobalStartStop,
                e
            );
        }
    }
}

impl Drop for AppModel {
    fn drop(&mut self) {
        debug!("Dropping...");
        match self.wv_process.kill() {
            Ok(_) => debug!("Killed ui_process"),
            Err(e) => error!("Error killing ui_process {}", e),
        }
    }
}

fn model(app: &App) -> AppModel {
    let global_settings = match storage::load_global_state() {
        Ok(gs) => {
            info!("Restoring global settings: {:#?}", gs);
            global::set_audio_device_name(&gs.audio_device_name);
            global::set_midi_clock_port(gs.midi_clock_port.clone());
            global::set_midi_control_in_port(gs.midi_control_in_port.clone());
            global::set_midi_control_out_port(gs.midi_control_out_port.clone());
            global::set_osc_port(gs.osc_port);
            gs
        }
        Err(e) => {
            error!("Error loading global settings: {}", e);
            GlobalSettings::default()
        }
    };

    let args: Vec<String> = env::args().collect();
    let initial_sketch = args
        .get(1)
        .map_or_else(|| "template".to_string(), |s| s.to_string());

    let registry = REGISTRY.read().unwrap();

    let sketch_info = registry.get(&initial_sketch).unwrap_or_else(|| {
        error!(
            "No sketch named `{}`. Defaulting to `template`",
            initial_sketch
        );
        registry.get("template").unwrap()
    });

    app.set_fullscreen_on_shortcut(false);
    app.set_exit_on_escape(false);

    let main_window_id = app
        .new_window()
        .size(sketch_info.config.w as u32, sketch_info.config.h as u32)
        .build()
        .unwrap();

    let rect = app
        .window(main_window_id)
        .expect("Unable to get window")
        .rect();

    let bpm = Bpm::new(sketch_info.config.bpm);
    let bpm_clone = bpm.clone();
    let ctx = LatticeContext::new(bpm_clone, WindowRect::new(rect));

    frame_controller::set_fps(sketch_info.config.fps);
    let sketch = (sketch_info.factory)(app, &ctx);

    let (raw_event_tx, event_rx) = mpsc::channel();
    let midi_tx = raw_event_tx.clone();
    AppModel::start_midi_clock_listener(midi_tx);

    let raw_bpm = bpm.get();

    let mut midi = midi::MidiOut::new(&global::midi_control_out_port());
    let midi_out = match midi.connect() {
        Ok(_) => Some(midi),
        Err(e) => {
            error!("{}", e);
            None
        }
    };

    let image_index = storage::load_image_index().inspect_err(log_err).ok();

    let event_tx = AppEventSender::new(raw_event_tx);
    let (web_view_tx, ui_process) = wv::launch(&event_tx).unwrap();
    let ui_tx = web_view_tx.clone();

    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(1_000));
            ui_tx.emit(wv::Event::AverageFps(frame_controller::average_fps()));
        }
    });

    let mut model = AppModel {
        app_rx: event_rx,
        app_tx: event_tx,
        clear_next_frame: Cell::new(true),
        ctx,
        hrcc: global_settings.hrcc,
        image_index,
        main_maximized: Cell::new(false),
        main_window_id,
        map_mode: MapMode::default(),
        midi_out,
        perf_mode: false,
        recording_state: RecordingState::default(),
        session_id: uuid_5(),
        sketch,
        sketch_config: sketch_info.config,
        tap_tempo: TapTempo::new(raw_bpm),
        tap_tempo_enabled: false,
        transition_time: global_settings.transition_time,
        wv_pending_messages: VecDeque::new(),
        wv_process: ui_process,
        wv_ready: false,
        wv_tx: web_view_tx,
    };

    model.init_sketch_environment(app);

    model
}

fn update(app: &App, model: &mut AppModel, update: Update) {
    while let Ok(event) = model.app_rx.try_recv() {
        model.on_app_event(app, event);
    }

    frame_controller::wrapped_update(
        app,
        &mut model.sketch,
        update,
        |app, sketch, update| sketch.update(app, update, &model.ctx),
    );

    if model.recording_state.is_encoding {
        model.recording_state.on_encoding_message(
            model.sketch_config,
            &mut model.session_id,
            &model.app_tx,
        );
    }
}

fn event(app: &App, model: &mut AppModel, event: Event) {
    match event {
        Event::WindowEvent {
            simple: Some(KeyPressed(key)),
            ..
        } => {
            let logo_pressed = app.keys.mods.logo();
            let shift_pressed = app.keys.mods.shift();
            let has_no_modifiers = !app.keys.mods.alt()
                && !app.keys.mods.ctrl()
                && !shift_pressed
                && !logo_pressed;

            let digit = match key {
                Key::Key0 => Some("0"),
                Key::Key1 => Some("1"),
                Key::Key2 => Some("2"),
                Key::Key3 => Some("3"),
                Key::Key4 => Some("4"),
                Key::Key5 => Some("5"),
                Key::Key6 => Some("6"),
                Key::Key7 => Some("7"),
                Key::Key8 => Some("8"),
                Key::Key9 => Some("9"),
                _ => None,
            };

            if let Some(digit) = digit.map(|s| s.to_string()) {
                if shift_pressed {
                    model.app_tx.emit(AppEvent::SnapshotStore(digit));
                } else if logo_pressed {
                    model.app_tx.emit(AppEvent::SnapshotRecall(digit));
                }
            }

            match key {
                Key::Space => {
                    model.app_tx.emit(AppEvent::Tap);
                }
                // A
                Key::A if has_no_modifiers => {
                    model.app_tx.emit(AppEvent::AdvanceSingleFrame);
                }
                // Cmd + F
                Key::F if logo_pressed => {
                    model.app_tx.emit(AppEvent::ToggleFullScreen);
                }
                // Cmd + G
                Key::G if logo_pressed => {
                    model.app_tx.emit(AppEvent::ToggleGuiFocus);
                }
                // Cmd + M
                Key::M if logo_pressed && !shift_pressed => {
                    model.app_tx.emit(AppEvent::ToggleMainFocus);
                }
                // R
                Key::R if has_no_modifiers => {
                    model.app_tx.emit(AppEvent::Reset);
                }
                // S
                Key::S if has_no_modifiers => {
                    model.app_tx.emit(AppEvent::CaptureFrame);
                }
                _ => {}
            }
        }
        Event::WindowEvent {
            id,
            simple: Some(Resized(_)),
            ..
        } => {
            if id == model.main_window_id {
                model.app_tx.emit(AppEvent::Resize);
            }
        }
        _ => {}
    }
}

fn view(app: &App, model: &AppModel, frame: Frame) {
    if model.clear_next_frame.get() {
        frame.clear(model.sketch.clear_color());
        model.clear_next_frame.set(false);
    }

    let did_render = frame_controller::wrapped_view(
        app,
        &model.sketch,
        frame,
        |app, sketch, frame| sketch.view(app, frame, &model.ctx),
    );

    if did_render {
        frame_controller::clear_force_render();

        if model.recording_state.is_recording {
            model.capture_recording_frame(app);
        }
    }
}
