use nannou_egui::egui;

use crate::framework::prelude::*;
use crate::runtime::prelude::MapMode;

use super::theme::DISABLED_OPACITY;

pub fn draw_controls(
    ui: &mut egui::Ui,
    controls: &mut UiControls,
    map_mode: &MapMode,
) -> bool {
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
                let has_mapping = map_mode.has(name);
                let is_disabled = is_disabled || has_mapping;
                let mut value = controls.float(name);

                let slider = ui.add_enabled(
                    !is_disabled,
                    egui::Slider::new(&mut value, *min..=*max)
                        .text(ternary!(
                            has_mapping,
                            format!("*{}*", name),
                            name.to_string()
                        ))
                        .step_by((*step).into()),
                );

                if slider.changed() {
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

                egui::Frame::none()
                    .multiply_with_opacity(ternary!(
                        is_disabled,
                        DISABLED_OPACITY,
                        1.0
                    ))
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
            Control::Separator {} | Control::DynamicSeparator { .. } => {
                ui.separator();
            }
        }
    }

    for (name, value) in updates {
        controls.update_value(&name, value);
    }

    any_changed
}
