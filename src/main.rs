use arboard::Clipboard;
use nannou::prelude::*;
use nannou_egui::egui::{self, FontDefinitions, FontFamily};
use nannou_egui::Egui;
use std::cell::{Cell, RefCell};
use std::env;
use std::error::Error;
use std::fs;
use std::sync::mpsc::Receiver;
use std::sync::{mpsc, Once};
use std::{path::PathBuf, str};

use framework::prelude::*;
use runtime::prelude::*;

pub mod framework;
pub mod runtime;
mod sketches;

const GUI_WIDTH: u32 = 560;

macro_rules! run_sketch {
    ($sketch_module:ident) => {{
        info!(
            "Loading {}",
            sketches::$sketch_module::SKETCH_CONFIG.display_name
        );
        frame_controller::ensure_controller(
            sketches::$sketch_module::SKETCH_CONFIG.fps,
        );

        nannou::app(|app| {
            model(
                app,
                sketches::$sketch_module::init_model,
                &sketches::$sketch_module::SKETCH_CONFIG,
            )
        })
        // Doesn't seem to work :(
        // .loop_mode(LoopMode::Wait)
        .update(|app, model, nannou_update| {
            update::<sketches::$sketch_module::Model>(
                app,
                model,
                nannou_update,
                sketches::$sketch_module::update,
            )
        })
        .view(|app, model, frame| {
            view::<sketches::$sketch_module::Model>(
                app,
                model,
                frame,
                sketches::$sketch_module::view,
            )
        })
        .event(|app, model, event| {
            if let Event::WindowEvent {
                id: _,
                simple: Some(event),
            } = event
            {
                if let KeyPressed(key) = event {
                    on_key_pressed(app, model, key);
                }
            }
        })
        .run();
    }};
}

fn main() {
    init_logger();
    init_theme();

    let args: Vec<String> = env::args().collect();
    let sketch_name = args.get(1).map(|s| s.as_str()).unwrap_or("template");

    match sketch_name {
        // --- GENUARY 2025
        "g25_1_horiz_vert" => run_sketch!(g25_1_horiz_vert),
        "g25_2_layers" => run_sketch!(g25_2_layers),
        "g25_5_isometric" => run_sketch!(g25_5_isometric),
        "g25_10_11_12" => run_sketch!(g25_10_11_12),
        "g25_13_triangle" => run_sketch!(g25_13_triangle),
        "g25_14_blank_and_white" => run_sketch!(g25_14_blank_and_white),
        "g25_18_wind" => run_sketch!(g25_18_wind),
        "g25_19_op_art" => run_sketch!(g25_19_op_art),
        "g25_20_23_brutal_arch" => run_sketch!(g25_20_23_brutal_arch),
        "g25_22_gradients_only" => run_sketch!(g25_22_gradients_only),
        "g25_26_symmetry" => run_sketch!(g25_26_symmetry),

        // --- SCRATCH
        "animation_script_test" => run_sketch!(animation_script_test),
        "animation_test" => run_sketch!(animation_test),
        "audio_test" => run_sketch!(audio_test),
        "bos" => run_sketch!(bos),
        "chromatic_aberration" => run_sketch!(chromatic_aberration),
        "control_script_test" => run_sketch!(control_script_test),
        "cv_test" => run_sketch!(cv_test),
        "lin_alg" => run_sketch!(lin_alg),
        "lines" => run_sketch!(lines),
        "midi_test" => run_sketch!(midi_test),
        "noise" => run_sketch!(noise),
        "osc_test" => run_sketch!(osc_test),
        "osc_transport_test" => run_sketch!(osc_transport_test),
        "perlin_loop" => run_sketch!(perlin_loop),
        "responsive_test" => run_sketch!(responsive_test),
        "sand_line" => run_sketch!(sand_line),
        "shader_experiments" => run_sketch!(shader_experiments),
        "vertical" => run_sketch!(vertical),
        "vertical_2" => run_sketch!(vertical_2),
        "wgpu_compute_test" => run_sketch!(wgpu_compute_test),
        "z_sim" => run_sketch!(z_sim),

        // --- MAIN
        "blob" => run_sketch!(blob),
        "displacement_1" => run_sketch!(displacement_1),
        "displacement_1a" => run_sketch!(displacement_1a),
        "displacement_2" => run_sketch!(displacement_2),
        "displacement_2a" => run_sketch!(displacement_2a),
        "drop" => run_sketch!(drop),
        "drop_walk" => run_sketch!(drop_walk),
        "floor_supervisor" => run_sketch!(floor_supervisor),
        "flow_field_basic" => run_sketch!(flow_field_basic),
        "heat_mask" => run_sketch!(heat_mask),
        "interference" => run_sketch!(interference),
        "sand_lines" => run_sketch!(sand_lines),
        "sierpinski_triangle" => run_sketch!(sierpinski_triangle),
        "spiral" => run_sketch!(spiral),
        "spiral_2" => run_sketch!(spiral_2),
        "template" => run_sketch!(template),
        "template_wgpu" => run_sketch!(template_wgpu),
        "wave_fract" => run_sketch!(wave_fract),
        "wgpu_displacement" => run_sketch!(wgpu_displacement),
        "wgpu_displacement_2" => run_sketch!(wgpu_displacement_2),
        "wgpu_test" => run_sketch!(wgpu_test),
        _ => {
            warn!("Sketch not found, running template");
            run_sketch!(template)
        }
    }
}

struct AppModel<S> {
    main_window_id: window::Id,
    #[allow(dead_code)]
    gui_window_id: window::Id,
    egui: Egui,
    session_id: String,
    alert_text: String,
    clear_flag: Cell<bool>,
    recording_state: RecordingState,
    sketch_model: S,
    sketch_config: &'static SketchConfig,
    gui_visible: Cell<bool>,
    main_visible: Cell<bool>,
    main_maximized: Cell<bool>,
}

fn model<S: SketchModel + 'static>(
    app: &App,
    init_sketch_model: fn(&App, WindowRect) -> S,
    sketch_config: &'static SketchConfig,
) -> AppModel<S> {
    let w = sketch_config.w as u32;
    let h = sketch_config.h as u32;

    let main_window_id = app
        .new_window()
        .title(sketch_config.display_name)
        .size(w, h)
        .build()
        .unwrap();

    let window_rect = app
        .window(main_window_id)
        .expect("Unable to get window")
        .rect();

    let mut sketch_model = init_sketch_model(app, WindowRect::new(window_rect));

    let (gui_w, gui_h) = calculate_gui_dimensions(
        sketch_model
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
        .view(view_gui::<S>)
        .resizable(false)
        .raw_event(|_app, model: &mut AppModel<S>, event| {
            model.egui.handle_raw_event(event);
        })
        .build()
        .unwrap();

    set_window_position(app, main_window_id, 0, 0);
    set_window_position(app, gui_window_id, sketch_config.w * 2, 0);

    let egui = Egui::from_window(&app.window(gui_window_id).unwrap());

    if let Some(values) = stored_controls(&sketch_config.name) {
        if let Some(controls) = sketch_model.controls() {
            for (name, value) in values.into_iter() {
                controls.update_value(&name, value);
            }
            info!("Controls restored")
        }
    }

    let session_id = generate_session_id();
    let recording_dir = frames_dir(&session_id, sketch_config.name);

    if sketch_config.play_mode != PlayMode::Loop {
        frame_controller::set_paused(true);
    }

    AppModel {
        main_window_id,
        gui_window_id,
        egui,
        session_id,
        clear_flag: Cell::new(false),
        alert_text: "".into(),
        recording_state: RecordingState::new(recording_dir.clone()),
        sketch_model,
        sketch_config,
        gui_visible: Cell::new(true),
        main_visible: Cell::new(true),
        main_maximized: Cell::new(false),
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

fn update<S: SketchModel>(
    app: &App,
    model: &mut AppModel<S>,
    update: Update,
    sketch_update_fn: fn(&App, &mut S, Update),
) {
    // GUI update must happen before frame_controller updates
    // to avoid race conditions. For example pressing Adv. will
    // set force_render=true, but if that happens at the end of update,
    // the view function will set it back before wrapped_update
    // could process it.
    model.egui.set_elapsed_time(update.since_start);
    let ctx = model.egui.begin_frame();
    update_gui(
        app,
        model.main_window_id,
        &mut model.session_id,
        model.sketch_config,
        &mut model.sketch_model,
        &mut model.alert_text,
        &mut model.clear_flag,
        &mut model.recording_state,
        &ctx,
    );

    model.sketch_model.set_window_rect(
        app.window(model.main_window_id)
            .expect("Unable to get window")
            .rect(),
    );

    frame_controller::wrapped_update(
        app,
        &mut model.sketch_model,
        update,
        sketch_update_fn,
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
                        tx.send(MidiInstruction::Start)
                            .expect("Unable to send Start instruction");
                    }
                    CONTINUE => {
                        tx.send(MidiInstruction::Continue)
                            .expect("Unable to send Continue instruction");
                    }
                    STOP => {
                        tx.send(MidiInstruction::Stop)
                            .expect("Unable to send Stop instruction");
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
                    model.sketch_config,
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
            model.sketch_config,
            &mut model.alert_text,
        );
    }
}

fn update_gui<S: SketchModel>(
    app: &App,
    main_window_id: window::Id,
    session_id: &mut String,
    sketch_config: &SketchConfig,
    sketch_model: &mut S,
    alert_text: &mut String,
    clear_flag: &Cell<bool>,
    recording_state: &mut RecordingState,
    ctx: &egui::Context,
) {
    apply_theme(ctx);
    let colors = ThemeColors::current();
    setup_monospaced_fonts(ctx);

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
                draw_copy_controls(ui, sketch_model, alert_text);
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

            ui.separator();
            draw_sketch_controls(ui, sketch_model, sketch_config, alert_text);
            draw_alert_panel(ctx, alert_text);
        });
}

fn view<S: SketchModel>(
    app: &App,
    model: &AppModel<S>,
    frame: Frame,
    sketch_view_fn: fn(&App, &S, Frame),
) {
    if model.clear_flag.get() {
        frame.clear(model.sketch_model.clear_color());
        model.clear_flag.set(false);
    }

    let did_render = frame_controller::wrapped_view(
        app,
        &model.sketch_model,
        frame,
        sketch_view_fn,
    );

    if did_render {
        frame_controller::clear_force_render();

        if model.recording_state.is_recording {
            let frame_count = model.recording_state.recorded_frames.get();
            match app.window(model.main_window_id) {
                Some(window) => match &model.recording_state.recording_dir {
                    Some(path) => {
                        let filename = format!("frame-{:06}.png", frame_count);
                        window.capture_frame(path.join(filename));
                    }
                    None => error!("Unable to capture frame {}", frame_count),
                },
                None => panic!("Unable to attain app.window handle"),
            }
            model
                .recording_state
                .recorded_frames
                .set(model.recording_state.recorded_frames.get() + 1);
        }
    }
}

fn view_gui<S: SketchModel>(_app: &App, model: &AppModel<S>, frame: Frame) {
    model.egui.draw_to_frame(&frame).unwrap();
}

fn on_key_pressed<S: SketchModel>(app: &App, model: &AppModel<S>, key: Key) {
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
    lattice_config_dir().map(|config_dir| {
        config_dir
            .join("Controls")
            .join(format!("{}_controls.json", sketch_name))
    })
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

fn draw_copy_controls<S: SketchModel>(
    ui: &mut egui::Ui,
    sketch_model: &mut S,
    alert_text: &mut String,
) {
    ui.add(egui::Button::new("CP Ctrls")).clicked().then(|| {
        if let Some(controls) = sketch_model.controls() {
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

fn draw_sketch_controls<S: SketchModel>(
    ui: &mut egui::Ui,
    sketch_model: &mut S,
    sketch_config: &SketchConfig,
    alert_text: &mut String,
) {
    if let Some(controls) = sketch_model.controls() {
        let any_changed = draw_controls(controls.as_controls(), ui);
        if any_changed {
            if frame_controller::is_paused()
                && sketch_config.play_mode != PlayMode::ManualAdvance
            {
                frame_controller::advance_single_frame();
            }

            match persist_controls(sketch_config.name, controls.as_controls()) {
                Ok(path_buf) => {
                    *alert_text =
                        format!("Controls persisted at {:?}", path_buf);
                    trace!("Controls persisted at {:?}", path_buf);
                }
                Err(e) => {
                    error!("Failed to persist controls: {}", e);
                    *alert_text = "Failed to persist controls".into();
                }
            }
        }
    }
}
