use crate::framework::util::HashMap;

pub trait ControlConfig<VWrapper, V> {}

/// Parent trait for all control collections.
///
/// A "config" represents a concrete [`ControlConfig`] implementation and is
/// intentionally separated from the value associated with it for efficiency.
/// `VWrapper` and `V` can refer to the same thing in the case a control uses a
/// single primitive value (like MIDI), otherwise `VWrapper` can represent an
/// enum over variants (like for UIControls or possibly in the future for OSC if
/// we want to support strings and booleans – we'll be able to without breaking
/// changes)
pub trait ControlCollection<
    C: ControlConfig<VWrapper, V>,
    VWrapper,
    V: Default,
    Map: IntoIterator<Item = (String, C)>,
>
{
    fn add(&mut self, name: &str, config: C);
    fn config(&self, name: &str) -> Option<C>;
    fn configs(&self) -> Map;
    fn get(&self, name: &str) -> V;
    fn get_optional(&self, name: &str) -> Option<V>;
    fn has(&self, name: &str) -> bool {
        self.config(name).is_some()
    }
    fn remove(&mut self, name: &str);
    fn set(&mut self, name: &str, value: VWrapper);
    fn values(&self) -> HashMap<String, VWrapper>;
    fn with_values_mut<F>(&mut self, f: F)
    where
        F: FnOnce(&mut HashMap<String, VWrapper>);
}
