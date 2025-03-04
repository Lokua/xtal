use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};

use super::prelude::*;

#[derive(Clone, Debug)]
pub struct MidiControlConfig {
    pub channel: u8,
    pub cc: u8,
    pub min: f32,
    pub max: f32,
    pub default: f32,
}

impl MidiControlConfig {
    pub fn new(midi: (u8, u8), range: (f32, f32), default: f32) -> Self {
        let (channel, cc) = midi;
        let (min, max) = range;
        Self {
            channel,
            cc,
            min,
            max,
            default,
        }
    }
}

#[derive(Default)]
struct MidiState {
    values: HashMap<String, f32>,
}

impl MidiState {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn get(&self, name: &str) -> f32 {
        *self.values.get(name).unwrap_or(&0.0)
    }

    pub fn set(&mut self, name: &str, value: f32) {
        self.values.insert(name.to_string(), value);
    }

    pub fn has(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }

    pub fn values(&self) -> HashMap<String, f32> {
        return self.values.clone();
    }
}

pub struct MidiControls {
    configs: HashMap<String, MidiControlConfig>,
    state: Arc<Mutex<MidiState>>,
    is_active: bool,
}

impl MidiControls {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            state: Arc::new(Mutex::new(MidiState::new())),
            is_active: false,
        }
    }

    pub fn add(&mut self, name: &str, config: MidiControlConfig) {
        self.state.lock().unwrap().set(name, config.default);
        self.configs.insert(name.to_string(), config);
    }

    pub fn get(&self, name: &str) -> f32 {
        self.state.lock().unwrap().get(name)
    }

    pub fn set(&self, name: &str, value: f32) {
        self.state.lock().unwrap().set(name, value)
    }

    pub fn has(&self, name: &str) -> bool {
        self.state.lock().unwrap().has(name)
    }

    pub fn values(&self) -> HashMap<String, f32> {
        return self.state.lock().unwrap().values();
    }

    pub fn update_value(&mut self, name: &str, value: f32) {
        self.state.lock().unwrap().set(&name, value);
    }
    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let state = self.state.clone();
        let configs = self.configs.clone();

        match midi::on_message(
            midi::ConnectionType::Control,
            crate::config::MIDI_CONTROL_PORT,
            move |message| {
                if message.len() < 3 {
                    return;
                }

                let status = message[0];
                let channel = status & 0x0F;
                let cc = message[1];
                let value = message[2] as f32 / 127.0;

                for (name, config) in configs.iter() {
                    if config.channel == channel && config.cc == cc {
                        let mapped_value =
                            value * (config.max - config.min) + config.min;

                        trace!(
                            "Message - \
                                channel: {}, \
                                cc: {}, \
                                value: {}, \
                                mapped: {}",
                            channel,
                            cc,
                            message[2],
                            mapped_value
                        );

                        state.lock().unwrap().set(name, mapped_value);
                    }
                }
            },
        ) {
            Ok(_) => {
                self.is_active = true;
                info!("MidiControls initialized successfully");
                Ok(())
            }
            Err(e) => {
                self.is_active = false;
                warn!(
                    "Failed to initialize MidiControls: {}. \
                        Using default values.",
                    e
                );
                Err(e)
            }
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }
}

pub struct MidiControlBuilder {
    controls: MidiControls,
}

impl MidiControlBuilder {
    pub fn new() -> Self {
        Self {
            controls: MidiControls::new(),
        }
    }

    pub fn control_mapped(
        mut self,
        name: &str,
        midi: (u8, u8),
        range: (f32, f32),
        default: f32,
    ) -> Self {
        self.controls
            .add(name, MidiControlConfig::new(midi, range, default));
        self
    }

    pub fn control(mut self, name: &str, midi: (u8, u8), default: f32) -> Self {
        self.controls
            .add(name, MidiControlConfig::new(midi, (0.0, 1.0), default));
        self
    }

    pub fn build(mut self) -> MidiControls {
        self.controls
            .start()
            .expect("Unable to build start MIDI receiver");
        self.controls
    }
}
