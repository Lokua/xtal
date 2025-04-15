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

impl Default for ControlValue {
    fn default() -> Self {
        Self::Float(0.0)
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

pub enum UiControl {
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
    Separator {
        name: String,
    },
}

impl UiControl {
    pub fn name(&self) -> &str {
        match self {
            UiControl::Slider { name, .. } => name,
            UiControl::Checkbox { name, .. } => name,
            UiControl::Select { name, .. } => name,
            UiControl::Separator { name } => name,
        }
    }

    pub fn value(&self) -> ControlValue {
        match self {
            UiControl::Slider { value, .. } => ControlValue::Float(*value),
            UiControl::Checkbox { value, .. } => ControlValue::Bool(*value),
            UiControl::Select { value, .. } => {
                ControlValue::String(value.clone())
            }
            UiControl::Separator { .. } => ControlValue::Bool(false),
        }
    }

    pub fn checkbox(name: &str, value: bool) -> UiControl {
        UiControl::Checkbox {
            name: name.to_string(),
            value,
            disabled: None,
        }
    }

    pub fn checkbox_x<F>(name: &str, value: bool, disabled: F) -> UiControl
    where
        F: Fn(&UiControls) -> bool + 'static,
    {
        UiControl::Checkbox {
            name: name.to_string(),
            value,
            disabled: Some(Box::new(disabled)),
        }
    }

    pub fn select<S>(name: &str, value: &str, options: &[S]) -> UiControl
    where
        S: AsRef<str>,
    {
        UiControl::Select {
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
    ) -> UiControl
    where
        S: AsRef<str>,
        F: Fn(&UiControls) -> bool + 'static,
    {
        UiControl::Select {
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
    ) -> UiControl {
        UiControl::Slider {
            name: name.to_string(),
            value,
            min: range.0,
            max: range.1,
            step,
            disabled: None,
        }
    }

    /// Convenience version of [`Self::slider`] with default [0.0, 1.0] range.
    pub fn slider_n(name: &str, value: f32) -> UiControl {
        UiControl::Slider {
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
    ) -> UiControl
    where
        F: Fn(&UiControls) -> bool + 'static,
    {
        UiControl::Slider {
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
            UiControl::Slider { disabled, .. }
            | UiControl::Checkbox { disabled, .. }
            | UiControl::Select { disabled, .. } => {
                disabled.as_ref().is_some_and(|f| f(controls))
            }
            _ => false,
        }
    }

    pub fn variant_string(&self) -> String {
        (match self {
            Self::Checkbox { .. } => "Checkbox",
            Self::Select { .. } => "Select",
            Self::Separator { .. } => "Separator",
            Self::Slider { .. } => "Slider",
        })
        .to_string()
    }

    pub fn is_separator(&self) -> bool {
        matches!(self, Self::Separator { .. })
    }
}

impl ControlConfig<ControlValue> for UiControl {}

impl Clone for UiControl {
    fn clone(&self) -> Self {
        match self {
            UiControl::Checkbox {
                name,
                value,
                disabled: _,
            } => UiControl::Checkbox {
                name: name.clone(),
                value: *value,
                disabled: None,
            },
            UiControl::Select {
                name,
                value,
                options,
                disabled: _,
            } => UiControl::Select {
                name: name.clone(),
                value: value.clone(),
                options: options.clone(),
                disabled: None,
            },
            UiControl::Separator { name } => {
                UiControl::Separator { name: name.clone() }
            }
            UiControl::Slider {
                name,
                value,
                min,
                max,
                step,
                disabled: _,
            } => UiControl::Slider {
                name: name.clone(),
                value: *value,
                min: *min,
                max: *max,
                step: *step,
                disabled: None,
            },
        }
    }
}

impl fmt::Debug for UiControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UiControl::Checkbox { name, value, .. } => f
                .debug_struct("Checkbox")
                .field("name", name)
                .field("value", value)
                .finish(),
            UiControl::Select {
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
            UiControl::Separator { name } => {
                f.debug_struct("Separator").field("name", name).finish()
            }
            UiControl::Slider {
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
    /// Holds the original Control references and their default values - runtime
    /// values are not included here!
    configs: Vec<UiControl>,
    values: ControlValues,
    change_tracker: ChangeTracker,
}

impl UiControls {
    pub fn new(controls: Vec<UiControl>) -> Self {
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

    pub fn with_previous(controls: Vec<UiControl>) -> Self {
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

    pub fn extend(&mut self, controls: Vec<UiControl>) {
        let values: ControlValues = controls
            .iter()
            .map(|control| (control.name().to_string(), control.value()))
            .collect();

        self.values.extend(values);
        self.configs.extend(controls);
    }

    pub fn configs(&self) -> &Vec<UiControl> {
        &self.configs
    }

    pub fn configs_mut(&mut self) -> &mut Vec<UiControl> {
        &mut self.configs
    }

    pub fn values_mut(&mut self) -> &mut ControlValues {
        &mut self.values
    }

    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&UiControl) -> bool,
    {
        self.configs.retain(f);
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
            Some(UiControl::Checkbox { .. }) => Some(self.bool_as_f32(name)),
            Some(UiControl::Select { .. }) => Some(self.string_as_f32(name)),
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
        if let Some(UiControl::Select { options, .. }) = self.config(name) {
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

    pub fn disabled(&self, name: &str) -> Option<bool> {
        self.config(name).map(|c| c.is_disabled(self))
    }

    pub fn slider_range(&self, name: &str) -> Option<(f32, f32)> {
        self.config(name).and_then(|control| match control {
            UiControl::Slider { min, max, .. } => Some((*min, *max)),
            _ => {
                error!(
                    "Unable to find a Control definition for Slider `{}`",
                    name
                );
                None
            }
        })
    }
}

impl ControlCollection<UiControl, ControlValue> for UiControls {
    fn add(&mut self, _name: &str, control: UiControl) {
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

    fn config(&self, name: &str) -> Option<&UiControl> {
        self.configs.iter().find(|c| c.name() == name)
    }

    fn configs(&self) -> HashMap<String, UiControl> {
        panic!()
    }

    fn get(&self, _name: &str) -> ControlValue {
        panic!()
    }

    fn get_optional(&self, _name: &str) -> Option<ControlValue> {
        panic!()
    }

    fn has(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }

    fn remove(&mut self, name: &str) {
        self.retain(|c| c.name() != name)
    }

    fn set(&mut self, name: &str, value: ControlValue) {
        if let Some(old_value) = self.values.get(name) {
            if *old_value != value {
                self.change_tracker.mark_changed();
                self.values.insert(name.to_string(), value);
            }
        }
    }

    fn values(&self) -> HashMap<String, ControlValue> {
        self.values.clone()
    }

    fn with_values_mut<F>(&self, _f: F)
    where
        F: FnOnce(&mut HashMap<String, ControlValue>),
    {
        todo!()
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
    controls: Vec<UiControl>,
}

impl UiControlBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn control(mut self, control: UiControl) -> Self {
        self.controls.push(control);
        self
    }

    pub fn checkbox(
        self,
        name: &str,
        value: bool,
        disabled: DisabledFn,
    ) -> Self {
        self.control(UiControl::Checkbox {
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
        self.control(UiControl::Select {
            name: name.into(),
            value: value.into(),
            options: options.iter().map(|s| s.as_ref().to_string()).collect(),
            disabled,
        })
    }

    pub fn separator_internal(self, name: &str) -> Self {
        self.control(UiControl::Separator {
            name: name.to_string(),
        })
    }

    pub fn separator(self) -> Self {
        self.separator_internal(&uuid_5())
    }

    pub fn slider(
        self,
        name: &str,
        value: f32,
        range: (f32, f32),
        step: f32,
        disabled: DisabledFn,
    ) -> Self {
        self.control(UiControl::Slider {
            name: name.to_string(),
            value,
            min: range.0,
            max: range.1,
            step,
            disabled,
        })
    }

    pub fn slider_normalized(self, name: &str, value: f32) -> Self {
        self.control(UiControl::Slider {
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
            UiControls::with_previous(vec![UiControl::slider_n("foo", 0.5)]);
        assert!(controls.changed());
        controls.mark_unchanged();
        assert!(!controls.changed());
    }

    #[test]
    fn test_any_changed_in() {
        let mut controls =
            UiControls::with_previous(vec![UiControl::slider_n("foo", 0.5)]);

        assert!(controls.any_changed_in(&["foo"]));
        controls.mark_unchanged();
        assert!(!controls.any_changed_in(&["foo"]));

        controls.set("foo", ControlValue::Float(0.7));
        assert!(controls.any_changed_in(&["foo"]));
    }

    #[test]
    fn test_mark_unchanged() {
        let mut controls =
            UiControls::with_previous(vec![UiControl::slider_n("foo", 0.5)]);

        controls.set("foo", ControlValue::Float(0.7));
        assert!(controls.changed());

        controls.mark_unchanged();
        assert!(!controls.changed());
    }
}
