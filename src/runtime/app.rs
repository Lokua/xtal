use chrono::Utc;
use nannou::prelude::*;
use std::cell::{Cell, Ref};
use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::Duration;
use std::{env, str, thread};

use super::map_mode::MapMode;
use super::recording::{frames_dir, RecordingState};
use super::registry::REGISTRY;
use super::serialization::SaveableProgramState;
use super::shared::lattice_project_root;
use super::storage::{self, load_program_state};
use super::tap_tempo::TapTempo;
use super::web_view::{self as ui};
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
    ReceiveMappings(Vec<(String, ChannelAndController)>),
    Record,
    RemoveMapping(String),
    Reset,
    Resize,
    SaveProgramState,
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
    main_window_id: window::Id,
    session_id: String,
    clear_next_frame: Cell<bool>,
    tap_tempo: TapTempo,
    tap_tempo_enabled: bool,
    perf_mode: bool,
    recording_state: RecordingState,
    sketch: Box<dyn SketchAll>,
    sketch_config: &'static SketchConfig,
    main_maximized: Cell<bool>,
    app_tx: AppEventSender,
    app_rx: AppEventReceiver,
    ctx: LatticeContext,
    midi_out: Option<midi::MidiOut>,
    transition_time: f32,
    image_index: Option<storage::ImageIndex>,
    map_mode: MapMode,
    hrcc: bool,
    ui_tx: ui::EventSender,
    ui_ready: bool,
    ui_pending_messages: VecDeque<ui::Event>,
}

impl AppModel {
    fn main_window<'a>(&self, app: &'a App) -> Option<Ref<'a, Window>> {
        app.window(self.main_window_id)
    }

    fn sketch_name(&self) -> String {
        self.sketch_config.name.to_string()
    }

    fn control_hub(&mut self) -> Option<&ControlHub<Timing>> {
        if let Some(provider) = self.sketch.controls() {
            provider.as_any().downcast_ref::<ControlHub<Timing>>()
        } else {
            None
        }
    }

    fn control_hub_mut(&mut self) -> Option<&mut ControlHub<Timing>> {
        if let Some(provider) = self.sketch.controls() {
            provider.as_any_mut().downcast_mut::<ControlHub<Timing>>()
        } else {
            None
        }
    }

    fn web_view_controls(&mut self) -> Vec<ui::SerializableControl> {
        let controls: Vec<ui::SerializableControl> = match self.control_hub() {
            Some(hub) => hub
                .ui_controls
                .configs()
                .iter()
                .map(|config| ui::SerializableControl::from((config, hub)))
                .collect(),
            None => vec![],
        };

        controls
    }

    fn on_app_event(&mut self, app: &App, event: AppEvent) {
        match event {
            AppEvent::AdvanceSingleFrame => {
                frame_controller::advance_single_frame();
            }
            AppEvent::Alert(text) => {
                self.ui_tx.emit(ui::Event::Alert(text));
            }
            AppEvent::AlertAndLog(text, level) => {
                self.ui_tx.emit(ui::Event::Alert(text.clone()));

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
                    hub.midi_controls.add(
                        &MapMode::proxy_name(&name),
                        MidiControlConfig::new(
                            (ch, cc),
                            hub.ui_controls.slider_range(&name),
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
                if let Err(connection_result) =
                    self.map_mode.start(&name, self.hrcc, move || {
                        app_tx.emit(AppEvent::SendMappings);
                    })
                {
                    error!("{}", connection_result);
                }
            }
            AppEvent::Hrcc(hrcc) => {
                self.hrcc = hrcc;
                if let Some(hub) = self.control_hub_mut() {
                    hub.midi_controls.hrcc = hrcc;
                    hub.midi_controls.restart().unwrap();
                }
            }
            AppEvent::HubPopulated => {
                let controls = self.web_view_controls();
                self.ui_tx.emit(ui::Event::HubPopulated(controls));
            }
            AppEvent::EncodingComplete => {
                self.ui_tx.emit(ui::Event::Encoding(false));
            }
            AppEvent::MidiStart | AppEvent::MidiContinue => {
                info!("Received MIDI Start/Continue. Resetting frame count.");

                frame_controller::reset_frame_count();

                if self.recording_state.is_queued {
                    match self.recording_state.start_recording() {
                        Ok(message) => {
                            self.app_tx.alert(message);
                            self.ui_tx.emit(ui::Event::StartRecording);
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
            AppEvent::ReceiveMappings(mappings) => {
                self.map_mode.update_from_vec(&mappings);
            }
            AppEvent::Record => {
                match self
                    .recording_state
                    .toggle_recording(self.sketch_config, &self.session_id)
                {
                    Ok(message) => {
                        self.app_tx.alert(message.clone());
                    }
                    Err(e) => {
                        self.app_tx.alert_and_log(
                            format!("Recording error: {}", e),
                            log::Level::Error,
                        );
                    }
                }
            }
            AppEvent::RemoveMapping(name) => {
                self.map_mode.remove(&name);
                self.map_mode.currently_mapping = None;
                self.app_tx.emit(AppEvent::SendMappings);
            }
            AppEvent::Reset => {
                frame_controller::reset_frame_count();
                self.app_tx.alert("Reset");
            }
            AppEvent::Resize => {
                if let Some(window) = self.main_window(app) {
                    let rect = window.rect();
                    let wr = &mut self.ctx.window_rect();

                    if rect.w() != wr.w() || rect.h() != wr.h() {
                        wr.set_current(rect);
                    }
                }
            }
            AppEvent::SaveProgramState => {
                match storage::save_program_state(
                    self.sketch_name().as_str(),
                    self.hrcc,
                    self.control_hub().unwrap(),
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
            }
            AppEvent::SendMappings => {
                let mappings = self.map_mode.mappings_as_vec();
                self.ui_tx.emit(ui::Event::Mappings(mappings));
            }
            AppEvent::SendMidi => {
                let messages = {
                    if let Some(hub) = self.control_hub() {
                        hub.midi_controls.messages()
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
                self.ui_tx.emit(ui::Event::SnapshotEnded(controls));
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
                            self.ui_tx.emit(ui::Event::Encoding(true));
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
                    self.ui_tx.emit(ui::Event::Bpm(self.ctx.bpm().get()));
                }
            }
            AppEvent::TapTempoEnabled(enabled) => {
                self.tap_tempo_enabled = enabled;
                self.ctx.bpm().set(self.sketch_config.bpm);
                self.ui_tx.emit(ui::Event::Bpm(self.ctx.bpm().get()));
            }
            AppEvent::TransitionTime(transition_time) => {
                self.transition_time = transition_time;
                if let Some(hub) = self.control_hub_mut() {
                    hub.set_transition_time(transition_time);
                }
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
                self.ui_tx.emit(ui::Event::ToggleGuiFocus);
            }
            AppEvent::ToggleMainFocus => {
                self.main_window(app).unwrap().set_visible(true);
            }
            AppEvent::UpdateUiControl((name, value)) => {
                let hub = self.control_hub_mut().unwrap();
                hub.update_ui_value(&name, value.clone());

                // Revaluate disabled state
                if matches!(
                    value,
                    ControlValue::Bool(_) | ControlValue::String(_)
                ) {
                    let controls = self.web_view_controls();
                    self.ui_tx.emit(ui::Event::UpdatedControls(controls));
                }
            }
            AppEvent::WebViewReady => {
                self.ui_ready = true;
                // Not clearing the queue as this is great for live reload!
                // TODO: find a better way since this can undo some state
                for message in &self.ui_pending_messages {
                    self.ui_tx.emit(message.clone());
                }
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
        let sketch = (sketch_info.factory)(app, &self.ctx);

        self.sketch = sketch;
        self.sketch_config = sketch_info.config;
        self.session_id = uuid_5();
        self.clear_next_frame.set(true);

        if let Some(provider) = self.sketch.controls() {
            provider.clear_snapshots();
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

        if let Some(window) = self.main_window(app) {
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
        }

        let paused = self.sketch_config.play_mode != PlayMode::Loop;
        frame_controller::set_paused(paused);

        self.load_program_state();

        let app_tx = self.app_tx.clone();
        if let Some(hub) = self.control_hub_mut() {
            hub.register_populated_callback(move || {
                app_tx.emit(AppEvent::HubPopulated);
            });
        }

        let app_tx = self.app_tx.clone();
        if let Some(hub) = self.control_hub_mut() {
            hub.register_snapshot_ended_callback(move || {
                app_tx.emit(AppEvent::SnapshotEnded);
            });
        }

        // TODO: set web view position
        let event = ui::Event::LoadSketch {
            bpm: self.ctx.bpm().get(),
            controls: self.web_view_controls(),
            display_name: self.sketch_config.display_name.to_string(),
            fps: frame_controller::fps(),
            mappings: self.map_mode.mappings_as_vec(),
            paused,
            sketch_name: self.sketch_name(),
            tap_tempo_enabled: self.tap_tempo_enabled,
        };

        if self.ui_ready {
            self.ui_tx.emit(event);
        } else {
            self.ui_pending_messages.push_back(event);
        }
    }

    /// Load MIDI, OSC, and ui controls along with any snapshots (and MIDI
    /// mappings [TODO]) the user has saved to disk
    fn load_program_state(&mut self) {
        let event_tx = self.app_tx.clone();
        let sketch_name = self.sketch_name();

        let mut current_state = match self.control_hub() {
            Some(hub) => SaveableProgramState {
                ui_controls: hub.ui_controls.clone(),
                midi_controls: hub.midi_controls.clone(),
                osc_controls: hub.osc_controls.clone(),
                snapshots: hub.snapshots.clone(),
                ..Default::default()
            },
            None => SaveableProgramState::default(),
        };

        match load_program_state(&sketch_name, &mut current_state) {
            Ok(state) => {
                self.hrcc = state.hrcc;

                let Some(hub) = self.control_hub_mut() else {
                    return;
                };

                hub.merge_program_state(state);

                if hub.snapshots.is_empty() {
                    event_tx
                        .alert_and_log("Controls restored", log::Level::Info);
                } else {
                    event_tx.alert_and_log(
                        format!(
                            "Controls restored. Available snapshots: {:?}",
                            hub.snapshot_keys_sorted()
                        ),
                        log::Level::Info,
                    );
                }
            }
            Err(e) => {
                warn!("Unable to restore controls: {}", e)
            }
        }
    }
}

fn model(app: &App) -> AppModel {
    let args: Vec<String> = env::args().collect();
    let initial_sketch = args
        .get(1)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "template".to_string());

    let registry = REGISTRY.read().unwrap();

    let sketch_info = registry.get(&initial_sketch).unwrap_or_else(|| {
        error!(
            "No sketch named `{}`. Defaulting to `template`",
            initial_sketch
        );
        registry.get("template").unwrap()
    });

    app.set_fullscreen_on_shortcut(false);

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

    let midi_handler_result = midi::on_message(
        midi::ConnectionType::GlobalStartStop,
        crate::config::MIDI_CLOCK_PORT,
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

    let raw_bpm = bpm.get();

    let mut midi = midi::MidiOut::new(crate::config::MIDI_CONTROL_OUT_PORT);
    let midi_out = match midi.connect() {
        Ok(_) => Some(midi),
        Err(e) => {
            error!("{}", e);
            None
        }
    };

    let image_index = storage::load_image_index()
        .map_err(|e| error!("{}", e))
        .ok();

    let event_tx = AppEventSender::new(raw_event_tx);
    let web_view_tx = ui::launch(&event_tx, sketch_info.config.name).unwrap();
    let ui_tx = web_view_tx.clone();

    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(1_000));
        ui_tx.emit(ui::Event::AverageFps(frame_controller::average_fps()));
    });

    let mut model = AppModel {
        main_window_id,
        session_id: uuid_5(),
        clear_next_frame: Cell::new(true),
        perf_mode: false,
        tap_tempo: TapTempo::new(raw_bpm),
        tap_tempo_enabled: false,
        recording_state: RecordingState::default(),
        sketch,
        sketch_config: sketch_info.config,
        main_maximized: Cell::new(false),
        app_tx: event_tx,
        app_rx: event_rx,
        midi_out,
        ctx,
        transition_time: 4.0,
        image_index,
        map_mode: MapMode::default(),
        hrcc: false,
        ui_tx: web_view_tx,
        ui_ready: false,
        ui_pending_messages: VecDeque::new(),
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
    #[allow(clippy::single_match)]
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
