use arboard::Clipboard;
use nannou::prelude::*;
use nannou_egui::egui::{self, FontDefinitions, FontFamily};
use nannou_egui::Egui;
use once_cell::sync::Lazy;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::{mpsc, Mutex, Once};
use std::{env, fs, str};

use framework::frame_controller;
use framework::prelude::*;
use runtime::prelude::*;

pub mod config;
pub mod framework;
pub mod runtime;
mod sketches;

const STORE_CONTROLS_CACHE_IN_PROJECT: bool = true;
const GUI_WIDTH: u32 = 560;

macro_rules! register_legacy_sketches {
    ($registry:expr, $($module:ident),*) => {
        $(
            $registry.register(
                &crate::sketches::$module::SKETCH_CONFIG,
                |app, rect| {
                    let model = crate::sketches::$module::init_model(
                        app,
                        WindowRect::new(rect)
                    );
                    Box::new(SketchAdapter::new(
                        model,
                        crate::sketches::$module::update,
                        crate::sketches::$module::view,
                    ))
                }
            );
        )*
    };
}

macro_rules! register_sketches {
    ($registry:expr, $($module:ident),*) => {
        $(
            $registry.register(
                &crate::sketches::$module::SKETCH_CONFIG,
                |app, rect| {
                    Box::new(crate::sketches::$module::init(
                        app,
                        WindowRect::new(rect)
                    ))
                }
            );
        )*
    };
}

struct SketchInfo {
    config: &'static SketchConfig,
    factory: Box<
        dyn for<'a> Fn(&'a App, Rect) -> Box<dyn Sketch + 'static>
            + Send
            + Sync,
    >,
}

static REGISTRY: Lazy<Mutex<SketchRegistry>> =
    Lazy::new(|| Mutex::new(SketchRegistry::new()));

struct SketchRegistry {
    sketches: HashMap<String, SketchInfo>,
    sorted_names: Option<Vec<String>>,
}

impl SketchRegistry {
    fn new() -> Self {
        Self {
            sketches: HashMap::new(),
            sorted_names: None,
        }
    }

    fn register<F>(&mut self, config: &'static SketchConfig, factory: F)
    where
        F: Fn(&App, Rect) -> Box<dyn Sketch> + Send + Sync + 'static,
    {
        self.sketches.insert(
            config.name.to_string(),
            SketchInfo {
                config,
                factory: Box::new(factory),
            },
        );
    }

    fn get(&self, name: &str) -> Option<&SketchInfo> {
        self.sketches.get(name)
    }

    fn names(&mut self) -> &Vec<String> {
        if self.sorted_names.is_none() {
            let mut names: Vec<String> =
                self.sketches.keys().cloned().collect();
            names.sort();
            self.sorted_names = Some(names);
        }
        self.sorted_names.as_ref().unwrap()
    }
}

fn main() {
    init_logger();
    init_theme();

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

enum UiEvent {
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

    let (gui_w, gui_h) = calculate_gui_dimensions(
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

    if let Some(values) = stored_controls(&sketch_config.name) {
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
    static MIDI_MESSAGE_RX: RefCell<Option<Receiver<MidiInstruction>>> =
        RefCell::new(None);
}

fn update(app: &App, model: &mut DynamicModel, update: Update) {
    model.egui.borrow_mut().set_elapsed_time(update.since_start);
    {
        let mut egui = model.egui.borrow_mut();
        let ctx = egui.begin_frame();
        let (tx, _) = &model.event_channel;
        update_gui(
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

fn update_gui(
    app: &App,
    current_sketch_name: &mut String,
    main_window_id: window::Id,
    session_id: &mut String,
    sketch_config: &SketchConfig,
    controls: Option<&mut Controls>,
    alert_text: &mut String,
    clear_flag: &Cell<bool>,
    recording_state: &mut RecordingState,
    event_tx: &mpsc::Sender<UiEvent>,
    ctx: &egui::Context,
) {
    apply_theme(ctx);
    setup_monospaced_fonts(ctx);
    let colors = ThemeColors::current();

    let mut registry = REGISTRY.lock().unwrap();
    let sketch_names = registry.names().clone();

    egui::CentralPanel::default()
        .frame(
            egui::Frame::default()
                .fill(colors.bg_primary)
                .inner_margin(egui::Margin::same(16.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let main_window = app.window(main_window_id);

                ui.add(egui::Button::new("Save")).clicked().then(|| {
                    if let Some(window) = main_window {
                        capture_frame(
                            &window,
                            app,
                            sketch_config.name,
                            alert_text,
                        );
                    }
                });

                draw_pause_button(ui, alert_text);
                draw_adv_button(ui);
                draw_reset_button(ui, alert_text);
                draw_clear_button(ui, clear_flag, alert_text);
                draw_clear_cache_button(ui, sketch_config.name, alert_text);
                if let Some(controls) = &controls {
                    draw_copy_controls(ui, *controls, alert_text);
                } else {
                    ui.add_enabled(false, egui::Button::new("CP Ctrls"));
                }
                draw_queue_record_button(ui, recording_state, alert_text);
                draw_record_button(
                    ui,
                    sketch_config,
                    session_id,
                    recording_state,
                    alert_text,
                );

                draw_avg_fps(ui);
            });

            ui.horizontal(|ui| {
                egui::ComboBox::from_label("")
                    .selected_text(current_sketch_name.clone())
                    .show_ui(ui, |ui| {
                        for name in &sketch_names {
                            if ui
                                .selectable_label(
                                    *current_sketch_name == *name,
                                    name,
                                )
                                .clicked()
                            {
                                if *current_sketch_name != *name {
                                    if registry.get(name).is_some() {
                                        event_tx
                                            .send(UiEvent::SwitchSketch(
                                                name.clone(),
                                            ))
                                            .unwrap();
                                    }
                                }
                            }
                        }
                    });
            });

            ui.separator();

            if let Some(controls) = controls {
                draw_sketch_controls(ui, controls, sketch_config, alert_text);
            }

            draw_alert_panel(ctx, alert_text);
        });
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
        let (gui_w, gui_h) = calculate_gui_dimensions(
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

    if let Some(values) = stored_controls(&sketch_info.config.name) {
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

fn persist_controls(
    sketch_name: &str,
    controls: &Controls,
) -> Result<PathBuf, Box<dyn Error>> {
    let path = controls_storage_path(sketch_name)
        .ok_or("Could not determine the configuration directory")?;
    if let Some(parent_dir) = path.parent() {
        fs::create_dir_all(parent_dir)?;
    }
    let serialized = controls.to_serialized();
    let json = serde_json::to_string_pretty(&serialized)?;
    fs::write(&path, json)?;
    Ok(path)
}

fn stored_controls(sketch_name: &str) -> Option<ControlValues> {
    let path = controls_storage_path(sketch_name)?;
    let bytes = fs::read(path).ok()?;
    let string = str::from_utf8(&bytes).ok()?;
    let serialized = serde_json::from_str::<SerializedControls>(string).ok()?;
    Some(serialized.values)
}

fn delete_stored_controls(sketch_name: &str) -> Result<(), Box<dyn Error>> {
    let path = controls_storage_path(sketch_name)
        .ok_or("Could not determine the configuration directory")?;
    if path.exists() {
        fs::remove_file(path)?;
        info!("Deleted controls for sketch: {}", sketch_name);
    } else {
        warn!("No stored controls found for sketch: {}", sketch_name);
    }
    Ok(())
}

fn controls_storage_path(sketch_name: &str) -> Option<PathBuf> {
    if STORE_CONTROLS_CACHE_IN_PROJECT {
        return Some(
            lattice_project_root()
                .join("control-cache")
                .join(format!("{}_controls.json", sketch_name)),
        );
    }

    lattice_config_dir().map(|config_dir| {
        config_dir
            .join("Controls")
            .join(format!("{}_controls.json", sketch_name))
    })
}

fn capture_frame(
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

// Can't figure this out. EGUI controls don't seem to stack linearly.
// The more controls, the more the empty space grows between the last control and alert panel.
// Stick with per-sketch heights for now.
fn calculate_gui_dimensions(controls: Option<&mut Controls>) -> (u32, u32) {
    const HEADER_HEIGHT: u32 = 40;
    const ALERT_HEIGHT: u32 = 40;
    const CONTROL_HEIGHT: u32 = 26;
    const THRESHOLD: u32 = 5;
    const MIN_FINAL_GAP: u32 = 4;

    let controls_height = controls.map_or(0, |controls| {
        let count = controls.get_controls().len() as u32;
        let reduced_height = (CONTROL_HEIGHT as f32 * 0.95) as u32;
        let base = THRESHOLD * reduced_height;
        let remaining = count - THRESHOLD;
        base + (remaining * reduced_height)
    });

    let height = HEADER_HEIGHT + controls_height + MIN_FINAL_GAP + ALERT_HEIGHT;

    (GUI_WIDTH, height)
}

fn setup_monospaced_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    fonts
        .families
        .insert(FontFamily::Monospace, vec!["Hack".to_owned()]);

    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();

    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(10.0, FontFamily::Monospace),
    );

    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(10.0, FontFamily::Monospace),
    );

    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(12.0, FontFamily::Monospace),
    );

    ctx.set_style(style);
}

fn draw_pause_button(ui: &mut egui::Ui, alert_text: &mut String) {
    ui.add(egui::Button::new(if frame_controller::is_paused() {
        " Play"
    } else {
        "Pause"
    }))
    .clicked()
    .then(|| {
        let next_is_paused = !frame_controller::is_paused();
        frame_controller::set_paused(next_is_paused);
        info!("Paused: {}", next_is_paused);
        *alert_text =
            (if next_is_paused { "Paused" } else { "Resumed" }).into();
    });
}

fn draw_adv_button(ui: &mut egui::Ui) {
    ui.add_enabled(frame_controller::is_paused(), egui::Button::new("Adv."))
        .clicked()
        .then(|| {
            frame_controller::advance_single_frame();
        });
}

fn draw_reset_button(ui: &mut egui::Ui, alert_text: &mut String) {
    ui.add(egui::Button::new("Reset")).clicked().then(|| {
        frame_controller::reset_frame_count();
        info!("Frame count reset");
        *alert_text = "Reset".into()
    });
}

fn draw_clear_button(
    ui: &mut egui::Ui,
    clear_flag: &Cell<bool>,
    alert_text: &mut String,
) {
    ui.add(egui::Button::new("Clear")).clicked().then(|| {
        clear_flag.set(true);
        info!("Frame cleared");
        *alert_text = "Cleared".into()
    });
}

fn draw_clear_cache_button(
    ui: &mut egui::Ui,
    sketch_name: &str,
    alert_text: &mut String,
) {
    ui.add(egui::Button::new("Clear Cache")).clicked().then(|| {
        if let Err(e) = delete_stored_controls(sketch_name) {
            error!("Failed to clear controls cache: {}", e);
        } else {
            *alert_text = "Controls cache cleared".into();
        }
    });
}

fn draw_copy_controls(
    ui: &mut egui::Ui,
    controls: &Controls,
    alert_text: &mut String,
) {
    ui.add(egui::Button::new("CP Ctrls")).clicked().then(|| {
        if let Ok(mut clipboard) = Clipboard::new() {
            let serialized = controls.to_serialized();
            if let Ok(json) = serde_json::to_string_pretty(&serialized) {
                let _ = clipboard.set_text(&json);
                *alert_text = "Control state copied to clipboard".into();
            } else {
                *alert_text = "Failed to serialize controls".into();
            }
        } else {
            *alert_text = "Failed to access clipboard".into();
        }
    });
}

fn draw_queue_record_button(
    ui: &mut egui::Ui,
    recording_state: &mut RecordingState,
    alert_text: &mut String,
) {
    let button_label = if recording_state.is_queued {
        "QUEUED"
    } else {
        "Q Rec."
    };

    ui.add_enabled(
        !recording_state.is_recording && !recording_state.is_encoding,
        egui::Button::new(button_label),
    )
    .clicked()
    .then(|| {
        if recording_state.is_queued {
            recording_state.is_queued = false;
            *alert_text = "".into();
        } else {
            recording_state.is_queued = true;
            *alert_text =
                "Recording queued. Awaiting MIDI Start message.".into();
        }
    });
}

fn draw_record_button(
    ui: &mut egui::Ui,
    sketch_config: &SketchConfig,
    session_id: &str,
    recording_state: &mut RecordingState,
    alert_text: &mut String,
) {
    let button_label = if recording_state.is_recording {
        "STOP"
    } else if recording_state.is_encoding {
        "Encoding"
    } else {
        "Record"
    };

    ui.add_enabled(
        !recording_state.is_encoding,
        egui::Button::new(button_label),
    )
    .clicked()
    .then(|| {
        if let Err(e) = recording_state.toggle_recording(
            sketch_config,
            session_id,
            alert_text,
        ) {
            error!("Recording error: {}", e);
            *alert_text = format!("Recording error: {}", e);
        }
    });
}

fn draw_avg_fps(ui: &mut egui::Ui) {
    let colors = ThemeColors::current();
    let avg_fps = frame_controller::average_fps();
    ui.label("FPS:");
    ui.colored_label(colors.text_data, format!("{:.1}", avg_fps));
}

fn draw_alert_panel(ctx: &egui::Context, alert_text: &str) {
    let colors = ThemeColors::current();

    egui::TopBottomPanel::bottom("alerts")
        .frame(
            egui::Frame::default()
                .fill(colors.bg_secondary)
                .outer_margin(egui::Margin::same(6.0))
                .inner_margin(egui::Margin::same(4.0)),
        )
        .show_separator_line(false)
        .min_height(40.0)
        .show(ctx, |ui| {
            let mut text = alert_text.to_owned();
            let response = ui.add(
                egui::TextEdit::multiline(&mut text)
                    .text_color(colors.text_secondary)
                    .desired_width(ui.available_width())
                    .frame(false)
                    .margin(egui::vec2(0.0, 0.0))
                    .interactive(true),
            );

            if response.clicked() {
                if let Ok(mut clipboard) = Clipboard::new() {
                    let _ = clipboard.set_text(alert_text);
                }
            }
        });
}

fn draw_sketch_controls(
    ui: &mut egui::Ui,
    controls: &mut Controls,
    sketch_config: &SketchConfig,
    alert_text: &mut String,
) {
    let any_changed = draw_controls(controls, ui);
    if any_changed {
        if frame_controller::is_paused()
            && sketch_config.play_mode != PlayMode::ManualAdvance
        {
            frame_controller::advance_single_frame();
        }

        match persist_controls(sketch_config.name, controls) {
            Ok(path_buf) => {
                *alert_text = format!("Controls persisted at {:?}", path_buf);
                trace!("Controls persisted at {:?}", path_buf);
            }
            Err(e) => {
                error!("Failed to persist controls: {}", e);
                *alert_text = "Failed to persist controls".into();
            }
        }
    }
}
