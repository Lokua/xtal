use nannou_egui::egui;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ControlValue {
    Float(f32),
    Bool(bool),
    String(String),
}

type DisabledFn = Option<Box<dyn Fn(&Controls) -> bool>>;

#[derive(Serialize, Deserialize)]
pub enum Control {
    Slider {
        name: String,
        value: f32,
        min: f32,
        max: f32,
        step: f32,
        #[serde(skip)]
        disabled: DisabledFn,
    },
    Checkbox {
        name: String,
        value: bool,
        #[serde(skip)]
        disabled: DisabledFn,
    },
    Select {
        name: String,
        value: String,
        options: Vec<String>,
        #[serde(skip)]
        disabled: DisabledFn,
    },
    Button {
        name: String,
        #[serde(skip)]
        disabled: DisabledFn,
    },
    Separator {},
}

impl Control {
    pub fn name(&self) -> &str {
        match self {
            Control::Slider { name, .. } => name,
            Control::Checkbox { name, .. } => name,
            Control::Select { name, .. } => name,
            Control::Button { name, .. } => name,
            Control::Separator {} => "",
        }
    }

    pub fn value(&self) -> ControlValue {
        match self {
            Control::Slider { value, .. } => ControlValue::Float(*value),
            Control::Checkbox { value, .. } => ControlValue::Bool(*value),
            Control::Select { value, .. } => {
                ControlValue::String(value.clone())
            }
            Control::Button { .. } => ControlValue::Bool(false),
            Control::Separator { .. } => ControlValue::Bool(false),
        }
    }

    pub fn checkbox(name: &str, value: bool) -> Control {
        Control::Checkbox {
            name: name.to_string(),
            value,
            disabled: None,
        }
    }

    pub fn select<S>(name: &str, value: &str, options: &[S]) -> Control
    where
        S: AsRef<str>,
    {
        Control::Select {
            name: name.into(),
            value: value.into(),
            options: options.iter().map(|s| s.as_ref().to_string()).collect(),
            disabled: None,
        }
    }

    pub fn select_x<S, F>(
        name: &str,
        value: &str,
        options: &[S],
        disabled: F,
    ) -> Control
    where
        S: AsRef<str>,
        F: Fn(&Controls) -> bool + 'static,
    {
        Control::Select {
            name: name.into(),
            value: value.into(),
            options: options.iter().map(|s| s.as_ref().to_string()).collect(),
            disabled: Some(Box::new(disabled)),
        }
    }

    pub fn slider(
        name: &str,
        value: f32,
        range: (f32, f32),
        step: f32,
    ) -> Control {
        Control::Slider {
            name: name.to_string(),
            value,
            min: range.0,
            max: range.1,
            step,
            disabled: None,
        }
    }

    pub fn slider_norm(name: &str, value: f32) -> Control {
        Control::Slider {
            name: name.to_string(),
            value,
            min: 0.0,
            max: 1.0,
            step: 0.0001,
            disabled: None,
        }
    }

    pub fn slider_x<F>(
        name: &str,
        value: f32,
        range: (f32, f32),
        step: f32,
        disabled: F,
    ) -> Control
    where
        F: Fn(&Controls) -> bool + 'static,
    {
        Control::Slider {
            name: name.to_string(),
            value,
            min: range.0,
            max: range.1,
            step,
            disabled: Some(Box::new(disabled)),
        }
    }

    pub fn checkbox_x<F>(name: &str, value: bool, disabled: F) -> Control
    where
        F: Fn(&Controls) -> bool + 'static,
    {
        Control::Checkbox {
            name: name.to_string(),
            value,
            disabled: Some(Box::new(disabled)),
        }
    }

    fn is_disabled(&self, controls: &Controls) -> bool {
        match self {
            Control::Slider { disabled, .. }
            | Control::Button { disabled, .. }
            | Control::Checkbox { disabled, .. }
            | Control::Select { disabled, .. } => {
                disabled.as_ref().map_or(false, |f| f(controls))
            }
            _ => false,
        }
    }
}

pub type ControlValues = HashMap<String, ControlValue>;

#[derive(Serialize, Deserialize)]
pub struct SerializedControls {
    pub values: HashMap<String, ControlValue>,
}

#[derive(Serialize, Deserialize)]
pub struct Controls {
    controls: Vec<Control>,
    values: ControlValues,
    #[serde(skip)]
    changed: bool,
    #[serde(skip)]
    save_previous: bool,
    #[serde(skip)]
    previous_values: ControlValues,
}

impl Controls {
    pub fn new(controls: Vec<Control>) -> Self {
        let values: ControlValues = controls
            .iter()
            .map(|control| (control.name().to_string(), control.value()))
            .collect();

        Self {
            controls,
            values,
            changed: true,
            save_previous: false,
            previous_values: ControlValues::new(),
        }
    }

    pub fn with_previous(controls: Vec<Control>) -> Self {
        let values: ControlValues = controls
            .iter()
            .map(|control| (control.name().to_string(), control.value()))
            .collect();

        Self {
            controls,
            values,
            changed: true,
            save_previous: true,
            previous_values: ControlValues::new(),
        }
    }

    pub fn get_controls(&self) -> &Vec<Control> {
        &self.controls
    }

    pub fn values(&self) -> &ControlValues {
        &self.values
    }

    pub fn previous_values(&self) -> &ControlValues {
        &self.previous_values
    }

    pub fn float(&self, name: &str) -> f32 {
        self.check_contains_key(name);
        match self.values.get(name).unwrap() {
            ControlValue::Float(v) => *v,
            _ => panic!("Control '{}' exists but is not a float", name),
        }
    }

    pub fn bool(&self, name: &str) -> bool {
        self.check_contains_key(name);
        match self.values.get(name).unwrap() {
            ControlValue::Bool(v) => *v,
            _ => panic!("Control '{}' exists but is not a bool", name),
        }
    }

    pub fn string(&self, name: &str) -> String {
        self.check_contains_key(name);
        match self.values.get(name).unwrap() {
            ControlValue::String(v) => v.clone(),
            _ => panic!("Control '{}' exists but is not a string", name),
        }
    }

    fn check_contains_key(&self, key: &str) {
        if !self.values.contains_key(key) {
            panic!("Control {} does not exist", key);
        }
    }

    pub fn changed(&self) -> bool {
        self.changed
    }

    pub fn any_changed_in(&self, names: &[&str]) -> bool {
        if !self.save_previous {
            panic!(
                "Cannot check previous values when `save_previous` is false"
            );
        }

        if self.previous_values.is_empty() {
            return true;
        }

        for name in names {
            self.check_contains_key(name);
            if let Some(current) = self.values.get(*name) {
                if let Some(previous) = self.previous_values.get(*name) {
                    if current != previous {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn update_value(&mut self, name: &str, value: ControlValue) {
        if let Some(old_value) = self.values.get(name) {
            if *old_value != value {
                self.changed = true;
                self.values.insert(name.to_string(), value);
            }
        }
    }

    pub fn mark_unchanged(&mut self) {
        self.changed = false;
        if self.save_previous {
            self.previous_values = self.values.clone();
        }
    }

    pub fn get_original_config(&self, name: &str) -> Option<&Control> {
        self.controls.iter().find(|control| control.name() == name)
    }

    pub fn slider_range(&self, name: &str) -> (f32, f32) {
        self.get_original_config(name)
            .and_then(|control| match control {
                Control::Slider { min, max, .. } => Some((*min, *max)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("Unable to find range for {}", name))
    }

    pub fn to_serialized(&self) -> SerializedControls {
        let filtered_values: HashMap<String, ControlValue> = self
            .values
            .iter()
            .filter(|(key, _)| !key.is_empty())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        SerializedControls {
            values: filtered_values,
        }
    }

    pub fn from_serialized(
        serialized: SerializedControls,
        controls: Vec<Control>,
    ) -> Self {
        Self {
            controls,
            values: serialized.values,
            changed: true,
            save_previous: false,
            previous_values: HashMap::new(),
        }
    }

    pub fn add(&mut self, control: Control) {
        let name = control.name().to_string();
        let value = control.value();

        if self.values.contains_key(&name) {
            panic!("Control '{}' already exists", name);
        }

        self.controls.push(control);
        self.values.insert(name, value);
        self.changed = true;
    }
}

pub fn draw_controls(controls: &mut Controls, ui: &mut egui::Ui) -> bool {
    let mut any_changed = false;
    let mut updates = Vec::new();

    for control in &controls.controls {
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
        }
    }

    for (name, value) in updates {
        controls.update_value(&name, value);
    }

    any_changed
}
