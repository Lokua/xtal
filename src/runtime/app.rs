use nannou::prelude::*;
use nannou_egui::Egui;
use std::cell::{Cell, RefCell};
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

        register_legacy_sketches!(
            registry,
            //
            // --- MAIN
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
            wave_fract,
            //
            // --- DEV
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
            wgpu_dev,
            //
            // --- GENUARY 2025
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
            g25_5_isometric,
            //
            // --- SCRATCH
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
            z_sim,
            //
            // --- TEMPLATES
            basic_cube_shader_template,
            fullscreen_shader_template // template
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
}

struct DynamicModel {
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
    event_channel: (mpsc::Sender<UiEvent>, mpsc::Receiver<UiEvent>),
}

fn model(app: &App) -> DynamicModel {
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
        .raw_event(|_app, model: &mut DynamicModel, event| {
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

    DynamicModel {
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
        event_channel: mpsc::channel(),
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

fn update(app: &App, model: &mut DynamicModel, update: Update) {
    model.egui.borrow_mut().set_elapsed_time(update.since_start);
    {
        let mut egui = model.egui.borrow_mut();
        let ctx = egui.begin_frame();
        let (tx, _) = &model.event_channel;
        gui::update_gui(
            app,
            &mut model.current_sketch_name,
            model.main_window_id,
            &mut model.session_id,
            &model.current_sketch_config,
            model
                .current_sketch
                .controls()
                .map(|provider| provider.as_controls()),
            &mut model.alert_text,
            &mut model.clear_flag,
            &mut model.recording_state,
            &tx,
            &ctx,
        );
    }

    while let Ok(event) = model.event_channel.1.try_recv() {
        match event {
            UiEvent::SwitchSketch(name) => {
                switch_sketch(app, model, &name);
            }
        }
    }

    if let Some(window) = app.window(model.main_window_id) {
        let rect = window.rect();
        model.current_sketch.set_window_rect(rect);
    }

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
                    &mut model.alert_text,
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

fn event(app: &App, model: &mut DynamicModel, event: Event) {
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

fn view(app: &App, model: &DynamicModel, frame: Frame) {
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

fn view_gui(_app: &App, model: &DynamicModel, frame: Frame) {
    model.egui.borrow().draw_to_frame(&frame).unwrap();
}

fn switch_sketch(app: &App, model: &mut DynamicModel, name: &str) {
    let registry = REGISTRY.lock().unwrap();
    let sketch_info = registry.get(name).unwrap();

    if let Some(window) = app.window(model.main_window_id) {
        window.set_title(&sketch_info.config.display_name);
        set_window_position(app, model.main_window_id, 0, 0);
        let winit_window = window.winit_window();
        set_window_size(
            winit_window,
            sketch_info.config.w,
            sketch_info.config.h,
        );
    }

    let rect = app
        .window(model.main_window_id)
        .expect("Unable to get window")
        .rect();

    let new_sketch = (sketch_info.factory)(app, rect);

    model.current_sketch = new_sketch;
    model.current_sketch_name = name.to_string();
    model.current_sketch_config = sketch_info.config;

    if let Some(window) = app.window(model.gui_window_id) {
        window.set_title(&format!(
            "{} Controls",
            sketch_info.config.display_name
        ));
        let winit_window = window.winit_window();
        set_window_position(
            app,
            model.gui_window_id,
            sketch_info.config.w * 2,
            0,
        );
        let (gui_w, gui_h) = gui::calculate_gui_dimensions(
            model
                .current_sketch
                .controls()
                .map(|provider| provider.as_controls()),
        );
        set_window_size(
            winit_window,
            sketch_info.config.gui_w.unwrap_or(gui_w) as i32,
            sketch_info.config.gui_h.unwrap_or(gui_h) as i32,
        );
    }

    frame_controller::ensure_controller(sketch_info.config.fps);

    if sketch_info.config.play_mode != PlayMode::Loop {
        frame_controller::set_paused(true);
    }

    model.clear_flag.set(true);
    model.alert_text =
        format!("Switched to {}", sketch_info.config.display_name);

    if let Some(values) = storage::stored_controls(&sketch_info.config.name) {
        if let Some(controls) = model.current_sketch.controls() {
            for (name, value) in values.into_iter() {
                controls.update_value(&name, value);
            }
            info!("Controls restored")
        }
    }
}

fn set_window_size(window: &nannou::winit::window::Window, w: i32, h: i32) {
    let logical_size = nannou::winit::dpi::LogicalSize::new(w, h);
    window.set_inner_size(logical_size);
}

fn on_key_pressed(app: &App, model: &DynamicModel, key: Key) {
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
    alert_text: &mut String,
    instruction: MidiInstruction,
) {
    match instruction {
        MidiInstruction::Start => {
            info!("Received MIDI Start message. Resetting frame count.");
            frame_controller::reset_frame_count();
            if recording_state.is_queued {
                recording_state
                    .start_recording(alert_text)
                    .expect("Unable to start frame recording.");
            }
        }
        MidiInstruction::Continue => {
            if recording_state.is_queued {
                info!(
                    "Received MIDI Continue message. \
                    Resetting frame count due to QUE_RECORD state."
                );
                frame_controller::reset_frame_count();
                recording_state
                    .start_recording(alert_text)
                    .expect("Unable to start frame recording.");
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

pub fn capture_frame(
    window: &nannou::window::Window,
    app: &App,
    sketch_name: &str,
    alert_text: &mut String,
) {
    let filename = format!("{}-{}.png", sketch_name, uuid_5());
    let file_path = app.project_path().unwrap().join("images").join(&filename);

    window.capture_frame(file_path.clone());
    info!("Image saved to {:?}", file_path);
    *alert_text = format!("Image saved to {:?}", file_path);
}
