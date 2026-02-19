use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::control_traits::{ControlCollection, ControlConfig};
use crate::framework::prelude::{HashMap, info};
use crate::motion::SlewLimiter;

#[derive(Clone, Debug)]
pub struct AudioControlConfig {
    pub channel: usize,
    pub slew: SlewLimiter,
    pub pre: f32,
    pub detect: f32,
    pub min: f32,
    pub max: f32,
    pub value: f32,
}

impl AudioControlConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        channel: usize,
        slew: SlewLimiter,
        pre: f32,
        detect: f32,
        range: (f32, f32),
        value: f32,
    ) -> Self {
        Self {
            channel,
            slew,
            pre,
            detect,
            min: range.0,
            max: range.1,
            value,
        }
    }
}

impl ControlConfig<f32, f32> for AudioControlConfig {}

#[derive(Clone, Debug, Default)]
pub struct AudioControls {
    pub is_active: bool,
    values: Arc<Mutex<HashMap<String, f32>>>,
    configs: HashMap<String, AudioControlConfig>,
}

impl AudioControls {
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.is_active = true;
        Ok(())
    }

    pub fn restart(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.is_active = false;
        info!("Restarting audio controls");
        thread::sleep(Duration::from_millis(10));
        self.start()
    }
}

impl
    ControlCollection<
        AudioControlConfig,
        f32,
        f32,
        HashMap<String, AudioControlConfig>,
    > for AudioControls
{
    fn add(&mut self, name: &str, config: AudioControlConfig) {
        self.values
            .lock()
            .unwrap()
            .insert(name.to_string(), config.value);
        self.configs.insert(name.to_string(), config);
    }

    fn config(&self, name: &str) -> Option<AudioControlConfig> {
        self.configs.get(name).cloned()
    }

    fn configs(&self) -> HashMap<String, AudioControlConfig> {
        self.configs.clone()
    }

    fn get(&self, name: &str) -> f32 {
        *self.values.lock().unwrap().get(name).unwrap_or(&0.0)
    }

    fn get_optional(&self, name: &str) -> Option<f32> {
        self.values.lock().unwrap().get(name).copied()
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
pub struct AudioControlBuilder {
    controls: AudioControls,
}

impl AudioControlBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn control(mut self, name: &str, config: AudioControlConfig) -> Self {
        self.controls.add(name, config);
        self
    }

    pub fn build(self) -> AudioControls {
        self.controls
    }
}
