//! Control sketch parameters with OSC.
//!
//! Sketches do not need to interact with this module directly - see
//! [`ControlHub`].
use nannou_osc as osc;
use std::sync::{Arc, Mutex};

use crate::framework::osc_receiver::SHARED_OSC_RECEIVER;
use crate::framework::prelude::*;

#[derive(Clone, Debug)]
pub struct OscControlConfig {
    /// The OSC address _without_ leading slash
    pub address: String,
    pub min: f32,
    pub max: f32,
    /// Represents the initial value of this control and will not be updated
    /// after instantiation
    pub value: f32,
}

impl OscControlConfig {
    pub fn new(address: &str, range: (f32, f32), value: f32) -> Self {
        check_address(address);
        let (min, max) = range;
        Self {
            address: address.to_string(),
            min,
            max,
            value,
        }
    }
}

impl ControlConfig<f32, f32> for OscControlConfig {}

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

#[derive(Clone, Debug)]
pub struct OscControls {
    pub is_active: bool,
    /// Holds the original [`OscControlConfig`] references and their default
    /// values – runtime values are not included here!
    configs: HashMap<String, OscControlConfig>,
    state: Arc<Mutex<State>>,
}

impl Default for OscControls {
    fn default() -> Self {
        Self {
            configs: HashMap::default(),
            state: Arc::new(Mutex::new(State {
                values: HashMap::default(),
            })),
            is_active: false,
        }
    }
}

impl OscControls {
    pub fn new() -> Self {
        Self::default()
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

impl
    ControlCollection<
        OscControlConfig,
        f32,
        f32,
        HashMap<String, OscControlConfig>,
    > for OscControls
{
    fn add(&mut self, address: &str, config: OscControlConfig) {
        check_address(address);
        self.state.lock().unwrap().set(address, config.value);
        self.configs.insert(address.to_string(), config);
    }

    fn config(&self, name: &str) -> Option<OscControlConfig> {
        self.configs.get(name).cloned()
    }

    fn configs(&self) -> HashMap<String, OscControlConfig> {
        self.configs.clone()
    }

    fn get(&self, address: &str) -> f32 {
        check_address(address);
        self.state.lock().unwrap().get(address)
    }

    fn get_optional(&self, address: &str) -> Option<f32> {
        check_address(address);
        let state = self.state.lock().unwrap();
        state.get_optional(address).copied()
    }

    fn has(&self, address: &str) -> bool {
        check_address(address);
        self.state.lock().unwrap().has(address)
    }

    fn remove(&mut self, address: &str) {
        self.state.lock().unwrap().remove(address);
        self.configs.remove(address);
    }

    fn set(&mut self, address: &str, value: f32) {
        check_address(address);
        self.state.lock().unwrap().set(address, value);
    }

    fn values(&self) -> HashMap<String, f32> {
        return self.state.lock().unwrap().values();
    }

    fn with_values_mut<F>(&mut self, f: F)
    where
        F: FnOnce(&mut HashMap<String, f32>),
    {
        let mut state = self.state.lock().unwrap();
        f(&mut state.values);
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
