use arboard::Clipboard;
use nannou_egui::egui;
use std::str;

use crate::framework::{frame_controller, prelude::*};
use crate::runtime::app;
use crate::runtime::prelude::*;

use super::theme::DISABLED_OPACITY;

pub const GUI_WIDTH: u32 = 538;

// Not sure this is really needed
pub const DRAW_CLEAR_STORED_CONTROLS_BUTTON: bool = false;

pub fn init() {
    theme::init_light_dark();
}

pub fn update(
    sketch_config: &SketchConfig,
    controls: Option<&mut UiControls>,
    alert_text: &mut String,
    perf_mode: &mut bool,
    tap_tempo: &mut bool,
    bpm: f32,
    transition_time: f32,
    recording_state: &mut RecordingState,
    event_tx: &app::AppEventSender,
    ctx: &egui::Context,
) {
    theme::apply(ctx);
    let colors = theme::Colors::current();

    let registry = REGISTRY.read().unwrap();
    let sketch_names = registry.names().clone();
    let is_paused = frame_controller::is_paused();

    egui::CentralPanel::default()
        .frame(
            egui::Frame::default()
                .fill(colors.bg_primary)
                .inner_margin(egui::Margin::same(16.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                draw_save_button(ui, event_tx);
                ui.separator();
                draw_pause_button(ui, is_paused, event_tx);
                draw_adv_button(ui, is_paused, event_tx);
                draw_reset_button(ui, event_tx);
                ui.separator();
                draw_clear_button(ui, event_tx);
                ui.separator();
                draw_queue_record_button(
                    ui,
                    recording_state.is_queued,
                    recording_state.is_recording || recording_state.is_encoding,
                    event_tx,
                );
                draw_record_button(
                    ui,
                    recording_state.is_recording,
                    recording_state.is_encoding,
                    event_tx,
                );
                ui.separator();
                draw_avg_fps(ui);
            });

            ui.separator();

            ui.horizontal(|ui| {
                draw_sketch_selector(
                    ui,
                    recording_state.is_recording,
                    &sketch_config.name,
                    &sketch_names,
                    event_tx,
                );
                draw_perf_mode_checkbox(ui, perf_mode, event_tx);
                ui.separator();
                draw_bpm(ui, bpm);
                draw_tap_tempo_checkbox(ui, tap_tempo, event_tx);
                ui.separator();
                draw_transition_time_selector(
                    ui,
                    transition_time,
                    controls.is_none(),
                    event_tx,
                );
                ui.separator();
                draw_send_midi_button(ui, event_tx);
                ui.separator();
                draw_save_controls_button(ui, controls.is_none(), event_tx);
            });

            ui.separator();

            if let Some(controls) = controls {
                draw_sketch_controls(ui, controls);
            }

            draw_alert_panel(ctx, alert_text);
        });
}

pub fn calculate_gui_dimensions(
    controls: Option<&mut UiControls>,
) -> (u32, u32) {
    const HEADER_HEIGHT: u32 = 40;
    const ALERT_HEIGHT: u32 = 40;
    const CONTROL_HEIGHT: u32 = 26;
    const THRESHOLD: u32 = 5;
    const MIN_FINAL_GAP: u32 = 4;

    let controls_height = controls.map_or(0, |controls| {
        let count = controls.configs().len() as u32;
        let reduced_height = (CONTROL_HEIGHT as f32 * 0.95) as u32;
        let base = THRESHOLD * reduced_height;
        let remaining = count - THRESHOLD;
        base + (remaining * reduced_height)
    });

    let height = HEADER_HEIGHT + controls_height + MIN_FINAL_GAP + ALERT_HEIGHT;

    (GUI_WIDTH, height)
}

fn draw_save_button(ui: &mut egui::Ui, event_tx: &app::AppEventSender) {
    ui.add(egui::Button::new("Image")).clicked().then(|| {
        event_tx.send(app::AppEvent::CaptureFrame);
    });
}

fn draw_pause_button(
    ui: &mut egui::Ui,
    is_paused: bool,
    event_tx: &app::AppEventSender,
) {
    ui.add(
        egui::Button::new(ternary!(is_paused, "Play", "Pause"))
            .min_size(egui::vec2(40.0, 0.0)),
    )
    .clicked()
    .then(|| {
        event_tx.send(app::AppEvent::TogglePlay);
    });
}

fn draw_adv_button(
    ui: &mut egui::Ui,
    is_paused: bool,
    event_tx: &app::AppEventSender,
) {
    ui.add_enabled(is_paused, egui::Button::new("Advance"))
        .clicked()
        .then(|| {
            event_tx.send(app::AppEvent::AdvanceSingleFrame);
        });
}

fn draw_reset_button(ui: &mut egui::Ui, event_tx: &app::AppEventSender) {
    ui.add(egui::Button::new("Reset")).clicked().then(|| {
        event_tx.send(app::AppEvent::Reset);
    });
}

fn draw_clear_button(ui: &mut egui::Ui, event_tx: &app::AppEventSender) {
    ui.add(egui::Button::new("Clear Buf")).clicked().then(|| {
        event_tx.send(app::AppEvent::ClearNextFrame);
        event_tx.alert("Cleared");
        info!("Frame cleared");
    });
}

fn draw_queue_record_button(
    ui: &mut egui::Ui,
    is_queued: bool,
    is_disabled: bool,
    event_tx: &app::AppEventSender,
) {
    let button_label = ternary!(is_queued, "QUEUED", "Q Record");
    ui.add_enabled(
        !is_disabled,
        egui::Button::new(button_label).min_size(egui::vec2(60.0, 0.0)),
    )
    .clicked()
    .then(|| {
        event_tx.send(app::AppEvent::QueueRecord);
    });
}

fn draw_record_button(
    ui: &mut egui::Ui,
    is_recording: bool,
    is_encoding: bool,
    event_tx: &app::AppEventSender,
) {
    let button_label = if is_recording {
        "STOP"
    } else if is_encoding {
        "Encoding"
    } else {
        "Record"
    };

    ui.add_enabled(
        !is_encoding,
        egui::Button::new(button_label).min_size(egui::vec2(50.0, 0.0)),
    )
    .clicked()
    .then(|| {
        event_tx.send(app::AppEvent::Record);
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
    selected_sketch_name: &str,
    sketch_names: &Vec<String>,
    event_tx: &app::AppEventSender,
) {
    let max_length = 15;
    let display_name = if selected_sketch_name.len() > max_length {
        format!("{}...", &selected_sketch_name[..max_length])
    } else {
        selected_sketch_name.to_string()
    };

    egui::Frame::none()
        .multiply_with_opacity(ternary!(is_disabled, DISABLED_OPACITY, 1.0))
        .show(ui, |ui| {
            ui.set_enabled(!is_disabled);
            egui::ComboBox::from_id_source("sketch_selector")
                .selected_text(display_name)
                .width(138.0)
                .show_ui(ui, |ui| {
                    ui.set_min_width(200.0);

                    for name in sketch_names {
                        if ui
                            .selectable_label(
                                selected_sketch_name == name,
                                name,
                            )
                            .clicked()
                        {
                            event_tx.send(app::AppEvent::SwitchSketch(
                                name.clone(),
                            ));
                        }
                    }
                });
        });
}

fn draw_perf_mode_checkbox(
    ui: &mut egui::Ui,
    perf_mode: &mut bool,
    event_tx: &app::AppEventSender,
) {
    if ui.add(egui::Checkbox::new(perf_mode, "Perf")).changed() {
        event_tx.send(app::AppEvent::TogglePerfMode(perf_mode.clone()))
    }
}

fn draw_bpm(ui: &mut egui::Ui, bpm: f32) {
    let colors = theme::Colors::current();
    ui.label("BPM:");
    ui.colored_label(colors.text_data, format!("{:.1}", bpm));
}

fn draw_tap_tempo_checkbox(
    ui: &mut egui::Ui,
    tap_tempo: &mut bool,
    event_tx: &app::AppEventSender,
) {
    if ui.add(egui::Checkbox::new(tap_tempo, "Tap")).changed() {
        event_tx.send(app::AppEvent::ToggleTapTempo(tap_tempo.clone()))
    }
}

fn draw_transition_time_selector(
    ui: &mut egui::Ui,
    transition_time: f32,
    is_disabled: bool,
    event_tx: &app::AppEventSender,
) {
    egui::Frame::none()
        .multiply_with_opacity(ternary!(is_disabled, DISABLED_OPACITY, 1.0))
        .show(ui, |ui| {
            ui.set_enabled(!is_disabled);

            egui::ComboBox::from_id_source("transition_time")
                .selected_text(transition_time.to_string())
                .width(58.0)
                .show_ui(ui, |ui| {
                    ui.set_min_width(78.0);

                    for time in control_hub::TRANSITION_TIMES {
                        if ui
                            .selectable_label(
                                transition_time == time,
                                time.to_string(),
                            )
                            .clicked()
                        {
                            event_tx
                                .send(app::AppEvent::SetTransitionTime(time));
                        }
                    }
                });
        });
}

fn draw_send_midi_button(ui: &mut egui::Ui, event_tx: &app::AppEventSender) {
    ui.add(egui::Button::new("MIDI")).clicked().then(|| {
        event_tx.send(app::AppEvent::SendMidi);
    });
}

fn draw_save_controls_button(
    ui: &mut egui::Ui,
    is_disabled: bool,
    event_tx: &app::AppEventSender,
) {
    ui.add_enabled(!is_disabled, egui::Button::new("Save"))
        .clicked()
        .then(|| {
            event_tx.send(app::AppEvent::SaveControls);
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

fn draw_sketch_controls(ui: &mut egui::Ui, controls: &mut UiControls) {
    controls_adapter::draw_controls(controls, ui);
}
