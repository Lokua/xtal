use nannou_egui::egui;

use crate::framework::prelude::*;

pub fn draw_controls(controls: &mut UiControls, ui: &mut egui::Ui) -> bool {
    let mut any_changed = false;
    let mut updates = Vec::new();

    for control in controls.configs() {
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
