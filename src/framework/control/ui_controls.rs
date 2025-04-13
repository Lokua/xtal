//! Control sketch parameters with GUI controls.
//!
//! Sketches do not need to interact with this module directly - see
//! [`ControlHub`].
use std::fmt::{self, Debug};

use serde::{Deserialize, Serialize};

use crate::framework::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum ControlValue {
    Float(f32),
    Bool(bool),
    String(String),
}

impl ControlValue {
    pub fn as_float(&self) -> Option<f32> {
        if let ControlValue::Float(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let ControlValue::Bool(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        if let ControlValue::String(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl From<f32> for ControlValue {
    fn from(value: f32) -> Self {
        Self::Float(value)
    }
}
impl From<bool> for ControlValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}
impl From<String> for ControlValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

/// Used by [`UiControls`] to compute if a [`Control`] should be disabled or not
/// based on the value of other controls
///
/// # Example
/// ```rust
/// let controls = Controls::New(&[
///     Control::Checkbox {
///         name: "animate_phase".to_string(),
///         value: true,
///         disabled: None,
///     },
///     Control::Slider {
///         name: "phase",
///         value: 0.0,
///         min: 0.0,
///         max: 1.0,
///         // Slider will automatically become disabled when animate_phase is true
///         disabled: Some(Box::new(|controls| controls.bool("animate_phase"))),
///     };
/// ])
/// ```
pub type DisabledFn = Option<Box<dyn Fn(&UiControls) -> bool>>;

pub enum Control {
    Slider {
        name: String,
        value: f32,
        min: f32,
        max: f32,
        step: f32,
        disabled: DisabledFn,
    },
    Checkbox {
        name: String,
        value: bool,
        disabled: DisabledFn,
    },
    Select {
        name: String,
        value: String,
        options: Vec<String>,
        disabled: DisabledFn,
    },
    Separator {},
    DynamicSeparator {
        name: String,
    },
}

impl Control {
    pub fn name(&self) -> &str {
        match self {
            Control::Slider { name, .. } => name,
            Control::Checkbox { name, .. } => name,
            Control::Select { name, .. } => name,
            Control::Separator {} => "",
            Control::DynamicSeparator { name } => name,
        }
    }

    pub fn value(&self) -> ControlValue {
        match self {
            Control::Slider { value, .. } => ControlValue::Float(*value),
            Control::Checkbox { value, .. } => ControlValue::Bool(*value),
            Control::Select { value, .. } => {
                ControlValue::String(value.clone())
            }
            Control::Separator { .. } => ControlValue::Bool(false),
            Control::DynamicSeparator { .. } => ControlValue::Bool(false),
        }
    }

    pub fn checkbox(name: &str, value: bool) -> Control {
        Control::Checkbox {
            name: name.to_string(),
            value,
            disabled: None,
        }
    }

    pub fn checkbox_x<F>(name: &str, value: bool, disabled: F) -> Control
    where
        F: Fn(&UiControls) -> bool + 'static,
    {
        Control::Checkbox {
            name: name.to_string(),
            value,
            disabled: Some(Box::new(disabled)),
        }
    }

    pub fn dynamic_separator() -> Control {
        Control::DynamicSeparator {
            name: format!("__dynamic_slider__{}", uuid(11)),
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
        F: Fn(&UiControls) -> bool + 'static,
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

    /// Convenience version of [`Self::slider`] with default [0.0, 1.0] range.
    pub fn slider_n(name: &str, value: f32) -> Control {
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
        F: Fn(&UiControls) -> bool + 'static,
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

    pub fn is_disabled(&self, controls: &UiControls) -> bool {
        match self {
            Control::Slider { disabled, .. }
            | Control::Checkbox { disabled, .. }
            | Control::Select { disabled, .. } => {
                disabled.as_ref().is_some_and(|f| f(controls))
            }
            _ => false,
        }
    }

    pub fn variant_string(&self) -> String {
        (match self {
            Self::Checkbox { .. } => "Checkbox",
            Self::DynamicSeparator { .. } => "DynamicSeparator",
            Self::Select { .. } => "Select",
            Self::Separator {} => "Separator",
            Self::Slider { .. } => "Slider",
        })
        .to_string()
    }

    pub fn is_separator(&self) -> bool {
        matches!(self, Self::Separator { .. } | Self::DynamicSeparator { .. })
    }
}

impl Clone for Control {
    fn clone(&self) -> Self {
        match self {
            Control::Slider {
                name,
                value,
                min,
                max,
                step,
                disabled: _,
            } => Control::Slider {
                name: name.clone(),
                value: *value,
                min: *min,
                max: *max,
                step: *step,
                disabled: None,
            },
            Control::Checkbox {
                name,
                value,
                disabled: _,
            } => Control::Checkbox {
                name: name.clone(),
                value: *value,
                disabled: None,
            },
            Control::Select {
                name,
                value,
                options,
                disabled: _,
            } => Control::Select {
                name: name.clone(),
                value: value.clone(),
                options: options.clone(),
                disabled: None,
            },
            Control::Separator {} => Control::Separator {},
            Control::DynamicSeparator { name } => {
                Control::DynamicSeparator { name: name.clone() }
            }
        }
    }
}

impl fmt::Debug for Control {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Control::Slider {
                name,
                value,
                min,
                max,
                step,
                ..
            } => f
                .debug_struct("Slider")
                .field("name", name)
                .field("value", value)
                .field("min", min)
                .field("max", max)
                .field("step", step)
                .finish(),
            Control::Checkbox { name, value, .. } => f
                .debug_struct("Checkbox")
                .field("name", name)
                .field("value", value)
                .finish(),
            Control::Select {
                name,
                value,
                options,
                ..
            } => f
                .debug_struct("Select")
                .field("name", name)
                .field("value", value)
                .field("options", options)
                .finish(),
            Control::Separator {} => f.debug_struct("Separator").finish(),
            Control::DynamicSeparator { name, .. } => f
                .debug_struct("DynamicSeparator")
                .field("name", name)
                .finish(),
        }
    }
}

pub type ControlValues = HashMap<String, ControlValue>;

/// A generic abstraction over UI controls that sketches can directly interact
/// with without being coupled to a specific UI framework. The original version
/// of Lattice used Egui for this purpose but has since moved on to using a
/// WebView for greater UI flexibility
#[derive(Clone)]
pub struct UiControls {
    /// Holds the original Control references and their default values - values
    /// are not updated!
    configs: Vec<Control>,
    values: ControlValues,
    change_tracker: ChangeTracker,
}

impl UiControls {
    pub fn new(controls: Vec<Control>) -> Self {
        let values: ControlValues = controls
            .iter()
            .map(|control| (control.name().to_string(), control.value()))
            .collect();

        Self {
            configs: controls,
            values,
            change_tracker: ChangeTracker::new(false),
        }
    }

    pub fn with_previous(controls: Vec<Control>) -> Self {
        let values: ControlValues = controls
            .iter()
            .map(|control| (control.name().to_string(), control.value()))
            .collect();

        Self {
            configs: controls,
            values,
            change_tracker: ChangeTracker::new(true),
        }
    }

    pub fn extend(&mut self, controls: Vec<Control>) {
        let values: ControlValues = controls
            .iter()
            .map(|control| (control.name().to_string(), control.value()))
            .collect();

        self.values.extend(values);
        self.configs.extend(controls);
    }

    pub fn configs(&self) -> &Vec<Control> {
        &self.configs
    }

    pub fn configs_mut(&mut self) -> &mut Vec<Control> {
        &mut self.configs
    }

    pub fn config(&self, name: &str) -> Option<&Control> {
        self.configs.iter().find(|c| c.name() == name)
    }

    pub fn values(&self) -> &ControlValues {
        &self.values
    }

    pub fn values_mut(&mut self) -> &mut ControlValues {
        &mut self.values
    }

    pub fn has(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }

    /// Same as `float`, only will try to coerce a possibly existing Checkbox's
    /// bool to 0.0 or 1.0 or a Select's string into its matching option index
    /// (useful in shader context where you are only passing in banks of
    /// `vec4<f32>` to uniforms)
    pub fn get(&self, name: &str) -> f32 {
        self.get_optional(name).unwrap_or_else(|| {
            warn_once!(
                "`get` could not retrieve a value for `{}`. Returning 0.0",
                name
            );
            0.0
        })
    }

    /// The same as [`UiControls::get`] yet doesn't return a fallback value of
    /// 0.0 in the case of invalids. This is for internal use.
    pub fn get_optional(&self, name: &str) -> Option<f32> {
        if let Some(value) =
            self.values.get(name).and_then(ControlValue::as_float)
        {
            return Some(value);
        }

        match self.config(name) {
            Some(Control::Checkbox { .. }) => Some(self.bool_as_f32(name)),
            Some(Control::Select { .. }) => Some(self.string_as_f32(name)),
            _ => None,
        }
    }

    pub fn float(&self, name: &str) -> f32 {
        self.values
            .get(name)
            .and_then(ControlValue::as_float)
            .unwrap_or_else(|| {
                error!("No float for `{}`. Returning 0.0.", name);
                0.0
            })
    }

    pub fn bool(&self, name: &str) -> bool {
        self.values
            .get(name)
            .and_then(ControlValue::as_bool)
            .unwrap_or_else(|| {
                error!("No bool for `{}`. Returning false.", name);
                false
            })
    }

    /// Converts checkbox value into 0.0 or 1.0 (useful in shader context)
    pub fn bool_as_f32(&self, name: &str) -> f32 {
        bool_to_f32(self.bool(name))
    }

    pub fn string(&self, name: &str) -> String {
        self.values
            .get(name)
            .and_then(ControlValue::as_string)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| {
                error!(
                    "No String for Control named `{}`. Returning empty.",
                    name
                );
                "".to_string()
            })
    }

    /// Returns the matching option index of a select as f32 (useful in shader
    /// context)
    pub fn string_as_f32(&self, name: &str) -> f32 {
        let value = self.string(name);
        if let Some(Control::Select { options, .. }) = self.config(name) {
            return options.iter().position(|x| *x == value).unwrap_or(0)
                as f32;
        }
        0.0
    }

    pub fn changed(&self) -> bool {
        self.change_tracker.changed()
    }
    pub fn any_changed_in(&self, names: &[&str]) -> bool {
        self.change_tracker.any_changed_in(names, &self.values)
    }
    pub fn mark_unchanged(&mut self) {
        self.change_tracker.mark_unchanged(&self.values);
    }
    pub fn mark_changed(&mut self) {
        self.change_tracker.mark_changed();
    }

    pub fn update_value(&mut self, name: &str, value: ControlValue) {
        if let Some(old_value) = self.values.get(name) {
            if *old_value != value {
                self.change_tracker.mark_changed();
                self.values.insert(name.to_string(), value);
            }
        }
    }

    pub fn disabled(&self, name: &str) -> Option<bool> {
        self.config(name).map(|c| c.is_disabled(self))
    }

    pub fn slider_range(&self, name: &str) -> Option<(f32, f32)> {
        self.config(name).and_then(|control| match control {
            Control::Slider { min, max, .. } => Some((*min, *max)),
            _ => {
                error!(
                    "Unable to find a Control definition for Slider `{}`",
                    name
                );
                None
            }
        })
    }

    pub fn add(&mut self, control: Control) {
        let name = control.name().to_string();
        let value = control.value();

        if let Some(index) = self.configs.iter().position(|c| c.name() == name)
        {
            self.configs[index] = control;
        } else {
            self.configs.push(control);
        }

        self.values.insert(name, value);
        self.change_tracker.mark_changed();
    }

    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&Control) -> bool,
    {
        self.configs.retain(f);
    }
}

impl Default for UiControls {
    fn default() -> Self {
        UiControls::new(vec![])
    }
}

impl fmt::Debug for UiControls {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("Controls");
        debug_struct.field("controls", &self.configs);
        debug_struct.field("values", &self.values);
        debug_struct.finish()
    }
}

#[derive(Default)]
pub struct UiControlBuilder {
    controls: Vec<Control>,
}

impl UiControlBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn control(mut self, control: Control) -> Self {
        self.controls.push(control);
        self
    }

    pub fn checkbox(
        self,
        name: &str,
        value: bool,
        disabled: DisabledFn,
    ) -> Self {
        self.control(Control::Checkbox {
            name: name.to_string(),
            value,
            disabled,
        })
    }

    pub fn select<S>(
        self,
        name: &str,
        value: &str,
        options: &[S],
        disabled: DisabledFn,
    ) -> Self
    where
        S: AsRef<str>,
    {
        self.control(Control::Select {
            name: name.into(),
            value: value.into(),
            options: options.iter().map(|s| s.as_ref().to_string()).collect(),
            disabled,
        })
    }

    pub fn separator(self) -> Self {
        self.control(Control::Separator {})
    }

    pub fn dynamic_separator(self, name: &str) -> Self {
        self.control(Control::DynamicSeparator {
            name: name.to_string(),
        })
    }

    pub fn slider(
        self,
        name: &str,
        value: f32,
        range: (f32, f32),
        step: f32,
        disabled: DisabledFn,
    ) -> Self {
        self.control(Control::Slider {
            name: name.to_string(),
            value,
            min: range.0,
            max: range.1,
            step,
            disabled,
        })
    }

    pub fn slider_normalized(self, name: &str, value: f32) -> Self {
        self.control(Control::Slider {
            name: name.to_string(),
            value,
            min: 0.0,
            max: 1.0,
            step: 0.001,
            disabled: None,
        })
    }

    pub fn build(self) -> UiControls {
        UiControls::with_previous(self.controls)
    }
}

#[derive(Clone)]
struct ChangeTracker {
    save_previous: bool,
    changed: bool,
    previous_values: ControlValues,
}

impl Default for ChangeTracker {
    fn default() -> Self {
        Self::new(false)
    }
}

impl ChangeTracker {
    fn new(save_previous: bool) -> Self {
        Self {
            changed: true,
            save_previous,
            previous_values: ControlValues::default(),
        }
    }

    fn changed(&self) -> bool {
        self.check_can_save_previous();
        self.changed
    }

    fn any_changed_in(&self, names: &[&str], values: &ControlValues) -> bool {
        self.check_can_save_previous();

        if self.previous_values.is_empty() {
            for name in names {
                if !values.contains_key(*name) {
                    panic!("Control {} does not exist", name);
                }
            }
            return true;
        }

        for name in names {
            for name in names {
                if !values.contains_key(*name) {
                    panic!("Control {} does not exist", name);
                }
            }
            if let Some(current) = values.get(*name) {
                if let Some(previous) = self.previous_values.get(*name) {
                    if current != previous {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn mark_unchanged(&mut self, latest_values: &ControlValues) {
        self.check_can_save_previous();
        self.changed = false;
        self.previous_values = latest_values.clone();
    }

    fn mark_changed(&mut self) {
        self.changed = true;
    }

    fn check_can_save_previous(&self) {
        if !self.save_previous {
            panic!(
                "Cannot check previous values when `save_previous` is false.\n\
                Use `Controls::with_previous` instead of `new`.
                "
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controls_changed() {
        let mut controls =
            UiControls::with_previous(vec![Control::slider_n("foo", 0.5)]);
        assert!(controls.changed());
        controls.mark_unchanged();
        assert!(!controls.changed());
    }

    #[test]
    fn test_any_changed_in() {
        let mut controls =
            UiControls::with_previous(vec![Control::slider_n("foo", 0.5)]);

        assert!(controls.any_changed_in(&["foo"]));
        controls.mark_unchanged();
        assert!(!controls.any_changed_in(&["foo"]));

        controls.update_value("foo", ControlValue::Float(0.7));
        assert!(controls.any_changed_in(&["foo"]));
    }

    #[test]
    fn test_mark_unchanged() {
        let mut controls =
            UiControls::with_previous(vec![Control::slider_n("foo", 0.5)]);

        controls.update_value("foo", ControlValue::Float(0.7));
        assert!(controls.changed());

        controls.mark_unchanged();
        assert!(!controls.changed());
    }
}
