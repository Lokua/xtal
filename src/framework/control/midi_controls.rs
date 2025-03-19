//! Control sketch parameters with MIDI.
//!
//! Sketches do not need to interact with this module directly - see
//! [`ControlHub`].
use nannou::math::map_range;
use std::error::Error;
use std::sync::{Arc, Mutex};

use crate::framework::midi::is_control_change;
use crate::framework::prelude::*;

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

pub type ChannelAndControl = (u8, u8);
type Msb = u8;

#[derive(Debug, Default)]
struct MidiState {
    values: HashMap<String, f32>,
    last: HashMap<ChannelAndControl, Msb>,
}

impl MidiState {
    fn new() -> Self {
        Self {
            values: HashMap::default(),
            last: HashMap::default(),
        }
    }

    fn get(&self, name: &str) -> f32 {
        *self.values.get(name).unwrap_or(&0.0)
    }

    fn set(&mut self, name: &str, value: f32) {
        self.values.insert(name.to_string(), value);
    }

    fn has(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }

    fn values(&self) -> HashMap<String, f32> {
        self.values.clone()
    }

    fn last(&self, ch_cc: ChannelAndControl) -> Option<Msb> {
        self.last.get(&ch_cc).copied()
    }

    fn set_last(&mut self, ch_cc: ChannelAndControl, msb: Msb) {
        self.last.insert(ch_cc, msb);
    }

    fn remove_last(&mut self, ch_cc: ChannelAndControl) {
        self.last.remove(&ch_cc);
    }
}

#[derive(Clone, Debug)]
pub struct MidiControls {
    /// "High Resolution CC" AKA 14bit MIDI control change for CCs 0-31
    pub hrcc: bool,
    configs: HashMap<String, MidiControlConfig>,
    state: Arc<Mutex<MidiState>>,
    is_active: bool,
}

impl Default for MidiControls {
    fn default() -> Self {
        Self {
            configs: HashMap::default(),
            state: Arc::new(Mutex::new(MidiState::new())),
            is_active: false,
            hrcc: false,
        }
    }
}

impl MidiControls {
    pub fn new() -> Self {
        Self::default()
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

    pub fn update_value(&mut self, name: &str, value: f32) {
        self.state.lock().unwrap().set(name, value);
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

    pub fn configs(&self) -> HashMap<String, MidiControlConfig> {
        self.configs.clone()
    }

    fn configs_by_channel_and_cc(
        &self,
    ) -> HashMap<ChannelAndControl, (String, MidiControlConfig)> {
        self.configs()
            .iter()
            .map(|(name, config)| {
                ((config.channel, config.cc), (name.clone(), config.clone()))
            })
            .collect()
    }

    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let state = self.state.clone();
        let config_lookup = self.configs_by_channel_and_cc();
        let hrcc = self.hrcc;

        trace!("config_lookup: {:#?}", config_lookup);

        match midi::on_message(
            midi::ConnectionType::Control,
            crate::config::MIDI_CONTROL_IN_PORT,
            move |_, message| {
                if message.len() < 3 || !is_control_change(message[0]) {
                    return;
                }

                trace!("on_message {}", "-".repeat(24));
                trace!("raw: {:?}", message);

                let status = message[0];
                let channel = status & 0x0F;
                let cc = message[1];
                let ch_cc = (channel, cc);
                let value = message[2];

                // This is a regular 7bit message
                if !hrcc || cc > 63 {
                    if let Some((name, config)) = config_lookup.get(&ch_cc) {
                        let value = value as f32 / 127.0;
                        let mapped_value =
                            value * (config.max - config.min) + config.min;

                        state.lock().unwrap().set(name, mapped_value);

                        trace!("Storing regular 7bit (!hrcc ** > 63 block)");
                    }

                    return;
                }

                if cc < 32 {
                    if !config_lookup.contains_key(&ch_cc) {
                        return;
                    }

                    let mut state = state.lock().unwrap();

                    if state.last(ch_cc).is_some() {
                        warn!("Received consecutive MSB without matching LSB");
                    }

                    let value_14bit = value as u16 * 128;
                    let msb = (value_14bit >> 7) as u8;

                    state.set_last(ch_cc, msb);

                    trace!("Storing MSB");

                    return;
                }

                let mut state = state.lock().unwrap();
                let msb_cc = cc - 32;
                let last = state.last((channel, msb_cc));

                // This is a regular 32-63 7bit message
                if last.is_none() {
                    if let Some((name, config)) = config_lookup.get(&ch_cc) {
                        let value = message[2] as f32 / 127.0;
                        let mapped_value =
                            value * (config.max - config.min) + config.min;

                        state.set(name, mapped_value);

                        trace!("Storing regular 7bit (32-63 block)");
                    }

                    return;
                }

                // This is the LSB of an MSB/LSB pair

                let msb = last.unwrap();

                let (name, config) =
                    config_lookup.get(&(channel, msb_cc)).unwrap();

                let msb = msb as u16;
                let lsb = value as u16;
                let value_14bit = (msb << 7) | lsb;
                let normalized_value = value_14bit as f32 / 16383.0;

                let mapped_value =
                    normalized_value * (config.max - config.min) + config.min;

                state.set(name, mapped_value);
                state.remove_last((channel, msb_cc));

                trace!(
                    "Storing 14bit value. value: {}, norm: {}, mapped: {}",
                    value_14bit,
                    normalized_value,
                    mapped_value
                );
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

    pub fn restart(&mut self) -> Result<(), Box<dyn Error>> {
        self.is_active = false;
        self.start()
    }

    pub fn messages(&self) -> Vec<[u8; 3]> {
        let values = self.values();
        let mut messages: Vec<[u8; 3]> = vec![];
        for (name, value) in values.iter() {
            let mut message: [u8; 3] = [0; 3];
            let config = self.configs.get(name).unwrap();
            message[0] = 176 + config.channel;
            message[1] = config.cc;
            let value = map_range(*value, config.min, config.max, 0.0, 127.0);
            let value = constrain::clamp(value, 0.0, 127.0);
            message[2] = value.round() as u8;
            messages.push(message);
        }
        messages
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }
}

pub struct MidiControlBuilder {
    controls: MidiControls,
}

impl Default for MidiControlBuilder {
    fn default() -> Self {
        Self {
            controls: MidiControls::new(),
        }
    }
}

impl MidiControlBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn control(
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

    pub fn control_n(
        mut self,
        name: &str,
        midi: (u8, u8),
        default: f32,
    ) -> Self {
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
