use chrono::Utc;
use nannou::prelude::*;
use nannou_egui::Egui;
use std::cell::{Cell, Ref, RefCell};
use std::sync::mpsc;
use std::{env, str};

use super::prelude::*;
use super::tap_tempo::TapTempo;
use crate::config::MIDI_CONTROL_OUT_PORT;
use crate::framework::{frame_controller, prelude::*};

pub fn run() {
    init_logger();
    gui::init();

    {
        let mut registry = REGISTRY.write().unwrap();

        register_sketches!(
            registry,
            // -----------------------------------------------------------------
            // MAIN
            // -----------------------------------------------------------------
            blob,
            breakpoints_2,
            brutalism,
            displacement_2a,
            drop,
            drop_walk,
            flow_field_basic,
            heat_mask,
            interference,
            kalos,
            kalos_2,
            sand_lines,
            sierpinski_triangle,
            spiral,
            spiral_lines,
            wave_fract,
            // -----------------------------------------------------------------
            // DEV
            // -----------------------------------------------------------------
            animation_dev,
            audio_controls_dev,
            audio_dev,
            control_script_dev,
            cv_dev,
            dynamic_uniforms,
            effects_wavefolder_dev,
            midi_dev,
            non_yaml_dev,
            osc_dev,
            osc_transport_dev,
            responsive_dev,
            shader_to_texture_dev,
            wgpu_compute_dev,
            // -----------------------------------------------------------------
            // GENUARY 2025
            // -----------------------------------------------------------------
            g25_1_horiz_vert,
            g25_2_layers,
            g25_5_isometric,
            g25_10_11_12,
            g25_13_triangle,
            g25_14_black_and_white,
            g25_18_wind,
            g25_19_op_art,
            g25_20_23_brutal_arch,
            g25_22_gradients_only,
            g25_26_symmetry,
            // -----------------------------------------------------------------
            // SCRATCH
            // -----------------------------------------------------------------
            bos,
            chromatic_aberration,
            displacement_1,
            displacement_1a,
            displacement_2,
            lines,
            noise,
            perlin_loop,
            sand_line,
            shader_experiments,
            vertical,
            vertical_2,
            z_sim,
            // -----------------------------------------------------------------
            // TEMPLATES
            // -----------------------------------------------------------------
            template,
            basic_cube_shader_template,
            fullscreen_shader_template
        );

        registry.prepare();
    }

    nannou::app(model)
        .update(update)
        .view(view)
        .event(event)
        .run();
}

#[derive(Debug)]
pub enum AppEvent {
    AdvanceSingleFrame,
    Alert(String),
    AlertAndLog(String, log::Level),
    CaptureFrame,
    ClearNextFrame,
    MidiContinue,
    MidiStart,
    MidiStop,
    QueueRecord,
    Record,
    Reset,
    Resize,
    SaveControls,
    SendMidi,
    SetTransitionTime(f32),
    SnapshotRecall(String),
    SnapshotStore(String),
    SwitchSketch(String),
    Tap,
    ToggleFullScreen,
    ToggleGuiFocus,
    ToggleMainFocus,
    TogglePerfMode(bool),
    TogglePlay,
    ToggleTapTempo(bool),
}

#[derive(Clone)]
pub struct AppEventSender {
    tx: mpsc::Sender<AppEvent>,
}

impl AppEventSender {
    fn new(tx: mpsc::Sender<AppEvent>) -> Self {
        Self { tx }
    }

    pub fn send(&self, event: AppEvent) {
        self.tx.send(event).expect("Failed to send event");
    }

    pub fn alert(&self, message: impl Into<String>) {
        self.send(AppEvent::Alert(message.into()));
    }

    pub fn alert_and_log(&self, message: impl Into<String>, level: log::Level) {
        self.send(AppEvent::AlertAndLog(message.into(), level));
    }
}

pub type AppEventReceiver = mpsc::Receiver<AppEvent>;

struct AppModel {
    main_window_id: window::Id,
    gui_window_id: window::Id,
    egui: RefCell<Egui>,
    session_id: String,
    alert_text: String,
    clear_next_frame: Cell<bool>,
    tap_tempo: TapTempo,
    tap_tempo_enabled: bool,
    perf_mode: bool,
    recording_state: RecordingState,
    sketch: Box<dyn SketchAll>,
    sketch_config: &'static SketchConfig,
    main_maximized: Cell<bool>,
    event_tx: AppEventSender,
    event_rx: AppEventReceiver,
    ctx: LatticeContext,
    midi_out: Option<midi::MidiOut>,
    transition_time: f32,
    image_index: Option<storage::ImageIndex>,
}

impl AppModel {
    fn main_window<'a>(&self, app: &'a App) -> Option<Ref<'a, Window>> {
        app.window(self.main_window_id)
    }

    fn gui_window<'a>(&self, app: &'a App) -> Option<Ref<'a, Window>> {
        app.window(self.gui_window_id)
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

    fn on_app_event(&mut self, app: &App, event: AppEvent) {
        match event {
            AppEvent::AdvanceSingleFrame => {
                frame_controller::advance_single_frame();
            }
            AppEvent::Alert(text) => {
                self.alert_text = text;
            }
            AppEvent::AlertAndLog(text, level) => {
                self.alert_text = text.clone();

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

                self.event_tx.alert_and_log(
                    format!("Image saved to {:?}", file_path),
                    log::Level::Info,
                );
            }
            AppEvent::ClearNextFrame => {
                self.clear_next_frame.set(true);
            }
            AppEvent::MidiStart | AppEvent::MidiContinue => {
                info!(
                    "Received MIDI Start/Continue message. \
                    Resetting frame count."
                );

                frame_controller::reset_frame_count();

                if self.recording_state.is_queued {
                    match self.recording_state.start_recording() {
                        Ok(message) => {
                            self.event_tx.alert(message);
                        }
                        Err(e) => {
                            self.event_tx.alert_and_log(
                                format!("Failed to start recording: {}", e),
                                log::Level::Error,
                            );
                        }
                    }
                }
            }
            AppEvent::MidiStop => {
                if self.recording_state.is_recording
                    && !self.recording_state.is_encoding
                {
                    match self
                        .recording_state
                        .stop_recording(self.sketch_config, &self.session_id)
                    {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Failed to stop recording: {}", e);
                        }
                    }
                }
            }
            AppEvent::QueueRecord => {
                if self.recording_state.is_queued {
                    self.recording_state.is_queued = false;
                    self.alert_text = "".into();
                } else {
                    self.recording_state.is_queued = true;
                    self.alert_text =
                        "Recording queued. Awaiting MIDI Start message".into();
                }
            }
            AppEvent::Record => {
                match self
                    .recording_state
                    .toggle_recording(self.sketch_config, &self.session_id)
                {
                    Ok(message) => {
                        self.alert_text = message;
                    }
                    Err(e) => {
                        self.event_tx.alert_and_log(
                            format!("Recording error: {}", e),
                            log::Level::Error,
                        );
                    }
                }
            }
            AppEvent::Reset => {
                frame_controller::reset_frame_count();
                self.alert_text = "Reset".into();
            }
            AppEvent::Resize => {
                if let Some(window) = self.main_window(app) {
                    let rect = window.rect();
                    let window_rect = &mut self.ctx.window_rect();

                    if rect.w() != window_rect.w()
                        || rect.h() != window_rect.h()
                    {
                        window_rect.set_current(rect);
                    }
                }
            }
            AppEvent::SaveControls => {
                let sketch_name = self.sketch_name();
                match storage::save_controls(
                    sketch_name.as_str(),
                    self.control_hub().unwrap(),
                ) {
                    Ok(path_buf) => {
                        self.event_tx.alert_and_log(
                            format!("Controls persisted at {:?}", path_buf),
                            log::Level::Info,
                        );
                    }
                    Err(e) => {
                        self.event_tx.alert_and_log(
                            format!("Failed to persist controls: {}", e),
                            log::Level::Info,
                        );
                    }
                }
            }
            AppEvent::SendMidi => {
                if let Some(midi_out) = &mut self.midi_out {
                    if let Some(provider) = self.sketch.controls() {
                        if let Some(midi_controls) = provider.midi_controls() {
                            for message in midi_controls.messages() {
                                if let Err(e) = midi_out.send(&message) {
                                    error!(
                                        "Error sending MIDI message: {:?}; error: {}",
                                        message,
                                        e
                                    );
                                    break;
                                }
                            }
                            self.alert_text = "MIDI sent".into();
                        }
                    }
                } else {
                    warn!("Unable to send MIDI; no MIDI out connection");
                }
            }
            AppEvent::SetTransitionTime(transition_time) => {
                self.transition_time = transition_time;
                if let Some(hub) = self.sketch.controls() {
                    hub.set_transition_time(transition_time);
                }
            }
            AppEvent::SnapshotRecall(digit) => {
                if let Some(provider) = self.sketch.controls() {
                    match provider.recall_snapshot(&digit) {
                        Ok(_) => {
                            self.alert_text =
                                format!("Snapshot {:?} recalled", digit);
                        }
                        Err(e) => {
                            self.event_tx.alert_and_log(e, log::Level::Info);
                        }
                    }
                }
            }
            AppEvent::SnapshotStore(digit) => {
                if let Some(provider) = self.sketch.controls() {
                    provider.take_snapshot(&digit);
                    self.alert_text = format!("Snapshot {:?} saved", digit);
                } else {
                    error!("Unable to store snapshot ???");
                }
            }
            AppEvent::SwitchSketch(name) => {
                if self.sketch_config.name != name {
                    self.switch_sketch(app, &name);
                }
            }
            AppEvent::Tap => {
                if self.tap_tempo_enabled {
                    self.ctx.bpm().set(self.tap_tempo.tap());
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
                self.gui_window(app).unwrap().set_visible(true);
            }
            AppEvent::ToggleMainFocus => {
                self.main_window(app).unwrap().set_visible(true);
            }
            AppEvent::TogglePerfMode(perf_mode) => {
                self.perf_mode = perf_mode;
            }
            AppEvent::TogglePlay => {
                let next_is_paused = !frame_controller::is_paused();
                frame_controller::set_paused(next_is_paused);
                self.event_tx.alert_and_log(
                    ternary!(next_is_paused, "Paused", "Resumed"),
                    log::Level::Info,
                );
            }
            AppEvent::ToggleTapTempo(tap_tempo_enabled) => {
                self.tap_tempo_enabled = tap_tempo_enabled;
                self.ctx.bpm().set(self.sketch_config.bpm);
                self.alert_text = ternary!(
                    tap_tempo_enabled,
                    "Tap `Space` key to set BPM",
                    "Tap tempo disabled. Sketch BPM has been restored."
                )
                .into();
            }
        }
    }

    fn capture_recording_frame(&self, app: &App) {
        if !self.recording_state.is_recording {
            return;
        }

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

        self.alert_text =
            format!("Switched to {}", sketch_info.config.display_name);
    }

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

        let (gui_w, gui_h) =
            gui::calculate_gui_dimensions(self.sketch.ui_controls());

        if let Some(gui_window) = self.gui_window(app) {
            gui_window.set_title(&format!(
                "{} Controls",
                self.sketch_config.display_name
            ));

            if !self.perf_mode {
                set_window_position(
                    app,
                    self.gui_window_id,
                    self.sketch_config.w * 2,
                    0,
                );
            }

            set_window_size(
                gui_window.winit_window(),
                self.sketch_config.gui_w.unwrap_or(gui_w) as i32,
                self.sketch_config.gui_h.unwrap_or(gui_h) as i32,
            );
        }

        if self.sketch_config.play_mode != PlayMode::Loop {
            frame_controller::set_paused(true);
        }

        self.load_controls();
    }

    fn load_controls(&mut self) {
        let event_tx = self.event_tx.clone();

        if let Some(provider) = self.sketch.controls() {
            match provider.load_controls(self.sketch_config.name) {
                Some(Ok(())) => {
                    if let Some(hub) = self.control_hub() {
                        if hub.snapshots.is_empty() {
                            event_tx.alert_and_log(
                                "Controls restored",
                                log::Level::Info,
                            );
                        } else {
                            event_tx.alert_and_log(
                                format!(
                                    "Controls restored. Available snapshots: {:?}",
                                    hub.snapshot_keys_sorted()
                                ),
                                log::Level::Info
                            );
                        }
                    }
                }
                Some(Err(e)) => {
                    warn!("Unable to restore controls: {}", e)
                }
                None => warn!("Unable to restore controls"),
            };
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

    let gui_window_id = app
        .new_window()
        .view(view_gui)
        .raw_event(|_app, model: &mut AppModel, event| {
            model.egui.get_mut().handle_raw_event(event);
        })
        .build()
        .unwrap();

    let egui =
        RefCell::new(Egui::from_window(&app.window(gui_window_id).unwrap()));

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

    let mut midi = midi::MidiOut::new(MIDI_CONTROL_OUT_PORT);
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

    let mut model = AppModel {
        main_window_id,
        gui_window_id,
        egui,
        session_id: uuid_5(),
        alert_text: String::new(),
        clear_next_frame: Cell::new(true),
        perf_mode: false,
        tap_tempo: TapTempo::new(raw_bpm),
        tap_tempo_enabled: false,
        recording_state: RecordingState::new(frames_dir("", "")),
        sketch,
        sketch_config: sketch_info.config,
        main_maximized: Cell::new(false),
        event_tx: AppEventSender::new(raw_event_tx),
        event_rx,
        midi_out,
        ctx,
        transition_time: 4.0,
        image_index,
    };

    model.init_sketch_environment(app);

    model
}

fn update(app: &App, model: &mut AppModel, update: Update) {
    model.egui.borrow_mut().set_elapsed_time(update.since_start);

    {
        let mut egui = model.egui.borrow_mut();
        let ctx = egui.begin_frame();
        let bpm = model.ctx.bpm().get();
        gui::update(
            model.sketch_config,
            model.sketch.ui_controls(),
            &mut model.alert_text,
            &mut model.perf_mode,
            &mut model.tap_tempo_enabled,
            bpm,
            model.transition_time,
            &mut model.recording_state,
            &model.event_tx,
            &ctx,
        );
    }

    while let Ok(event) = model.event_rx.try_recv() {
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
            &model.event_tx,
        );
    }
}

/// Shared between main and gui windows
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
                    model.event_tx.send(AppEvent::SnapshotStore(digit));
                } else if logo_pressed {
                    model.event_tx.send(AppEvent::SnapshotRecall(digit));
                }
            }

            match key {
                Key::Space => {
                    model.event_tx.send(AppEvent::Tap);
                }
                // A
                Key::A if has_no_modifiers => {
                    model.event_tx.send(AppEvent::AdvanceSingleFrame);
                }
                // Cmd + F
                Key::F if logo_pressed => {
                    model.event_tx.send(AppEvent::ToggleFullScreen);
                }
                // Cmd + G
                Key::G if logo_pressed => {
                    model.event_tx.send(AppEvent::ToggleGuiFocus);
                }
                // Cmd + M
                Key::M if logo_pressed => {
                    model.event_tx.send(AppEvent::ToggleMainFocus);
                }
                // R
                Key::R if has_no_modifiers => {
                    model.event_tx.send(AppEvent::Reset);
                }
                // S
                Key::S if has_no_modifiers => {
                    model.event_tx.send(AppEvent::CaptureFrame);
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
                model.event_tx.send(AppEvent::Resize);
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
        model.capture_recording_frame(app);
    }
}

fn view_gui(_app: &App, model: &AppModel, frame: Frame) {
    model.egui.borrow().draw_to_frame(&frame).unwrap();
}
