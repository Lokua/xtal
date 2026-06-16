//! UI-facing controls and values for sketch parameters.
//!
//! Sketches do not need to interact with this module directly – see
//! [`ControlHub`].

use std::fmt::{self, Debug};

use indexmap::IndexMap;
use log::error;
use serde::{Deserialize, Serialize};

use crate::core::prelude::*;
use crate::warn_once;

/// Runtime value stored for one UI control.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum ControlValue {
    /// Numeric slider value.
    Float(f32),
    /// Checkbox value.
    Bool(bool),
    /// Select option value.
    String(String),
}

impl ControlValue {
    /// Returns the contained float, if this is [`Self::Float`].
    pub fn as_float(&self) -> Option<f32> {
        if let ControlValue::Float(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    /// Returns the contained bool, if this is [`Self::Bool`].
    pub fn as_bool(&self) -> Option<bool> {
        if let ControlValue::Bool(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    /// Returns the contained string, if this is [`Self::String`].
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

/// Predicate used by [`UiControls`] to compute whether a control is disabled.
///
/// # Example
/// ```text
/// Control::Slider {
///     name: "phase",
///     value: 0.0,
///     min: 0.0,
///     max: 1.0,
///
///     // Slider will automatically become disabled when animate_phase is true
///     disabled: Some(Box::new(|controls| controls.bool("animate_phase"))),
/// };
/// ```
pub type DisabledFn = Option<Box<dyn Fn(&UiControls) -> bool>>;

/// UI control definition mirrored to the web UI.
pub enum UiControlConfig {
    /// Numeric slider control.
    Slider {
        /// Stable control name.
        name: String,
        /// Represents the initial value of this control and will not be updated
        /// after instantiation
        value: f32,
        /// Minimum slider value.
        min: f32,
        /// Maximum slider value.
        max: f32,
        /// Slider step size.
        step: f32,
        /// See [`DisabledFn`]
        disabled: DisabledFn,
    },
    /// Boolean checkbox control.
    Checkbox {
        /// Stable control name.
        name: String,
        /// Represents the initial value of this control and will not be updated
        /// after instantiation
        value: bool,
        /// See [`DisabledFn`]
        disabled: DisabledFn,
    },
    /// String select control.
    Select {
        /// Stable control name.
        name: String,
        /// Represents the initial value of this control and will not be updated
        /// after instantiation
        value: String,
        /// Allowed option labels.
        options: Vec<String>,
        /// See [`DisabledFn`]
        disabled: DisabledFn,
    },
    /// Visual separator in the UI.
    Separator {
        /// Stable separator id.
        name: String,
    },
}

impl UiControlConfig {
    /// Returns the stable control name.
    pub fn name(&self) -> &str {
        match self {
            UiControlConfig::Slider { name, .. } => name,
            UiControlConfig::Checkbox { name, .. } => name,
            UiControlConfig::Select { name, .. } => name,
            UiControlConfig::Separator { name } => name,
        }
    }

    /// Returns this config's initial value.
    pub fn value(&self) -> ControlValue {
        match self {
            UiControlConfig::Slider { value, .. } => {
                ControlValue::Float(*value)
            }
            UiControlConfig::Checkbox { value, .. } => {
                ControlValue::Bool(*value)
            }
            UiControlConfig::Select { value, .. } => {
                ControlValue::String(value.clone())
            }
            UiControlConfig::Separator { .. } => ControlValue::Bool(false),
        }
    }

    /// Creates a checkbox config.
    pub fn checkbox(name: &str, value: bool) -> UiControlConfig {
        UiControlConfig::Checkbox {
            name: name.to_string(),
            value,
            disabled: None,
        }
    }

    /// Creates a select config.
    pub fn select<S>(name: &str, value: &str, options: &[S]) -> UiControlConfig
    where
        S: AsRef<str>,
    {
        UiControlConfig::Select {
            name: name.into(),
            value: value.into(),
            options: options.iter().map(|s| s.as_ref().to_string()).collect(),
            disabled: None,
        }
    }

    /// Creates a slider config with explicit range and step.
    pub fn slider(
        name: &str,
        value: f32,
        range: (f32, f32),
        step: f32,
    ) -> UiControlConfig {
        UiControlConfig::Slider {
            name: name.to_string(),
            value,
            min: range.0,
            max: range.1,
            step,
            disabled: None,
        }
    }

    /// Creates a normalized slider with default `0.0..=1.0` range.
    pub fn slider_n(name: &str, value: f32) -> UiControlConfig {
        UiControlConfig::Slider {
            name: name.to_string(),
            value,
            min: 0.0,
            max: 1.0,
            step: 0.0001,
            disabled: None,
        }
    }

    /// Returns whether this control is disabled for the current UI state.
    pub fn is_disabled(&self, controls: &UiControls) -> bool {
        match self {
            UiControlConfig::Slider { disabled, .. }
            | UiControlConfig::Checkbox { disabled, .. }
            | UiControlConfig::Select { disabled, .. } => {
                disabled.as_ref().is_some_and(|f| f(controls))
            }
            _ => false,
        }
    }

    /// Returns this config's variant name for UI serialization.
    pub fn variant_string(&self) -> String {
        (match self {
            Self::Checkbox { .. } => "Checkbox",
            Self::Select { .. } => "Select",
            Self::Separator { .. } => "Separator",
            Self::Slider { .. } => "Slider",
        })
        .to_string()
    }

    /// Returns whether this config is a separator.
    pub fn is_separator(&self) -> bool {
        matches!(self, Self::Separator { .. })
    }
}

impl ControlConfig<ControlValue, f32> for UiControlConfig {}

impl Clone for UiControlConfig {
    fn clone(&self) -> Self {
        match self {
            UiControlConfig::Checkbox {
                name,
                value,
                disabled: _,
            } => UiControlConfig::Checkbox {
                name: name.clone(),
                value: *value,
                disabled: None,
            },
            UiControlConfig::Select {
                name,
                value,
                options,
                disabled: _,
            } => UiControlConfig::Select {
                name: name.clone(),
                value: value.clone(),
                options: options.clone(),
                disabled: None,
            },
            UiControlConfig::Separator { name } => {
                UiControlConfig::Separator { name: name.clone() }
            }
            UiControlConfig::Slider {
                name,
                value,
                min,
                max,
                step,
                disabled: _,
            } => UiControlConfig::Slider {
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

impl fmt::Debug for UiControlConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UiControlConfig::Checkbox {
                name,
                value,
                disabled,
                ..
            } => f
                .debug_struct("Checkbox")
                .field("name", name)
                .field("value", value)
                .field("disabled", &disabled.as_ref().map(|_| "<function>"))
                .finish(),
            UiControlConfig::Select {
                name,
                value,
                options,
                disabled,
                ..
            } => f
                .debug_struct("Select")
                .field("name", name)
                .field("value", value)
                .field("options", options)
                .field("disabled", &disabled.as_ref().map(|_| "<function>"))
                .finish(),
            UiControlConfig::Separator { name } => {
                f.debug_struct("Separator").field("name", name).finish()
            }
            UiControlConfig::Slider {
                name,
                value,
                min,
                max,
                step,
                disabled,
                ..
            } => f
                .debug_struct("Slider")
                .field("name", name)
                .field("value", value)
                .field("min", min)
                .field("max", max)
                .field("step", step)
                .field("disabled", &disabled.as_ref().map(|_| "<function>"))
                .finish(),
        }
    }
}

/// Runtime UI control values keyed by control name.
pub type ControlValues = HashMap<String, ControlValue>;

/// A generic abstraction over UI controls that sketches can directly interact
/// with without being coupled to a specific UI framework. The original version
/// of Xtal used Egui for this purpose but has since moved on to using a
/// WebView for greater UI flexibility
#[derive(Clone, Default)]
pub struct UiControls {
    /// Holds the original [`UiControlConfig`] references and their default
    /// values – runtime values are not included here!
    configs: IndexMap<String, UiControlConfig>,
    values: HashMap<String, ControlValue>,
    change_tracker: ChangeTracker,
}

impl UiControls {
    /// Creates a UI collection from ordered control configs.
    pub fn new(controls: &[UiControlConfig]) -> Self {
        let configs: IndexMap<String, UiControlConfig> = controls
            .iter()
            .map(|control| (control.name().to_string(), control.clone()))
            .collect();

        let values: HashMap<String, ControlValue> = controls
            .iter()
            .map(|control| (control.name().to_string(), control.value()))
            .collect();

        Self {
            configs,
            values,
            change_tracker: ChangeTracker::default(),
        }
    }

    /// Returns a float control value or logs and returns `0.0`.
    pub fn float(&self, name: &str) -> f32 {
        self.values
            .get(name)
            .and_then(ControlValue::as_float)
            .unwrap_or_else(|| {
                error!("No float for `{}`. Returning 0.0.", name);
                0.0
            })
    }

    /// Returns a checkbox value or logs and returns `false`.
    pub fn bool(&self, name: &str) -> bool {
        self.values
            .get(name)
            .and_then(ControlValue::as_bool)
            .unwrap_or_else(|| {
                error!("No bool for `{}`. Returning false.", name);
                false
            })
    }

    /// Converts a checkbox value into `0.0` or `1.0`.
    pub fn bool_as_f32(&self, name: &str) -> f32 {
        bool_to_f32(self.bool(name))
    }

    /// Returns a select string value or logs and returns an empty string.
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

    /// Returns the matching option index of a select as `f32`.
    pub fn string_as_f32(&self, name: &str) -> f32 {
        let value = self.string(name);
        if let Some(UiControlConfig::Select { options, .. }) = self.config(name)
        {
            return options.iter().position(|x| *x == value).unwrap_or(0)
                as f32;
        }
        0.0
    }

    /// Returns whether any UI value has changed since the last reset.
    pub fn changed(&self) -> bool {
        self.change_tracker.changed()
    }
    /// Returns whether any named UI value has changed since the last reset.
    pub fn any_changed_in(&self, names: &[&str]) -> bool {
        self.change_tracker.any_changed_in(names, &self.values)
    }
    /// Marks all current values as consumed by the sketch.
    pub fn mark_unchanged(&mut self) {
        self.change_tracker.mark_unchanged(&self.values);
    }
    /// Marks the collection as changed.
    pub fn mark_changed(&mut self) {
        self.change_tracker.mark_changed();
    }

    /// Returns whether a named control is currently disabled.
    pub fn disabled(&self, name: &str) -> bool {
        self.configs.get(name).is_some_and(|c| c.is_disabled(self))
    }

    /// Returns the slider range for `name`, if `name` is a slider.
    pub fn slider_range(&self, name: &str) -> Option<(f32, f32)> {
        self.config(name).and_then(|control| match control {
            UiControlConfig::Slider { min, max, .. } => Some((min, max)),
            _ => {
                error!(
                    "Unable to find a Control definition for Slider `{}`",
                    name
                );
                None
            }
        })
    }

    /// Returns ordered config references.
    pub fn config_refs(&self) -> &IndexMap<String, UiControlConfig> {
        &self.configs
    }
}

impl
    ControlCollection<
        UiControlConfig,
        ControlValue,
        f32,
        IndexMap<String, UiControlConfig>,
    > for UiControls
{
    fn add(&mut self, name: &str, control: UiControlConfig) {
        let value = control.value();
        self.configs.insert(name.to_string(), control);
        self.values.insert(name.to_string(), value);
        self.change_tracker.mark_changed();
    }

    fn config(&self, name: &str) -> Option<UiControlConfig> {
        self.configs.get(name).cloned()
    }

    fn configs(&self) -> IndexMap<String, UiControlConfig> {
        self.configs.clone()
    }

    /// Same as `float`, only will try to coerce a possibly existing Checkbox's
    /// bool to 0.0 or 1.0 or a Select's string into its matching option index
    /// (useful in shader context where you are only passing in banks of
    /// `vec4<f32>` to uniforms)
    fn get(&self, name: &str) -> f32 {
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
    fn get_optional(&self, name: &str) -> Option<f32> {
        if let Some(value) =
            self.values.get(name).and_then(ControlValue::as_float)
        {
            return Some(value);
        }

        match self.config(name) {
            Some(UiControlConfig::Checkbox { .. }) => {
                Some(self.bool_as_f32(name))
            }
            Some(UiControlConfig::Select { .. }) => {
                Some(self.string_as_f32(name))
            }
            _ => None,
        }
    }

    fn has(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }

    fn remove(&mut self, name: &str) {
        self.configs.shift_remove(name);
        self.values.remove(name);
    }

    fn set(&mut self, name: &str, value: ControlValue) {
        if let Some(old_value) = self.values.get(name)
            && *old_value != value
        {
            self.change_tracker.mark_changed();
            self.values.insert(name.to_string(), value);
        }
    }

    fn values(&self) -> HashMap<String, ControlValue> {
        self.values.clone()
    }

    fn with_values_mut<F>(&mut self, f: F)
    where
        F: FnOnce(&mut HashMap<String, ControlValue>),
    {
        f(&mut self.values)
    }
}

impl fmt::Debug for UiControls {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("UiControls");
        debug_struct.field("configs", &self.configs);
        debug_struct.field("values", &self.values);
        debug_struct.finish()
    }
}

/// Builder for programmatic UI control collections.
#[derive(Default)]
pub struct UiControlBuilder {
    controls: Vec<UiControlConfig>,
}

impl UiControlBuilder {
    /// Creates an empty UI control builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an already constructed UI control config.
    pub fn control(mut self, control: UiControlConfig) -> Self {
        self.controls.push(control);
        self
    }

    /// Adds a checkbox control.
    pub fn checkbox(
        self,
        name: &str,
        value: bool,
        disabled: DisabledFn,
    ) -> Self {
        self.control(UiControlConfig::Checkbox {
            name: name.to_string(),
            value,
            disabled,
        })
    }

    /// Adds a select control.
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
        self.control(UiControlConfig::Select {
            name: name.into(),
            value: value.into(),
            options: options.iter().map(|s| s.as_ref().to_string()).collect(),
            disabled,
        })
    }

    /// Adds a separator with an explicit id.
    pub fn separator_internal(self, name: &str) -> Self {
        self.control(UiControlConfig::Separator {
            name: name.to_string(),
        })
    }

    /// Adds a separator with a generated id.
    pub fn separator(self) -> Self {
        self.separator_internal(&uuid_5())
    }

    /// Adds a slider with explicit range, step, and disabled predicate.
    pub fn slider(
        self,
        name: &str,
        value: f32,
        range: (f32, f32),
        step: f32,
        disabled: DisabledFn,
    ) -> Self {
        self.control(UiControlConfig::Slider {
            name: name.to_string(),
            value,
            min: range.0,
            max: range.1,
            step,
            disabled,
        })
    }

    /// Adds a normalized slider with default `0.0..=1.0` range.
    pub fn slider_n(self, name: &str, value: f32) -> Self {
        self.control(UiControlConfig::Slider {
            name: name.to_string(),
            value,
            min: 0.0,
            max: 1.0,
            step: 0.001,
            disabled: None,
        })
    }

    /// Builds the configured UI control collection.
    pub fn build(self) -> UiControls {
        UiControls::new(&self.controls)
    }
}

#[derive(Clone)]
struct ChangeTracker {
    changed: bool,
    previous_values: ControlValues,
}

impl Default for ChangeTracker {
    fn default() -> Self {
        Self {
            changed: true,
            previous_values: ControlValues::default(),
        }
    }
}

impl ChangeTracker {
    fn changed(&self) -> bool {
        self.changed
    }

    fn any_changed_in(&self, names: &[&str], values: &ControlValues) -> bool {
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
            if let Some(current) = values.get(*name)
                && let Some(previous) = self.previous_values.get(*name)
                && current != previous
            {
                return true;
            }
        }

        false
    }

    fn mark_unchanged(&mut self, latest_values: &ControlValues) {
        self.changed = false;
        self.previous_values = latest_values.clone();
    }

    fn mark_changed(&mut self) {
        self.changed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controls_changed() {
        let mut controls =
            UiControls::new(&[UiControlConfig::slider_n("foo", 0.5)]);
        assert!(controls.changed());
        controls.mark_unchanged();
        assert!(!controls.changed());
    }

    #[test]
    fn test_any_changed_in() {
        let mut controls =
            UiControls::new(&[UiControlConfig::slider_n("foo", 0.5)]);

        assert!(controls.any_changed_in(&["foo"]));
        controls.mark_unchanged();
        assert!(!controls.any_changed_in(&["foo"]));

        controls.set("foo", ControlValue::Float(0.7));
        assert!(controls.any_changed_in(&["foo"]));
    }

    #[test]
    fn test_mark_unchanged() {
        let mut controls =
            UiControls::new(&[UiControlConfig::slider_n("foo", 0.5)]);

        controls.set("foo", ControlValue::Float(0.7));
        assert!(controls.changed());

        controls.mark_unchanged();
        assert!(!controls.changed());
    }
}
