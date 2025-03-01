use nannou::prelude::*;
use nannou_egui::Egui;
use std::cell::{Cell, Ref, RefCell};
use std::sync::mpsc;
use std::{env, str};

use super::prelude::*;
use crate::framework::{frame_controller, prelude::*};

pub fn run() {
    init_logger();
    gui::init();

    {
        let mut registry = REGISTRY.write().unwrap();
        register_sketches!(registry, template);

        // ---------------------------------------------------------------------
        // MAIN
        // ---------------------------------------------------------------------
        register_legacy_sketches!(
            registry,
            blob,
            breakpoints,
            breakpoints_2,
            brutalism,
            displacement_2a,
            drop,
            drop_walk,
            floor_supervisor,
            flow_field_basic,
            heat_mask,
            interference,
            kalos,
            kalos_2,
            sand_lines,
            sierpinski_triangle,
            spiral,
            spiral_lines,
            wave_fract
        );

        // ---------------------------------------------------------------------
        // DEV
        // ---------------------------------------------------------------------
        register_legacy_sketches!(
            registry,
            animation_dev,
            audio_controls_dev,
            audio_dev,
            control_script_dev,
            cv_dev,
            midi_dev,
            multiple_sketches_dev,
            osc_dev,
            osc_transport_dev,
            responsive_dev,
            shader_to_texture_dev,
            wgpu_compute_dev,
            wgpu_dev
        );

        // ---------------------------------------------------------------------
        // GENUARY 2025
        // ---------------------------------------------------------------------
        register_legacy_sketches!(
            registry,
            g25_10_11_12,
            g25_13_triangle,
            g25_14_black_and_white,
            g25_18_wind,
            g25_19_op_art,
            g25_1_horiz_vert,
            g25_20_23_brutal_arch,
            g25_22_gradients_only,
            g25_26_symmetry,
            g25_2_layers,
            g25_5_isometric
        );

        // ---------------------------------------------------------------------
        // SCRATCH
        // ---------------------------------------------------------------------
        register_legacy_sketches!(
            registry,
            bos,
            chromatic_aberration,
            displacement_1,
            displacement_1a,
            displacement_2,
            lin_alg,
            lines,
            noise,
            perlin_loop,
            sand_line,
            shader_experiments,
            vertical,
            vertical_2,
            z_sim
        );

        // ---------------------------------------------------------------------
        // TEMPLATES
        // ---------------------------------------------------------------------
        register_legacy_sketches!(
            registry,
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
    Alert(String),
    CaptureFrame,
    ClearFlag(bool),
    Reset,
    SwitchSketch(String),
    MidiInstruction(MidiInstruction),
}

#[derive(Debug)]
pub enum MidiInstruction {
    Start,
    Continue,
    Stop,
}

pub struct AppEventSender {
    tx: mpsc::Sender<AppEvent>,
}

impl AppEventSender {
    fn new(tx: mpsc::Sender<AppEvent>) -> Self {
        Self { tx }
    }

    pub fn alert(&self, message: impl Into<String>) {
        self.tx
            .send(AppEvent::Alert(message.into()))
            .expect("Failed to send alert event");
    }

    pub fn send(&self, event: AppEvent) {
        self.tx.send(event).expect("Failed to send event");
    }
}

pub type AppEventReceiver = mpsc::Receiver<AppEvent>;

struct AppModel {
    main_window_id: window::Id,
    gui_window_id: window::Id,
    egui: RefCell<Egui>,
    session_id: String,
    alert_text: String,
    clear_flag: Cell<bool>,
    recording_state: RecordingState,
    sketch: Box<dyn Sketch>,
    sketch_config: &'static SketchConfig,
    gui_visible: Cell<bool>,
    main_visible: Cell<bool>,
    main_maximized: Cell<bool>,
    event_tx: AppEventSender,
    event_rx: AppEventReceiver,
}

impl AppModel {
    fn main_window<'a>(&self, app: &'a App) -> Option<Ref<'a, Window>> {
        app.window(self.main_window_id)
    }

    fn gui_window<'a>(&self, app: &'a App) -> Option<Ref<'a, Window>> {
        app.window(self.gui_window_id)
    }

    fn window_rect<'a>(&self, app: &'a App) -> Option<Rect> {
        self.main_window(app).map(|window| window.rect())
    }

    fn sketch_name(&self) -> String {
        self.sketch_config.name.to_string()
    }

    fn on_app_event(&mut self, app: &App, event: AppEvent) {
        match event {
            AppEvent::Alert(text) => {
                self.alert_text = text;
            }
            AppEvent::CaptureFrame => {
                let filename =
                    format!("{}-{}.png", self.sketch_name(), uuid_5());

                let file_path =
                    lattice_project_root().join("images").join(&filename);

                self.main_window(app)
                    .unwrap()
                    .capture_frame(file_path.clone());

                let alert_text = format!("Image saved to {:?}", file_path);
                self.alert_text = alert_text.clone();
                info!("{}", alert_text);
            }
            AppEvent::ClearFlag(clear) => {
                self.clear_flag = clear.into();
            }
            AppEvent::Reset => {
                frame_controller::reset_frame_count();
                self.alert_text = "Reset".into();
            }
            AppEvent::SwitchSketch(name) => {
                self.switch_sketch(app, &name);
            }
            AppEvent::MidiInstruction(instruction) => {
                self.on_midi_instruction(&instruction);
            }
        }
    }

    fn on_midi_instruction(&mut self, instruction: &MidiInstruction) {
        match instruction {
            MidiInstruction::Start | MidiInstruction::Continue => {
                info!(
                    "Received {:?} message. Resetting frame count.",
                    instruction
                );

                frame_controller::reset_frame_count();

                if self.recording_state.is_queued {
                    match self.recording_state.start_recording() {
                        Ok(message) => {
                            self.event_tx.alert(message);
                        }
                        Err(e) => {
                            let message =
                                format!("Failed to start recording: {}", e);
                            self.event_tx.alert(message.clone());
                            error!("{}", message);
                        }
                    }
                }
            }
            MidiInstruction::Stop => {
                if self.recording_state.is_recording
                    && !self.recording_state.is_encoding
                {
                    match self
                        .recording_state
                        .stop_recording(self.sketch_config, &self.session_id)
                    {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Failed stop recording: {}", e);
                        }
                    }
                }
            }
        }
    }

    fn capture_recording_frame(&self, app: &App) {
        frame_controller::clear_force_render();

        if !self.recording_state.is_recording {
            return;
        }

        let frame_count = self.recording_state.recorded_frames.get();
        let window = self.main_window(app).unwrap();

        let recording_dir = match &self.recording_state.recording_dir {
            Some(path) => path,
            None => {
                error!("Unable to capture frame {}", frame_count);
                return;
            }
        };

        let filename = format!("frame-{:06}.png", frame_count);
        window.capture_frame(recording_dir.join(filename));

        self.recording_state.recorded_frames.set(frame_count + 1);
    }

    fn switch_sketch(&mut self, app: &App, name: &str) {
        let registry = REGISTRY.read().unwrap();
        let sketch_info = registry.get(name).unwrap();

        self.main_window(app).map(|window| {
            window.set_title(&sketch_info.config.display_name);
            set_window_position(app, self.main_window_id, 0, 0);
            set_window_size(
                window.winit_window(),
                sketch_info.config.w,
                sketch_info.config.h,
            );
        });

        let rect = self.window_rect(app).unwrap();
        let new_sketch = (sketch_info.factory)(app, rect);

        self.sketch = new_sketch;
        self.sketch_config = sketch_info.config;
        self.session_id = uuid_5();
        self.clear_flag.set(true);
        self.recording_state = RecordingState::new(frames_dir(
            &self.session_id,
            &self.sketch_config.name,
        ));

        self.gui_window(app).map(|gui_window| {
            gui_window.set_title(&format!(
                "{} Controls",
                sketch_info.config.display_name
            ));

            set_window_position(
                app,
                self.gui_window_id,
                sketch_info.config.w * 2,
                0,
            );

            let (gui_w, gui_h) =
                gui::calculate_gui_dimensions(self.sketch.controls_provided());

            set_window_size(
                gui_window.winit_window(),
                sketch_info.config.gui_w.unwrap_or(gui_w) as i32,
                sketch_info.config.gui_h.unwrap_or(gui_h) as i32,
            );
        });

        frame_controller::ensure_controller(sketch_info.config.fps);

        if sketch_info.config.play_mode != PlayMode::Loop {
            frame_controller::set_paused(true);
        }

        restore_controls(
            &sketch_info.config.name,
            self.sketch.controls_provided(),
        );

        self.alert_text =
            format!("Switched to {}", sketch_info.config.display_name);
    }
}

fn model(app: &App) -> AppModel {
    let args: Vec<String> = env::args().collect();
    let initial_sketch = args
        .get(1)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "template".to_string());

    let registry = REGISTRY.read().unwrap();

    let sketch_info = registry
        .get(&initial_sketch)
        .unwrap_or_else(|| panic!("Sketch not found: {}", initial_sketch));

    let sketch_config = sketch_info.config;

    let main_window_id = app
        .new_window()
        .title(sketch_info.config.display_name)
        .size(sketch_config.w as u32, sketch_config.h as u32)
        .build()
        .unwrap();

    let window_rect = app
        .window(main_window_id)
        .expect("Unable to get window")
        .rect();

    let mut sketch = (sketch_info.factory)(app, window_rect);

    let (gui_w, gui_h) =
        gui::calculate_gui_dimensions(sketch.controls_provided());

    let gui_window_id = app
        .new_window()
        .title(format!("{} Controls", sketch_config.display_name))
        .size(
            sketch_config.gui_w.unwrap_or(gui_w),
            sketch_config.gui_h.unwrap_or(gui_h),
        )
        .view(view_gui)
        .resizable(true)
        .raw_event(|_app, model: &mut AppModel, event| {
            model.egui.get_mut().handle_raw_event(event);
        })
        .build()
        .unwrap();

    set_window_position(app, main_window_id, 0, 0);
    set_window_position(app, gui_window_id, sketch_config.w * 2, 0);

    let egui =
        RefCell::new(Egui::from_window(&app.window(gui_window_id).unwrap()));

    restore_controls(&sketch_config.name, sketch.controls_provided());

    let session_id = uuid_5();
    let recording_dir = frames_dir(&session_id, &sketch_config.name);

    if sketch_config.play_mode != PlayMode::Loop {
        frame_controller::set_paused(true);
    }

    let (event_tx, event_rx) = mpsc::channel();
    let midi_tx = event_tx.clone();

    midi::on_message(
        midi::ConnectionType::GlobalStartStop,
        crate::config::MIDI_CLOCK_PORT,
        move |message| match message[0] {
            START => midi_tx
                .send(AppEvent::MidiInstruction(MidiInstruction::Start))
                .unwrap(),
            CONTINUE => midi_tx
                .send(AppEvent::MidiInstruction(MidiInstruction::Continue))
                .unwrap(),
            STOP => midi_tx
                .send(AppEvent::MidiInstruction(MidiInstruction::Stop))
                .unwrap(),
            _ => {}
        },
    )
    .expect(&format!(
        "Failed to initialize {:?} MIDI connection",
        midi::ConnectionType::GlobalStartStop
    ));

    AppModel {
        main_window_id,
        gui_window_id,
        egui,
        session_id,
        alert_text: format!("{} loaded", initial_sketch),
        clear_flag: Cell::new(false),
        recording_state: RecordingState::new(recording_dir.clone()),
        sketch,
        sketch_config,
        gui_visible: Cell::new(true),
        main_visible: Cell::new(true),
        main_maximized: Cell::new(false),
        event_tx: AppEventSender::new(event_tx),
        event_rx,
    }
}

fn update(app: &App, model: &mut AppModel, update: Update) {
    model.egui.borrow_mut().set_elapsed_time(update.since_start);

    {
        let mut egui = model.egui.borrow_mut();
        let ctx = egui.begin_frame();
        gui::update(
            &mut model.session_id,
            &model.sketch_config,
            model.sketch.controls_provided(),
            &mut model.alert_text,
            &mut model.recording_state,
            &model.event_tx,
            &ctx,
        );
    }

    while let Ok(event) = model.event_rx.try_recv() {
        model.on_app_event(app, event);
    }

    model.main_window(app).map(|window| {
        let rect = window.rect();
        model.sketch.set_window_rect(rect);
    });

    frame_controller::wrapped_update(
        app,
        &mut model.sketch,
        update,
        |app, sketch, update| sketch.update(app, update),
    );

    if model.recording_state.is_encoding {
        model.recording_state.on_encoding_message(
            &model.sketch_config,
            &mut model.session_id,
            &mut model.alert_text,
        );
    }
}

fn event(app: &App, model: &mut AppModel, event: Event) {
    model.sketch.event(app, &event);

    match event {
        Event::WindowEvent {
            id,
            simple: Some(KeyPressed(key)),
            ..
        } if id == model.main_window_id => {
            on_key_pressed(app, model, key);
        }
        _ => {}
    }
}

fn view(app: &App, model: &AppModel, frame: Frame) {
    if model.clear_flag.get() {
        frame.clear(model.sketch.clear_color());
        model.clear_flag.set(false);
    }

    let did_render = frame_controller::wrapped_view(
        app,
        &model.sketch,
        frame,
        |app, sketch, frame| sketch.view(app, frame),
    );

    if did_render {
        model.capture_recording_frame(app);
    }
}

fn view_gui(_app: &App, model: &AppModel, frame: Frame) {
    model.egui.borrow().draw_to_frame(&frame).unwrap();
}

fn set_window_size(window: &nannou::winit::window::Window, w: i32, h: i32) {
    let logical_size = nannou::winit::dpi::LogicalSize::new(w, h);
    window.set_inner_size(logical_size);
}

fn on_key_pressed(app: &App, model: &AppModel, key: Key) {
    match key {
        Key::A if has_no_modifiers(app) => {
            frame_controller::advance_single_frame();
        }
        Key::C if has_no_modifiers(app) => {
            let window = app.window(model.gui_window_id).unwrap();
            let is_visible = model.gui_visible.get();

            if is_visible {
                window.set_visible(false);
                model.gui_visible.set(false);
            } else {
                window.set_visible(true);
                model.gui_visible.set(true);
            }
        }
        Key::S if has_no_modifiers(app) => {
            let window = app.window(model.main_window_id).unwrap();
            let is_visible = model.main_visible.get();

            if is_visible {
                window.set_visible(false);
                model.main_visible.set(false);
            } else {
                window.set_visible(true);
                model.main_visible.set(true);
            }
        }
        Key::F if has_no_modifiers(app) => {
            let window = app.window(model.main_window_id).unwrap();
            if let Some(monitor) = window.current_monitor() {
                let monitor_size = monitor.size();
                let is_maximized = model.main_maximized.get();

                if is_maximized {
                    window.set_inner_size_points(
                        model.sketch_config.w as f32,
                        model.sketch_config.h as f32,
                    );
                    model.main_maximized.set(false);
                } else {
                    window.set_inner_size_pixels(
                        monitor_size.width,
                        monitor_size.height,
                    );
                    model.main_maximized.set(true);
                }
            }
        }
        _ => {}
    }
}

fn has_no_modifiers(app: &App) -> bool {
    !app.keys.mods.alt()
        && !app.keys.mods.ctrl()
        && !app.keys.mods.shift()
        && !app.keys.mods.logo()
}

pub fn restore_controls(sketch_name: &str, controls: Option<&mut Controls>) {
    if let (Some(values), Some(controls)) =
        (storage::stored_controls(sketch_name), controls)
    {
        for (name, value) in values.into_iter() {
            controls.update_value(&name, value);
        }

        info!("Controls restored");
    }
}
