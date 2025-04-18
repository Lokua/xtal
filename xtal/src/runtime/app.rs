use chrono::Utc;
use nannou::prelude::*;
use std::cell::{Cell, Ref};
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::path::PathBuf;
use std::process::Child;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;
use std::{env, str, thread};

use super::map_mode::{MapMode, Mappings};
use super::recording::{self, RecordingState};
use super::registry::REGISTRY;
use super::serialization::{
    GLOBAL_SETTINGS_VERSION, GlobalSettings, TransitorySketchState,
};
use super::storage;
use super::tap_tempo::TapTempo;
use super::web_view::{self as wv};
use crate::framework::osc_receiver::SHARED_OSC_RECEIVER;
use crate::framework::{frame_controller, prelude::*};
use crate::runtime::global;

pub fn run() {
    nannou::app(model)
        .update(update)
        .view(view)
        .event(event)
        .run();
}

#[allow(rustdoc::private_intra_doc_links)]
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
    MappingsEnabled(bool),
    MidiContinue,
    MidiStart,
    MidiStop,
    OpenOsDir(wv::OsDir),
    Paused(bool),
    PerfMode(bool),
    QueueRecord,
    Quit,
    Randomize(Exclusions),
    ReceiveDir(wv::UserDir, String),
    ReceiveMappings(Mappings),
    RemoveMapping(String),
    Reset,
    Resize,
    Save(Exclusions),
    SendMidi,
    SendMappings,
    SnapshotDelete(String),
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
pub type ClearFlag = Rc<Cell<bool>>;

struct AppModel {
    app_rx: AppEventReceiver,
    app_tx: AppEventSender,
    clear_next_frame: ClearFlag,
    ctx: Context,
    hrcc: bool,
    image_index: Option<storage::ImageIndex>,
    mappings_enabled: bool,
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
    fn main_window<'b>(&self, app: &'b App) -> Option<Ref<'b, Window>> {
        app.window(self.main_window_id)
    }

    fn sketch_name(&self) -> String {
        self.sketch_config.name.to_string()
    }

    fn hub(&mut self) -> Option<&ControlHub<Timing>> {
        self.sketch.hub().and_then(|provider| {
            provider.as_any().downcast_ref::<ControlHub<Timing>>()
        })
    }

    fn hub_mut(&mut self) -> Option<&mut ControlHub<Timing>> {
        self.sketch.hub().and_then(|provider| {
            provider.as_any_mut().downcast_mut::<ControlHub<Timing>>()
        })
    }

    fn web_view_controls(&mut self) -> Vec<wv::Control> {
        self.hub().map_or_else(Vec::new, |hub| {
            hub.ui_controls
                .configs()
                .values()
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
                    &PathBuf::from(global::images_dir()).join(&filename);

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
                if let Some(hub) = self.hub_mut() {
                    hub.audio_controls
                        .restart()
                        .inspect_err(|e| {
                            error!("Error in ChangeAudioDevice: {}", e)
                        })
                        .ok();
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
                if let Some(hub) = self.hub_mut() {
                    hub.midi_controls
                        .restart()
                        .inspect_err(|e| {
                            error!("Error in ChangeMidiControlInputPort: {}", e)
                        })
                        .ok();
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
                if self.hub().is_none() {
                    return;
                }

                self.map_mode.currently_mapping = None;
                let mappings = self.map_mode.mappings();
                let app_tx = self.app_tx.clone();
                let hub = self.hub_mut().unwrap();

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

                    // Prevent blowing unchanged mappings away
                    if let Some(config) = hub.midi_controls.config(proxy_name) {
                        if config.channel == ch && config.cc == cc {
                            continue;
                        }
                    }

                    let slider_range = match hub.ui_controls.slider_range(&name)
                    {
                        Some(s) => s,
                        // Can happen when mappings have been setup for a
                        // slider, say `foo`, that is then is renamed. At this
                        // point we'll have a left over `foo__slider_proxy` but
                        // no `foo` slider to get a range from. This is a
                        // temporary solution until we find the best place to
                        // perform some validation and cleanup either when
                        // reading or writing saved state
                        None => {
                            app_tx.alert_and_log(
                                format!("No slider range for {}", name),
                                log::Level::Error,
                            );
                            continue;
                        }
                    };

                    hub.midi_controls.add(
                        proxy_name,
                        MidiControlConfig::new((ch, cc), slider_range, 0.0),
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
                self.hub_mut()
                    .unwrap()
                    .midi_controls
                    .remove(&MapMode::proxy_name(&name));

                self.map_mode.currently_mapping = Some(name.clone());

                let app_tx = self.app_tx.clone();
                self.map_mode
                    .start(&name, self.hrcc, move |result| {
                        if let Err(e) = result {
                            app_tx.alert_and_log(
                                format!("Error: {}", e),
                                log::Level::Error,
                            );
                        }
                        app_tx.emit(AppEvent::SendMappings);
                    })
                    .inspect_err(|e| error!("Error in CurrentlyMapping: {}", e))
                    .ok();
            }
            AppEvent::Hrcc(hrcc) => {
                self.hrcc = hrcc;
                if let Some(hub) = self.hub_mut() {
                    hub.midi_controls.hrcc = hrcc;
                    hub.midi_controls
                        .restart()
                        .inspect_err(|e| error!("Error in Hrcc: {}", e))
                        .ok();
                }
                self.save_global_state();

                self.app_tx.alert_and_log(
                    ternary!(
                        hrcc,
                        "Expecting 14bit MIDI CCs for channels 0-31",
                        "Expecting standard 7bit MIDI messages for all CCs"
                    ),
                    log::Level::Info,
                );
            }
            AppEvent::HubPopulated => {
                let controls = self.web_view_controls();
                let bypassed =
                    self.hub().map_or_else(HashMap::default, |h| h.bypassed());
                let event = wv::Event::HubPopulated((controls, bypassed));
                self.wv_tx.emit(event);
                self.app_tx.alert("Hub repopulated");
            }
            AppEvent::EncodingComplete => {
                self.wv_tx.emit(wv::Event::Encoding(false));
            }
            AppEvent::MappingsEnabled(enabled) => {
                self.mappings_enabled = enabled;
                if let Some(hub) = self.hub_mut() {
                    hub.midi_proxies_enabled = enabled;
                }
                self.save_global_state();
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
            AppEvent::OpenOsDir(os_dir) => {
                let result = match os_dir {
                    wv::OsDir::Cache => {
                        open::that(storage::cache_dir().unwrap_or_default())
                    }
                    wv::OsDir::Config => {
                        open::that(storage::config_dir().unwrap_or_default())
                    }
                };

                if let Err(e) = result {
                    self.app_tx.alert_and_log(
                        format!("Error in OpenOsDir: {}", e),
                        log::Level::Error,
                    );
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

                if self.recording_state.is_queued {
                    self.app_tx.alert_and_log(
                        "Recording queued. Awaiting MIDI start message.",
                        log::Level::Info,
                    );
                }
            }
            AppEvent::Quit => {
                debug!("AppEvent::Quit requested");
                match self.wv_process.kill() {
                    Ok(_) => debug!("Killed ui_process"),
                    Err(e) => error!("Error killing ui_process {}", e),
                }
                thread::sleep(Duration::from_millis(50));
                debug!("Exiting main process");
                std::process::exit(0);
            }
            AppEvent::Randomize(exclusions) => {
                let app_tx = self.app_tx.clone();
                if let Some(hub) = self.hub_mut() {
                    let msg = "Transition started";
                    app_tx.alert_and_log(msg, log::Level::Info);
                    hub.randomize(exclusions);
                }
            }
            AppEvent::ReceiveDir(user_dir, dir) => {
                if dir.is_empty() {
                    return error!(
                        "Received invalid user_dir: {:?}, dir: {}",
                        user_dir, dir
                    );
                }
                match user_dir {
                    wv::UserDir::Images => global::set_images_dir(dir),
                    wv::UserDir::UserData => {
                        global::set_user_data_dir(dir);
                        if let Some(image_index) = &self.image_index {
                            if !storage::image_metadata_exists()
                                && !image_index.items.is_empty()
                            {
                                storage::save_image_index(image_index)
                                    .inspect_err(|e| {
                                        error!(
                                            "Error saving image index: {}",
                                            e
                                        )
                                    })
                                    .ok();
                            }
                        }
                    }
                    wv::UserDir::Videos => global::set_videos_dir(dir),
                }
                self.save_global_state();
            }
            AppEvent::ReceiveMappings(mappings) => {
                self.map_mode.set_mappings(mappings);
            }
            AppEvent::RemoveMapping(name) => {
                self.map_mode.remove(&name);
                self.map_mode.currently_mapping = None;
                self.hub_mut()
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
            AppEvent::Save(exclusions) => {
                let mappings = self.map_mode.mappings();

                match storage::save_sketch_state(
                    self.sketch_name().as_str(),
                    self.hub().unwrap(),
                    mappings,
                    exclusions,
                ) {
                    Ok(path_buf) => {
                        self.app_tx.alert_and_log(
                            format!("Controls saved to {:?}", path_buf),
                            log::Level::Info,
                        );
                    }
                    Err(e) => {
                        self.app_tx.alert_and_log(
                            format!("Failed to save controls: {}", e),
                            log::Level::Error,
                        );
                    }
                }
            }
            AppEvent::SendMappings => {
                let mappings = self.map_mode.mappings();
                self.wv_tx.emit(wv::Event::Mappings(mappings));
            }
            AppEvent::SendMidi => {
                let hrcc = self.hrcc;

                let messages = self
                    .hub()
                    .map(|hub| {
                        if hrcc {
                            hub.midi_controls.messages_hrcc()
                        } else {
                            hub.midi_controls.messages()
                        }
                    })
                    .unwrap_or_default();

                if messages.is_empty() {
                    return;
                }

                let Some(midi_out) = &mut self.midi_out else {
                    self.app_tx.alert_and_log(
                        "Unable to send MIDI; no MIDI out connection",
                        log::Level::Warn,
                    );
                    return;
                };

                let mut any_sent = false;
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
                    } else {
                        any_sent = true;
                    }
                }

                if any_sent {
                    self.app_tx.alert_and_log("MIDI Sent", log::Level::Info);
                }
            }
            AppEvent::SnapshotEnded => {
                let controls = self.web_view_controls();
                self.wv_tx.emit(wv::Event::SnapshotEnded(controls));
                self.app_tx.alert_and_log(
                    "Snapshot/Transition ended",
                    log::Level::Info,
                );
                self.app_tx.emit(AppEvent::SendMidi);
            }
            AppEvent::SnapshotDelete(id) => {
                if let Some(hub) = self.hub_mut() {
                    hub.delete_snapshot(&id);
                    self.app_tx.alert_and_log(
                        format!("Snapshot {:?} deleted", id),
                        log::Level::Info,
                    );
                }
            }
            AppEvent::SnapshotRecall(id) => {
                if let Some(hub) = self.hub_mut() {
                    match hub.recall_snapshot(&id) {
                        Ok(_) => {
                            self.app_tx.alert_and_log(
                                format!("Snapshot {:?} recalled", id),
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
                if let Some(hub) = self.hub_mut() {
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
                self.switch_sketch(app, &name);
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
                self.app_tx.alert_and_log(
                    ternary!(
                        enabled,
                        "Tap `Space` key to set BPM",
                        "Sketch BPM has been restored"
                    ),
                    log::Level::Info,
                );
            }
            AppEvent::TransitionTime(transition_time) => {
                self.transition_time = transition_time;
                if let Some(hub) = self.hub_mut() {
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
                let window = self.main_window(app).unwrap();
                window.set_visible(true);
                window.winit_window().focus_window();
            }
            AppEvent::UpdateUiControl((name, value)) => {
                let hub = self.hub_mut().unwrap();
                hub.ui_controls.set(&name, value.clone());

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
                    images_dir: global::images_dir(),
                    is_light_theme: matches!(
                        dark_light::detect(),
                        dark_light::Mode::Light
                    ),
                    mappings_enabled: self.mappings_enabled,
                    midi_clock_port: global::midi_clock_port(),
                    midi_input_port: global::midi_control_in_port(),
                    midi_output_port: global::midi_control_out_port(),
                    midi_input_ports: midi::list_input_ports().unwrap(),
                    midi_output_ports: midi::list_output_ports().unwrap(),
                    osc_port: global::osc_port(),
                    sketch_names: registry.names().clone(),
                    sketch_name: self.sketch_name(),
                    transition_time: self.transition_time,
                    user_data_dir: global::user_data_dir(),
                    videos_dir: global::videos_dir(),
                });
            }
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
        self.sketch_config = sketch_info.config;
        self.session_id = recording::generate_session_id();
        self.clear_next_frame.set(true);

        let sketch = (sketch_info.factory)(app, &self.ctx);
        self.sketch = sketch;

        let mappings_enabled = self.mappings_enabled;
        if let Some(hub) = self.hub_mut() {
            hub.midi_proxies_enabled = mappings_enabled;
            hub.clear_snapshots();
        }

        self.init_sketch_environment(app);

        let display_name = sketch_info.config.display_name;
        self.app_tx.alert(format!("Switched to {}", display_name));
    }

    /// A helper to DRY-up the common needs of initializing a sketch on startup
    /// and switching sketches at runtime like window sizing, placement,
    /// persisted state recall, and sending data to the UI
    fn init_sketch_environment(&mut self, app: &App) {
        self.recording_state = recording::RecordingState::new(
            recording::frames_dir(&self.session_id, self.sketch_config.name),
        );

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

        let exclusions = self.load_sketch_state().unwrap_or_default();

        let mappings_enabled = self.mappings_enabled;
        let transition_time = self.transition_time;
        let tx1 = self.app_tx.clone();
        let tx2 = self.app_tx.clone();
        if let Some(hub) = self.hub_mut() {
            hub.register_populated_callback(move || {
                tx1.emit(AppEvent::HubPopulated);
            });
            hub.register_snapshot_ended_callback(move || {
                tx2.emit(AppEvent::SnapshotEnded);
            });
            hub.set_transition_time(transition_time);
            hub.midi_proxies_enabled = mappings_enabled;
        }

        let bypassed = self
            .hub_mut()
            .map_or_else(HashMap::default, |hub| hub.bypassed());

        let snapshot_slots = self
            .hub()
            .map_or_else(Vec::new, |hub| hub.snapshot_keys_sorted());

        let event = wv::Event::LoadSketch {
            bpm: self.ctx.bpm().get(),
            bypassed,
            controls: self.web_view_controls(),
            display_name: self.sketch_config.display_name.to_string(),
            fps: frame_controller::fps(),
            mappings: self.map_mode.mappings(),
            paused,
            perf_mode: self.perf_mode,
            sketch_name: self.sketch_name(),
            sketch_width: self.sketch_config.w,
            sketch_height: self.sketch_config.h,
            snapshot_slots,
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

    fn save_global_state(&mut self) {
        if let Err(e) = storage::save_global_state(GlobalSettings {
            version: GLOBAL_SETTINGS_VERSION.to_string(),
            images_dir: global::images_dir(),
            audio_device_name: global::audio_device_name(),
            hrcc: self.hrcc,
            mappings_enabled: self.mappings_enabled,
            midi_clock_port: global::midi_clock_port(),
            midi_control_in_port: global::midi_control_in_port(),
            midi_control_out_port: global::midi_control_out_port(),
            osc_port: global::osc_port(),
            transition_time: self.transition_time,
            user_data_dir: global::user_data_dir(),
            videos_dir: global::videos_dir(),
        }) {
            self.app_tx.alert_and_log(
                format!("Failed to persist global settings: {}", e),
                log::Level::Error,
            );
        } else {
            info!("Saved global state");
        }
    }

    /// Load MIDI, OSC, and UI controls along with any snapshots MIDI Mappings
    /// the user has saved to disk
    fn load_sketch_state(&mut self) -> Result<Exclusions, Box<dyn Error>> {
        let app_tx = self.app_tx.clone();
        let sketch_name = self.sketch_name();
        let mappings = self.map_mode.mappings();

        let mut current_state =
            self.hub()
                .map_or_else(TransitorySketchState::default, |hub| {
                    TransitorySketchState {
                        ui_controls: hub.ui_controls.clone(),
                        midi_controls: hub.midi_controls.clone(),
                        osc_controls: hub.osc_controls.clone(),
                        snapshots: hub.snapshots.clone(),
                        mappings,
                        exclusions: Vec::new(),
                    }
                });

        match storage::load_sketch_state(&sketch_name, &mut current_state) {
            Ok(state) => {
                self.map_mode.clear();
                self.map_mode.set_mappings(state.mappings.clone());

                let Some(hub) = self.hub_mut() else {
                    return Ok(Vec::new());
                };

                hub.merge_program_state(state);

                // TODO: not ideal to automatically start the MIDI listener in
                // hub init phase only to restart here each time
                hub.midi_controls
                    .restart()
                    .inspect_err(|e| {
                        error!("Error in load_sketch_state: {}", e)
                    })
                    .ok();

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
            info!("Restoring global settings: {:?}", gs);
            global::set_audio_device_name(&gs.audio_device_name);
            global::set_images_dir(gs.images_dir.clone());
            global::set_midi_clock_port(gs.midi_clock_port.clone());
            global::set_midi_control_in_port(gs.midi_control_in_port.clone());
            global::set_midi_control_out_port(gs.midi_control_out_port.clone());
            global::set_osc_port(gs.osc_port);
            global::set_user_data_dir(gs.user_data_dir.clone());
            global::set_videos_dir(gs.videos_dir.clone());
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
    let raw_bpm = bpm.get();

    let clear_next_frame = Rc::new(Cell::new(true));
    let ctx = Context::new(
        bpm_clone,
        clear_next_frame.clone(),
        WindowRect::new(rect),
    );

    frame_controller::set_fps(sketch_info.config.fps);
    let sketch = (sketch_info.factory)(app, &ctx);

    let (raw_event_tx, event_rx) = mpsc::channel();
    let midi_tx = raw_event_tx.clone();
    AppModel::start_midi_clock_listener(midi_tx);

    let mut midi = midi::MidiOut::new(&global::midi_control_out_port());
    let midi_out = match midi.connect() {
        Ok(_) => Some(midi),
        Err(e) => {
            error!("{}", e);
            None
        }
    };

    let image_index = storage::load_image_index()
        .inspect_err(|e| error!("Error in model: {}", e))
        .ok();

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
        clear_next_frame,
        ctx,
        hrcc: global_settings.hrcc,
        image_index,
        mappings_enabled: global_settings.mappings_enabled,
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

    // Should this come _after_ `wrapped_update` and possibly behind a
    // `did_update` returned from frame_controller?
    if let Some(hub) = model.hub_mut() {
        hub.update();
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
            let ctrl_pressed = app.keys.mods.ctrl();
            let has_no_modifiers = !app.keys.mods.alt()
                && !ctrl_pressed
                && !shift_pressed
                && !logo_pressed;

            let platform_mod_pressed =
                ternary!(cfg!(target_os = "macos"), logo_pressed, ctrl_pressed);

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
                } else if platform_mod_pressed {
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
                // F (any)
                Key::F => {
                    model.app_tx.emit(AppEvent::ToggleFullScreen);
                }
                // G
                Key::G if has_no_modifiers => {
                    model.app_tx.emit(AppEvent::ToggleGuiFocus);
                }
                // M or Shift M
                // Don't interfere with native minimization on macOS
                Key::M if !platform_mod_pressed => {
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
    let did_render = frame_controller::wrapped_view(
        app,
        &model.sketch,
        frame,
        |app, sketch, frame| sketch.view(app, frame, &model.ctx),
    );

    if did_render {
        frame_controller::clear_force_render();

        if model.clear_next_frame.get() {
            model.clear_next_frame.set(false);
        }

        if model.recording_state.is_recording {
            model.capture_recording_frame(app);
        }
    }
}
