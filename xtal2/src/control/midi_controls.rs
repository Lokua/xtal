use std::sync::{Arc, Mutex};

use super::control_traits::{ControlCollection, ControlConfig};
use crate::framework::prelude::HashMap;

#[derive(Clone, Debug)]
pub struct MidiControlConfig {
    pub channel: u8,
    pub cc: u8,
    pub min: f32,
    pub max: f32,
    pub value: f32,
}

impl MidiControlConfig {
    pub fn new(midi: (u8, u8), range: (f32, f32), value: f32) -> Self {
        Self {
            channel: midi.0,
            cc: midi.1,
            min: range.0,
            max: range.1,
            value,
        }
    }
}

impl ControlConfig<f32, f32> for MidiControlConfig {}

#[derive(Clone, Debug, Default)]
pub struct MidiControls {
    pub hrcc: bool,
    values: Arc<Mutex<HashMap<String, f32>>>,
    configs: HashMap<String, MidiControlConfig>,
    active: bool,
}

impl MidiControls {
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.active = true;
        Ok(())
    }

    pub fn restart(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.start()
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn messages(&self) -> Vec<[u8; 3]> {
        Vec::new()
    }

    pub fn messages_hrcc(&self) -> Vec<[u8; 3]> {
        Vec::new()
    }
}

impl
    ControlCollection<
        MidiControlConfig,
        f32,
        f32,
        HashMap<String, MidiControlConfig>,
    > for MidiControls
{
    fn add(&mut self, name: &str, config: MidiControlConfig) {
        self.values
            .lock()
            .unwrap()
            .insert(name.to_string(), config.value);
        self.configs.insert(name.to_string(), config);
    }

    fn config(&self, name: &str) -> Option<MidiControlConfig> {
        self.configs.get(name).cloned()
    }

    fn configs(&self) -> HashMap<String, MidiControlConfig> {
        self.configs.clone()
    }

    fn get(&self, name: &str) -> f32 {
        *self.values.lock().unwrap().get(name).unwrap_or(&0.0)
    }

    fn get_optional(&self, name: &str) -> Option<f32> {
        self.values.lock().unwrap().get(name).copied()
    }

    fn has(&self, name: &str) -> bool {
        self.values.lock().unwrap().contains_key(name)
    }

    fn remove(&mut self, name: &str) {
        self.values.lock().unwrap().remove(name);
        self.configs.remove(name);
    }

    fn set(&mut self, name: &str, value: f32) {
        self.values.lock().unwrap().insert(name.to_string(), value);
    }

    fn values(&self) -> HashMap<String, f32> {
        self.values.lock().unwrap().clone()
    }

    fn with_values_mut<F>(&mut self, f: F)
    where
        F: FnOnce(&mut HashMap<String, f32>),
    {
        let mut values = self.values.lock().unwrap();
        f(&mut values);
    }
}

#[derive(Default)]
pub struct MidiControlBuilder {
    controls: MidiControls,
}

impl MidiControlBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn control(mut self, name: &str, config: MidiControlConfig) -> Self {
        self.controls.add(name, config);
        self
    }

    pub fn build(self) -> MidiControls {
        self.controls
    }
}
