use arboard::Clipboard;
use nannou_egui::egui;
use std::str;

use crate::framework::{frame_controller, prelude::*};
use crate::runtime::app;
use crate::runtime::prelude::*;

pub const GUI_WIDTH: u32 = 560;

pub fn init() {
    theme::init_light_dark();
}

pub fn update(
    session_id: &mut String,
    sketch_config: &SketchConfig,
    controls: Option<&mut Controls>,
    alert_text: &mut String,
    recording_state: &mut RecordingState,
    event_tx: &app::AppEventSender,
    ctx: &egui::Context,
) {
    theme::apply(ctx);
    let colors = theme::Colors::current();

    let registry = REGISTRY.read().unwrap();
    let sketch_names = registry.names().clone();

    egui::CentralPanel::default()
        .frame(
            egui::Frame::default()
                .fill(colors.bg_primary)
                .inner_margin(egui::Margin::same(16.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                draw_save_button(ui, event_tx);
                draw_pause_button(ui, event_tx);
                draw_adv_button(ui);
                draw_reset_button(ui, event_tx);
                draw_clear_button(ui, event_tx);
                draw_clear_cache_button(ui, sketch_config, event_tx);
                if let Some(controls) = &controls {
                    draw_copy_controls(ui, *controls, event_tx);
                } else {
                    ui.add_enabled(false, egui::Button::new("CP Ctrls"));
                }
                draw_queue_record_button(ui, recording_state, event_tx);
                draw_record_button(
                    ui,
                    sketch_config,
                    session_id,
                    recording_state,
                    event_tx,
                );
                draw_avg_fps(ui);
            });

            ui.horizontal(|ui| {
                draw_sketch_selector(
                    ui,
                    recording_state.is_recording,
                    sketch_config,
                    &sketch_names,
                    &registry,
                    event_tx,
                );
            });

            ui.separator();

            if let Some(controls) = controls {
                draw_sketch_controls(ui, controls, sketch_config, event_tx);
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
        let count = controls.items().len() as u32;
        let reduced_height = (CONTROL_HEIGHT as f32 * 0.95) as u32;
        let base = THRESHOLD * reduced_height;
        let remaining = count - THRESHOLD;
        base + (remaining * reduced_height)
    });

    let height = HEADER_HEIGHT + controls_height + MIN_FINAL_GAP + ALERT_HEIGHT;

    (GUI_WIDTH, height)
}

fn draw_save_button(ui: &mut egui::Ui, event_tx: &app::AppEventSender) {
    ui.add(egui::Button::new("Save")).clicked().then(|| {
        event_tx.send(app::AppEvent::CaptureFrame);
    });
}

fn draw_pause_button(ui: &mut egui::Ui, event_tx: &app::AppEventSender) {
    ui.add(egui::Button::new(ternary!(
        frame_controller::is_paused(),
        " Play",
        "Pause"
    )))
    .clicked()
    .then(|| {
        let next_is_paused = !frame_controller::is_paused();
        frame_controller::set_paused(next_is_paused);
        info!("Paused: {}", next_is_paused);
        event_tx.alert(ternary!(next_is_paused, "Paused", "Resumed"))
    });
}

fn draw_adv_button(ui: &mut egui::Ui) {
    ui.add_enabled(frame_controller::is_paused(), egui::Button::new("Adv."))
        .clicked()
        .then(|| {
            frame_controller::advance_single_frame();
        });
}

fn draw_reset_button(ui: &mut egui::Ui, event_tx: &app::AppEventSender) {
    ui.add(egui::Button::new("Reset")).clicked().then(|| {
        event_tx.alert("Reset");
    });
}

fn draw_clear_button(ui: &mut egui::Ui, event_tx: &app::AppEventSender) {
    ui.add(egui::Button::new("Clear")).clicked().then(|| {
        event_tx.send(app::AppEvent::ClearFlag(true));
        event_tx.alert("Cleared");
        info!("Frame cleared");
    });
}

fn draw_clear_cache_button(
    ui: &mut egui::Ui,
    sketch_config: &SketchConfig,
    event_tx: &app::AppEventSender,
) {
    ui.add(egui::Button::new("Clear Cache")).clicked().then(|| {
        if let Err(e) = storage::delete_stored_controls(sketch_config.name) {
            error!("Failed to clear controls cache: {}", e);
        } else {
            event_tx.alert("Controls cache cleared");
        }
    });
}

fn draw_copy_controls(
    ui: &mut egui::Ui,
    controls: &Controls,
    event_tx: &app::AppEventSender,
) {
    ui.add(egui::Button::new("CP Ctrls")).clicked().then(|| {
        if let Ok(mut clipboard) = Clipboard::new() {
            let serialized = controls.to_serialized();
            if let Ok(json) = serde_json::to_string_pretty(&serialized) {
                let _ = clipboard.set_text(&json);
                event_tx.alert("Control state copied to clipboard");
            } else {
                event_tx.alert("Failed to serialize controls");
            }
        } else {
            event_tx.alert("Failed to access clipboard");
        }
    });
}

fn draw_queue_record_button(
    ui: &mut egui::Ui,
    recording_state: &mut RecordingState,
    event_tx: &app::AppEventSender,
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
        // TODO: move to AppEvent
        if recording_state.is_queued {
            recording_state.is_queued = false;
            event_tx.alert("");
        } else {
            recording_state.is_queued = true;
            event_tx.alert("Recording queued. Awaiting MIDI Start message");
        }
    });
}

fn draw_record_button(
    ui: &mut egui::Ui,
    sketch_config: &SketchConfig,
    session_id: &str,
    recording_state: &mut RecordingState,
    event_tx: &app::AppEventSender,
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
        // TODO: move to AppEvent
        match recording_state.toggle_recording(sketch_config, session_id) {
            Ok(message) => {
                event_tx.alert(message);
            }
            Err(e) => {
                let message = format!("Recording error: {}", e);
                event_tx.alert(message.clone());
                error!("{}", message);
            }
        }
    });
}

fn draw_avg_fps(ui: &mut egui::Ui) {
    let colors = theme::Colors::current();
    let avg_fps = frame_controller::average_fps();
    ui.label("FPS:");
    ui.colored_label(colors.text_data, format!("{:.1}", avg_fps));
}

fn draw_sketch_selector(
    ui: &mut egui::Ui,
    is_disabled: bool,
    sketch_config: &SketchConfig,
    sketch_names: &Vec<String>,
    registry: &SketchRegistry,
    event_tx: &app::AppEventSender,
) {
    egui::Frame::none()
        .multiply_with_opacity(ternary!(is_disabled, 0.4, 1.0))
        .show(ui, |ui| {
            ui.set_enabled(!is_disabled);
            egui::ComboBox::from_label("")
                .selected_text(sketch_config.name)
                .show_ui(ui, |ui| {
                    for name in sketch_names {
                        if ui
                            .selectable_label(sketch_config.name == name, name)
                            .clicked()
                        {
                            if sketch_config.name != name {
                                if registry.get(name).is_some() {
                                    event_tx.send(app::AppEvent::SwitchSketch(
                                        name.clone(),
                                    ));
                                }
                            }
                        }
                    }
                });
        });
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
    event_tx: &app::AppEventSender,
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
                let message = format!("Controls persisted at {:?}", path_buf);
                event_tx.alert(message.clone());
                trace!("{}", message);
            }
            Err(e) => {
                let message = format!("Failed to persist controls: {}", e);
                event_tx.alert(message.clone());
                error!("{}", message);
            }
        }
    }
}
