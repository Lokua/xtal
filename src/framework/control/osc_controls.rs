//! Control sketch parameters with OSC.
//!
//! Sketches do not need to interact with this module directly - see
//! [`ControlHub`].
use nannou_osc as osc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::framework::osc_receiver::SHARED_OSC_RECEIVER;
use crate::framework::prelude::*;

#[derive(Clone, Debug)]
pub struct OscControlConfig {
    pub address: String,
    pub min: f32,
    pub max: f32,
    pub default: f32,
}

impl OscControlConfig {
    pub fn new(address: &str, range: (f32, f32), default: f32) -> Self {
        check_address(address);
        let (min, max) = range;
        Self {
            address: address.to_string(),
            min,
            max,
            default,
        }
    }
}

#[derive(Debug, Default)]
struct OscState {
    values: HashMap<String, f32>,
}

impl OscState {
    fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    fn set(&mut self, address: &str, value: f32) {
        self.values.insert(address.to_string(), value);
    }

    fn get(&self, address: &str) -> f32 {
        *self.values.get(address).unwrap_or(&0.0)
    }

    fn has(&self, address: &str) -> bool {
        self.values.contains_key(address)
    }

    fn values(&self) -> HashMap<String, f32> {
        self.values.clone()
    }
}

#[derive(Clone, Debug)]
pub struct OscControls {
    pub is_active: bool,
    configs: HashMap<String, OscControlConfig>,
    state: Arc<Mutex<OscState>>,
}

impl Default for OscControls {
    fn default() -> Self {
        Self {
            configs: HashMap::new(),
            state: Arc::new(Mutex::new(OscState::new())),
            is_active: false,
        }
    }
}

impl OscControls {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, address: &str, config: OscControlConfig) {
        check_address(address);
        self.state.lock().unwrap().set(address, config.default);
        self.configs.insert(address.to_string(), config);
    }

    pub fn has(&self, address: &str) -> bool {
        check_address(address);
        self.state.lock().unwrap().has(address)
    }

    pub fn get(&self, address: &str) -> f32 {
        check_address(address);
        self.state.lock().unwrap().get(address)
    }

    pub fn set(&self, address: &str, value: f32) {
        check_address(address);
        self.state.lock().unwrap().set(address, value);
    }

    pub fn values(&self) -> HashMap<String, f32> {
        return self.state.lock().unwrap().values();
    }

    pub fn with_values_mut<F>(&self, f: F)
    where
        F: FnOnce(&mut HashMap<String, f32>),
    {
        let mut state = self.state.lock().unwrap();
        f(&mut state.values);
    }

    pub fn update_value(&mut self, address: &str, value: f32) {
        check_address(address);
        self.state.lock().unwrap().set(address, value);
    }

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

pub struct OscControlBuilder {
    controls: OscControls,
}

impl Default for OscControlBuilder {
    fn default() -> Self {
        Self {
            controls: OscControls::new(),
        }
    }
}

impl OscControlBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn control(
        mut self,
        address: &str,
        range: (f32, f32),
        default: f32,
    ) -> Self {
        let config = OscControlConfig::new(address, range, default);
        self.controls.add(address, config);
        self
    }

    pub fn control_n(mut self, address: &str, default: f32) -> Self {
        let config = OscControlConfig::new(address, (0.0, 1.0), default);
        self.controls.add(address, config);
        self
    }

    pub fn build(mut self) -> OscControls {
        if let Err(e) = self.controls.start() {
            error!(
                "Failed to initialize OSC controls: {}. Using default values.",
                e
            );
        }
        self.controls
    }
}

fn check_address(address: &str) {
    if address.starts_with('/') {
        panic!("Unsupported address format. Remove leading `/` from address");
    }
}
