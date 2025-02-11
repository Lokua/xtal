use dark_light;
use nannou_egui::egui::{self, Color32, Visuals};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::framework::prelude::*;

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
