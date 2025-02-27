use nannou::prelude::*;
use nannou_egui::Egui;
use std::cell::{Cell, Ref, RefCell};
use std::sync::{mpsc, Once};
use std::{env, str};

use super::prelude::*;
use crate::framework::{frame_controller, prelude::*};

pub fn run() {
    init_logger();
    gui::init();

    {
        let mut registry = REGISTRY.lock().unwrap();

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
    }

    nannou::app(model)
        .update(update)
        .view(view)
        .event(event)
        .run();
}

pub enum UiEvent {
    SwitchSketch(String),
    Alert(String),
    ClearFlag(bool),
    CaptureFrame,
}

pub struct UiEventSender {
    tx: mpsc::Sender<UiEvent>,
}

impl UiEventSender {
    fn new(tx: mpsc::Sender<UiEvent>) -> Self {
        Self { tx }
    }

    pub fn alert(&self, message: impl Into<String>) {
        self.tx
            .send(UiEvent::Alert(message.into()))
            .expect("Failed to send alert event");
    }

    pub fn send(&self, event: UiEvent) {
        self.tx.send(event).expect("Failed to send event");
    }
}

pub type UiEventReceiver = mpsc::Receiver<UiEvent>;

struct AppModel {
    main_window_id: window::Id,
    gui_window_id: window::Id,
    egui: RefCell<Egui>,
    session_id: String,
    alert_text: String,
    clear_flag: Cell<bool>,
    recording_state: RecordingState,
    current_sketch: Box<dyn Sketch>,
    current_sketch_name: String,
    current_sketch_config: &'static SketchConfig,
    gui_visible: Cell<bool>,
    main_visible: Cell<bool>,
    main_maximized: Cell<bool>,
    event_tx: UiEventSender,
    event_rx: UiEventReceiver,
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

    fn switch_sketch(&mut self, app: &App, name: &str) {
        let registry = REGISTRY.lock().unwrap();
        let sketch_info = registry.get(name).unwrap();

        self.main_window(app).map(|window| {
            window.set_title(&sketch_info.config.display_name);
            set_window_position(app, self.main_window_id, 0, 0);
            let winit_window = window.winit_window();
            set_window_size(
                winit_window,
                sketch_info.config.w,
                sketch_info.config.h,
            );
        });

        let rect = self.window_rect(app).unwrap();
        let new_sketch = (sketch_info.factory)(app, rect);

        self.current_sketch = new_sketch;
        self.current_sketch_name = name.to_string();
        self.current_sketch_config = sketch_info.config;

        self.gui_window(app).map(|gui_window| {
            let title =
                &format!("{} Controls", sketch_info.config.display_name);
            gui_window.set_title(title);
            let winit_window = gui_window.winit_window();
            set_window_position(
                app,
                self.gui_window_id,
                sketch_info.config.w * 2,
                0,
            );
            let (gui_w, gui_h) = gui::calculate_gui_dimensions(
                self.current_sketch
                    .controls()
                    .map(|provider| provider.as_controls()),
            );
            set_window_size(
                winit_window,
                sketch_info.config.gui_w.unwrap_or(gui_w) as i32,
                sketch_info.config.gui_h.unwrap_or(gui_h) as i32,
            );
        });

        frame_controller::ensure_controller(sketch_info.config.fps);

        if sketch_info.config.play_mode != PlayMode::Loop {
            frame_controller::set_paused(true);
        }

        self.clear_flag.set(true);
        self.alert_text =
            format!("Switched to {}", sketch_info.config.display_name);

        if let Some(values) = storage::stored_controls(&sketch_info.config.name)
        {
            if let Some(controls) = self.current_sketch.controls() {
                for (name, value) in values.into_iter() {
                    controls.update_value(&name, value);
                }
                info!("Controls restored")
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

    let registry = REGISTRY.lock().unwrap();

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

    let mut current_sketch = (sketch_info.factory)(app, window_rect);

    let (gui_w, gui_h) = gui::calculate_gui_dimensions(
        current_sketch
            .controls()
            .map(|provider| provider.as_controls()),
    );

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

    if let Some(values) = storage::stored_controls(&sketch_config.name) {
        if let Some(controls) = current_sketch.controls() {
            for (name, value) in values.into_iter() {
                controls.update_value(&name, value);
            }
            info!("Controls restored")
        }
    }

    let session_id = uuid_5();
    let recording_dir = frames_dir(&session_id, &sketch_config.name);

    if sketch_config.play_mode != PlayMode::Loop {
        frame_controller::set_paused(true);
    }

    let (event_tx, event_rx) = mpsc::channel();

    AppModel {
        main_window_id,
        gui_window_id,
        egui,
        session_id,
        alert_text: format!("{} loaded", initial_sketch),
        clear_flag: Cell::new(false),
        recording_state: RecordingState::new(recording_dir.clone()),
        current_sketch,
        current_sketch_name: initial_sketch,
        current_sketch_config: sketch_config,
        gui_visible: Cell::new(true),
        main_visible: Cell::new(true),
        main_maximized: Cell::new(false),
        event_tx: UiEventSender::new(event_tx),
        event_rx,
    }
}

static INIT_MIDI_HANDLER: Once = Once::new();

enum MidiInstruction {
    Start,
    Continue,
    Stop,
}

thread_local! {
    static MIDI_MESSAGE_RX: RefCell<Option<mpsc::Receiver<MidiInstruction>>> =
        RefCell::new(None);
}

fn update(app: &App, model: &mut AppModel, update: Update) {
    model.egui.borrow_mut().set_elapsed_time(update.since_start);

    {
        let mut egui = model.egui.borrow_mut();
        let ctx = egui.begin_frame();
        gui::update_gui(
            &mut model.current_sketch_name,
            &mut model.session_id,
            &model.current_sketch_config,
            model
                .current_sketch
                .controls()
                .map(|provider| provider.as_controls()),
            &mut model.alert_text,
            &mut model.recording_state,
            &model.event_tx,
            &ctx,
        );
    }

    while let Ok(event) = model.event_rx.try_recv() {
        match event {
            UiEvent::SwitchSketch(name) => {
                model.switch_sketch(app, &name);
            }
            UiEvent::Alert(text) => {
                model.alert_text = text;
            }
            UiEvent::ClearFlag(clear) => {
                model.clear_flag = clear.into();
            }
            UiEvent::CaptureFrame => {
                capture_frame(
                    model.main_window(app).unwrap(),
                    &model.current_sketch_name,
                    &model.event_tx,
                );
            }
        }
    }

    model.main_window(app).map(|window| {
        let rect = window.rect();
        model.current_sketch.set_window_rect(rect);
    });

    frame_controller::wrapped_update(
        app,
        &mut model.current_sketch,
        update,
        |app, sketch, update| sketch.update(app, update),
    );

    INIT_MIDI_HANDLER.call_once(|| {
        let (tx, rx) = mpsc::channel();
        MIDI_MESSAGE_RX.with(|cell| {
            *cell.borrow_mut() = Some(rx);
        });
        on_message(
            move |message| {
                match message[0] {
                    START => {
                        tx.send(MidiInstruction::Start).unwrap();
                    }
                    CONTINUE => {
                        tx.send(MidiInstruction::Continue).unwrap();
                    }
                    STOP => {
                        tx.send(MidiInstruction::Stop).unwrap();
                    }
                    _ => {}
                };
            },
            "[Global Start/Stop]",
        )
        .expect("Failed to initialize MIDI handler");
    });

    MIDI_MESSAGE_RX.with(|cell| {
        if let Some(rx) = cell.borrow_mut().as_ref() {
            if let Ok(instruction) = rx.try_recv() {
                on_midi_instruction(
                    &mut model.recording_state,
                    model.current_sketch_config,
                    &model.session_id,
                    &model.event_tx,
                    instruction,
                );
            }
        }
    });

    if model.recording_state.is_encoding {
        model.recording_state.on_encoding_message(
            &mut model.session_id,
            model.current_sketch_config,
            &mut model.alert_text,
        );
    }
}

fn event(app: &App, model: &mut AppModel, event: Event) {
    model.current_sketch.event(app, &event);

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
        frame.clear(model.current_sketch.clear_color());
        model.clear_flag.set(false);
    }

    frame_controller::wrapped_view(
        app,
        &model.current_sketch,
        frame,
        |app, sketch, frame| sketch.view(app, frame),
    );
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
                        model.current_sketch_config.w as f32,
                        model.current_sketch_config.h as f32,
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

fn on_midi_instruction(
    recording_state: &mut RecordingState,
    sketch_config: &SketchConfig,
    session_id: &str,
    event_tx: &UiEventSender,
    instruction: MidiInstruction,
) {
    match instruction {
        MidiInstruction::Start => {
            info!("Received MIDI Start message. Resetting frame count.");
            frame_controller::reset_frame_count();
            if recording_state.is_queued {
                match recording_state.start_recording() {
                    Ok(message) => {
                        event_tx.alert(message);
                    }
                    Err(e) => {
                        event_tx
                            .alert(format!("Failed to start recording: {}", e));
                        error!("Failed to start recording: {}", e);
                    }
                }
            }
        }
        MidiInstruction::Continue => {
            if recording_state.is_queued {
                info!(
                    "Received MIDI Continue message. \
                    Resetting frame count due to QUE_RECORD state."
                );
                frame_controller::reset_frame_count();
                match recording_state.start_recording() {
                    Ok(message) => {
                        event_tx.alert(message);
                    }
                    Err(e) => {
                        event_tx
                            .alert(format!("Failed to start recording: {}", e));
                        error!("Failed to start recording: {}", e);
                    }
                }
            }
        }
        MidiInstruction::Stop => {
            if recording_state.is_recording && !recording_state.is_encoding {
                recording_state
                    .stop_recording(sketch_config, session_id)
                    .expect("Error attempting to stop recording");
            }
        }
    }
}

pub fn capture_frame<'a>(
    window: Ref<'a, Window>,
    sketch_name: &str,
    event_tx: &UiEventSender,
) {
    let filename = format!("{}-{}.png", sketch_name, uuid_5());
    let file_path = lattice_project_root().join("images").join(&filename);
    window.capture_frame(file_path.clone());
    event_tx.alert(format!("Image saved to {:?}", file_path));
    info!("Image saved to {:?}", file_path);
}
