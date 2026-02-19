use std::sync::{Arc, Mutex};

use super::control_traits::{ControlCollection, ControlConfig};
use crate::framework::osc_receiver::SHARED_OSC_RECEIVER;
use crate::framework::prelude::*;
use nannou_osc as osc;

#[derive(Clone, Debug)]
pub struct OscControlConfig {
    pub address: String,
    pub min: f32,
    pub max: f32,
    pub value: f32,
}

impl OscControlConfig {
    pub fn new(address: &str, range: (f32, f32), value: f32) -> Self {
        Self {
            address: address.to_string(),
            min: range.0,
            max: range.1,
            value,
        }
    }
}

impl ControlConfig<f32, f32> for OscControlConfig {}

#[derive(Clone, Debug, Default)]
pub struct OscControls {
    pub is_active: bool,
    configs: HashMap<String, OscControlConfig>,
    state: Arc<Mutex<State>>,
}

impl OscControls {
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let state = self.state.clone();
        let configs = self.configs.clone();

        SHARED_OSC_RECEIVER.register_callback("*", move |msg| {
            let key = msg.addr.trim_start_matches('/');

            if let Some(config) = configs.get(key) {
                let value: Option<f32> = match msg.args.first() {
                    Some(osc::Type::Float(value)) => Some(*value),
                    Some(osc::Type::Int(value)) => Some(*value as f32),
                    Some(osc::Type::Double(value)) => Some(*value as f32),
                    _ => None,
                };

                if let Some(value) = value {
                    trace!("Setting {} to {}", key, value);
                    let mapped_value =
                        value * (config.max - config.min) + config.min;
                    state.lock().unwrap().set(key, mapped_value);
                }
            }
        });

        self.is_active = true;
        Ok(())
    }
}

impl
    ControlCollection<
        OscControlConfig,
        f32,
        f32,
        HashMap<String, OscControlConfig>,
    > for OscControls
{
    fn add(&mut self, name: &str, config: OscControlConfig) {
        check_address(name);
        self.state.lock().unwrap().set(name, config.value);
        self.configs.insert(name.to_string(), config);
    }

    fn config(&self, name: &str) -> Option<OscControlConfig> {
        self.configs.get(name).cloned()
    }

    fn configs(&self) -> HashMap<String, OscControlConfig> {
        self.configs.clone()
    }

    fn get(&self, name: &str) -> f32 {
        check_address(name);
        self.state.lock().unwrap().get(name)
    }

    fn get_optional(&self, name: &str) -> Option<f32> {
        check_address(name);
        self.state.lock().unwrap().get_optional(name).copied()
    }

    fn has(&self, name: &str) -> bool {
        check_address(name);
        self.state.lock().unwrap().has(name)
    }

    fn remove(&mut self, name: &str) {
        self.state.lock().unwrap().remove(name);
        self.configs.remove(name);
    }

    fn set(&mut self, name: &str, value: f32) {
        check_address(name);
        self.state.lock().unwrap().set(name, value);
    }

    fn values(&self) -> HashMap<String, f32> {
        self.state.lock().unwrap().values()
    }

    fn with_values_mut<F>(&mut self, f: F)
    where
        F: FnOnce(&mut HashMap<String, f32>),
    {
        let mut state = self.state.lock().unwrap();
        f(&mut state.values);
    }
}

#[derive(Default)]
pub struct OscControlBuilder {
    controls: OscControls,
}

impl OscControlBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn control(mut self, name: &str, config: OscControlConfig) -> Self {
        self.controls.add(name, config);
        self
    }

    pub fn build(self) -> OscControls {
        self.controls
    }
}

#[derive(Debug, Default)]
struct State {
    values: HashMap<String, f32>,
}

impl State {
    fn get(&self, address: &str) -> f32 {
        *self.values.get(address).unwrap_or(&0.0)
    }

    fn get_optional(&self, address: &str) -> Option<&f32> {
        self.values.get(address)
    }

    fn has(&self, address: &str) -> bool {
        self.values.contains_key(address)
    }

    fn remove(&mut self, name: &str) {
        self.values.remove(name);
    }

    fn set(&mut self, address: &str, value: f32) {
        self.values.insert(address.to_string(), value);
    }

    fn values(&self) -> HashMap<String, f32> {
        self.values.clone()
    }
}

fn check_address(address: &str) {
    if address.starts_with('/') {
        panic!("Unsupported address format. Remove leading `/`.");
    }
}
