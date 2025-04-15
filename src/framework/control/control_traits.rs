use crate::framework::util::HashMap;

pub trait ControlConfig<V> {}

/// Parent trait for all control collections. a `config` represents a concrete
/// [`ControlConfig`] implementation and is intentionally separated from the
/// value associated with it for efficiency; since values in some instances are
/// populated from separate threads, it would be excessive to have to clone a
/// config every time we wanted to query for a value.
pub trait ControlCollection<C: ControlConfig<V>, V: Default> {
    fn add(&mut self, name: &str, control: C);
    fn config(&self, name: &str) -> Option<&C>;
    fn configs(&self) -> HashMap<String, C>;
    fn get(&self, name: &str) -> V;
    fn get_optional(&self, name: &str) -> Option<V>;
    fn has(&self, name: &str) -> bool {
        self.config(name).is_some()
    }
    fn remove(&mut self, name: &str);
    fn set(&mut self, name: &str, value: V);
    fn values(&self) -> HashMap<String, V>;
    fn with_values_mut<F>(&self, f: F)
    where
        F: FnOnce(&mut HashMap<String, V>);
}
