use nannou_osc as osc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::osc_receiver::SHARED_OSC_RECEIVER;
use super::prelude::*;

pub const OSC_PORT: u16 = 2346;

#[derive(Clone, Debug)]
pub struct OscControlConfig {
    pub address: String,
    pub min: f32,
    pub max: f32,
    pub default: f32,
}

impl OscControlConfig {
    pub fn new(address: &str, range: (f32, f32), default: f32) -> Self {
        let (min, max) = range;
        Self {
            address: address.to_string(),
            min,
            max,
            default,
        }
    }
}

#[derive(Default)]
pub struct OscState {
    values: HashMap<String, f32>,
}

impl OscState {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn set(&mut self, address: &str, value: f32) {
        self.values.insert(address.to_string(), value);
    }

    pub fn get(&self, address: &str) -> f32 {
        *self.values.get(address).unwrap_or(&0.0)
    }
}

pub struct OscControls {
    configs: HashMap<String, OscControlConfig>,
    state: Arc<Mutex<OscState>>,
    is_active: bool,
}

impl OscControls {
    fn new() -> Self {
        Self {
            configs: HashMap::new(),
            state: Arc::new(Mutex::new(OscState::new())),
            is_active: false,
        }
    }

    fn add(&mut self, address: &str, config: OscControlConfig) {
        self.state.lock().unwrap().set(address, config.default);
        self.configs.insert(address.to_string(), config);
    }

    pub fn get(&self, address: &str) -> f32 {
        self.state.lock().unwrap().get(address)
    }

    fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let state = self.state.clone();
        let configs = self.configs.clone();

        SHARED_OSC_RECEIVER.register_callback("*", move |msg| {
            if let Some(config) = configs.get(&msg.addr) {
                let value: Option<f32> = match msg.args.get(0) {
                    Some(osc::Type::Float(value)) => Some(*value),
                    Some(osc::Type::Int(value)) => Some(*value as f32),
                    Some(osc::Type::Double(value)) => Some(*value as f32),
                    _ => None,
                };

                if let Some(value) = value {
                    trace!("Setting {} to {}", msg.addr, value);
                    let mapped_value =
                        value * (config.max - config.min) + config.min;
                    state.lock().unwrap().set(&msg.addr, mapped_value);
                }
            }
        });

        self.is_active = true;
        Ok(())
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }
}

pub struct OscControlBuilder {
    controls: OscControls,
}

impl OscControlBuilder {
    pub fn new() -> Self {
        Self {
            controls: OscControls::new(),
        }
    }

    pub fn control_mapped(
        mut self,
        address: &str,
        range: (f32, f32),
        default: f32,
    ) -> Self {
        let config = OscControlConfig::new(address, range, default);
        self.controls.add(address, config);
        self
    }

    pub fn control(mut self, address: &str, default: f32) -> Self {
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
