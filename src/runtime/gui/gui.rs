use arboard::Clipboard;
use nannou::prelude::*;
use nannou_egui::egui;
use std::cell::Cell;
use std::str;
use std::sync::mpsc;

use crate::framework::{frame_controller, prelude::*};
use crate::runtime::app;
use crate::runtime::prelude::*;

pub const GUI_WIDTH: u32 = 560;

pub fn init() {
    theme::init_light_dark();
}

/// The main event loop method for updating the GUI window
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
    theme::apply(ctx);
    let colors = theme::Colors::current();

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
                        app::capture_frame(
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
    let colors = theme::Colors::current();
    let avg_fps = frame_controller::average_fps();
    ui.label("FPS:");
    ui.colored_label(colors.text_data, format!("{:.1}", avg_fps));
}

fn draw_alert_panel(ctx: &egui::Context, alert_text: &str) {
    let colors = theme::Colors::current();

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
    let any_changed = controls_adapter::draw_controls(controls, ui);
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
