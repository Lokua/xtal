//! Shared traits for typed control collections.

use crate::core::util::HashMap;

/// Marker trait for configuration objects owned by a control collection.
///
/// `VWrapper` is the type accepted by `set`, and `V` is the scalar type
/// returned by `get`. Some controls use the same type for both, while UI
/// controls accept [`crate::control::ControlValue`] and return `f32` for shader
/// uniform compatibility.
pub trait ControlConfig<VWrapper, V> {}

/// Parent trait for all control collections.
///
/// A "config" represents a concrete [`ControlConfig`] implementation and is
/// intentionally separated from the value associated with it for efficiency.
/// `VWrapper` and `V` can refer to the same thing in the case a control uses a
/// single primitive value (like MIDI), otherwise `VWrapper` can represent an
/// enum over variants, like `UiControls` or possibly in the future for OSC if
/// strings and booleans are needed without breaking changes.
pub trait ControlCollection<
    C: ControlConfig<VWrapper, V>,
    VWrapper,
    V: Default,
    Map: IntoIterator<Item = (String, C)>,
>
{
    /// Adds or replaces one named control config and initializes its value.
    fn add(&mut self, name: &str, config: C);

    /// Returns the config for a named control, if it exists.
    fn config(&self, name: &str) -> Option<C>;

    /// Returns all configs in the collection.
    fn configs(&self) -> Map;

    /// Returns the current scalar value or a collection-specific fallback.
    fn get(&self, name: &str) -> V;

    /// Returns the current scalar value without applying a fallback.
    fn get_optional(&self, name: &str) -> Option<V>;

    /// Returns whether the collection contains a config for `name`.
    fn has(&self, name: &str) -> bool {
        self.config(name).is_some()
    }

    /// Removes a named config and its runtime value.
    fn remove(&mut self, name: &str);

    /// Sets the current runtime value for one control.
    fn set(&mut self, name: &str, value: VWrapper);

    /// Returns all current runtime values.
    fn values(&self) -> HashMap<String, VWrapper>;

    /// Gives mutable access to the backing value map.
    fn with_values_mut<F>(&mut self, f: F)
    where
        F: FnOnce(&mut HashMap<String, VWrapper>);
}
