use arboard::Clipboard;
use dark_light;
use nannou::prelude::*;
use nannou_egui::egui::{self, Color32, FontDefinitions, FontFamily, Visuals};
use std::cell::Cell;
use std::str;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

use super::app;
use super::prelude::*;
use crate::framework::{frame_controller, prelude::*};

pub const GUI_WIDTH: u32 = 560;

pub fn draw_controls(controls: &mut Controls, ui: &mut egui::Ui) -> bool {
    let mut any_changed = false;
    let mut updates = Vec::new();

    for control in controls.get_controls() {
        let is_disabled = control.is_disabled(controls);

        match control {
            Control::Slider {
                name,
                min,
                max,
                step,
                ..
            } => {
                let mut value = controls.float(name);
                if ui
                    .add_enabled(
                        !is_disabled,
                        egui::Slider::new(&mut value, *min..=*max)
                            .text(name)
                            .step_by((*step).into()),
                    )
                    .changed()
                {
                    updates.push((name.clone(), ControlValue::Float(value)));
                    any_changed = true;
                }
            }
            Control::Checkbox { name, .. } => {
                let mut value = controls.bool(name);
                if ui
                    .add_enabled(
                        !is_disabled,
                        egui::Checkbox::new(&mut value, name),
                    )
                    .changed()
                {
                    updates.push((name.clone(), ControlValue::Bool(value)));
                    any_changed = true;
                }
            }
            Control::Select { name, options, .. } => {
                let mut value = controls.string(name);
                let name_clone = name.clone();

                // Create a disabled frame that wraps the entire ComboBox
                egui::Frame::none()
                    .multiply_with_opacity(if is_disabled { 0.4 } else { 1.0 })
                    .show(ui, |ui| {
                        ui.set_enabled(!is_disabled);
                        egui::ComboBox::from_label(name)
                            .selected_text(&value)
                            .show_ui(ui, |ui| {
                                for option in options {
                                    if ui
                                        .selectable_value(
                                            &mut value,
                                            option.clone(),
                                            option,
                                        )
                                        .changed()
                                    {
                                        updates.push((
                                            name_clone.clone(),
                                            ControlValue::String(value.clone()),
                                        ));
                                        any_changed = true;
                                    }
                                }
                            });
                    });
            }
            Control::Button { name, .. } => {
                if ui
                    .add_enabled(!is_disabled, egui::Button::new(name))
                    .clicked()
                {
                    // Handle click
                }
            }
            Control::Separator {} => {
                ui.separator();
            }
            Control::DynamicSeparator { .. } => {
                ui.separator();
            }
        }
    }

    for (name, value) in updates {
        controls.update_value(&name, value);
    }

    any_changed
}

static USE_LIGHT_THEME: AtomicBool = AtomicBool::new(false);

pub struct ThemeColors {
    pub bg_primary: Color32,
    pub bg_secondary: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub button_bg: Color32,
    pub button_active: Color32,
    pub accent: Color32,
    pub text_data: Color32,
    pub shadow_color: Color32,
}

impl ThemeColors {
    pub fn current() -> Self {
        if USE_LIGHT_THEME.load(Ordering::Relaxed) {
            Self::light_theme()
        } else {
            Self::dark_theme()
        }
    }

    fn light_theme() -> Self {
        Self {
            bg_primary: Color32::from_gray(245),
            // e.g. alert_text bg
            bg_secondary: Color32::from_gray(220),
            text_primary: Color32::from_gray(20),
            text_secondary: Color32::from_gray(40),
            // Also controls slider track, handle, number inputs
            button_bg: Color32::from_gray(190),
            button_active: Color32::from_gray(210),
            accent: Color32::from_rgb(20, 138, 242),
            text_data: Color32::from_rgb(0, 100, 0),
            shadow_color: Color32::from_black_alpha(32),
        }
    }

    fn dark_theme() -> Self {
        Self {
            bg_primary: Color32::from_gray(3),
            bg_secondary: Color32::from_gray(2),
            text_primary: Color32::from_gray(230),
            text_secondary: Color32::from_gray(180),
            button_bg: Color32::from_gray(10),
            button_active: Color32::from_gray(20),
            accent: Color32::from_rgb(20, 138, 242),
            text_data: Color32::from_rgb(0, 255, 0),
            shadow_color: Color32::from_black_alpha(128),
        }
    }
}

pub fn apply_theme(ctx: &egui::Context) {
    let colors = ThemeColors::current();
    let mut style = (*ctx.style()).clone();

    let mut visuals = if USE_LIGHT_THEME.load(Ordering::Relaxed) {
        Visuals::light()
    } else {
        Visuals::dark()
    };

    visuals.button_frame = true;
    visuals.widgets.noninteractive.bg_fill = colors.bg_secondary;
    visuals.widgets.inactive.bg_fill = colors.button_bg;
    visuals.widgets.inactive.weak_bg_fill = colors.button_bg;
    visuals.widgets.hovered.bg_fill = colors.button_active;
    visuals.widgets.active.bg_fill = colors.accent;
    visuals.widgets.noninteractive.fg_stroke.color = colors.text_primary;
    visuals.widgets.noninteractive.bg_stroke.color = colors.button_bg;
    visuals.widgets.inactive.fg_stroke.color = colors.text_primary;
    visuals.widgets.inactive.bg_stroke.color = colors.button_active;
    visuals.widgets.hovered.fg_stroke.color = colors.text_primary;
    visuals.widgets.hovered.bg_stroke.color = colors.accent;
    visuals.widgets.active.fg_stroke.color = colors.text_primary;
    visuals.widgets.active.bg_stroke.color = colors.accent;
    visuals.selection.bg_fill = colors.accent.linear_multiply(0.4);
    visuals.selection.stroke.color = colors.accent;
    visuals.window_fill = colors.bg_primary;
    visuals.panel_fill = colors.bg_secondary;
    visuals.popup_shadow.color = colors.shadow_color;

    style.visuals = visuals;
    style.spacing.slider_width = 320.0;

    ctx.set_style(style);
}

pub fn init_theme() {
    let is_light = matches!(dark_light::detect(), dark_light::Mode::Light);
    USE_LIGHT_THEME.store(is_light, Ordering::Relaxed);
}

pub fn update_gui(
    app: &App,
    current_sketch_name: &mut String,
    main_window_id: window::Id,
    session_id: &mut String,
    sketch_config: &SketchConfig,
    controls: Option<&mut Controls>,
    alert_text: &mut String,
    clear_flag: &Cell<bool>,
    recording_state: &mut RecordingState,
    event_tx: &mpsc::Sender<app::UiEvent>,
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
                        super::app::capture_frame(
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
                                            .send(app::UiEvent::SwitchSketch(
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

pub fn calculate_gui_dimensions(controls: Option<&mut Controls>) -> (u32, u32) {
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
        if let Err(e) = storage::delete_stored_controls(sketch_name) {
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

        match storage::persist_controls(sketch_config.name, controls) {
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
