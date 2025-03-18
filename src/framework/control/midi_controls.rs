//! Control sketch parameters with MIDI.
//!
//! Sketches do not need to interact with this module directly - see
//! [`ControlHub`].
use nannou::math::map_range;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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

type ChannelAndControl = (u8, u8);
type TimestampAndMsb = (u64, u8);

#[derive(Debug, Default)]
struct MidiState {
    values: HashMap<String, f32>,
    last: HashMap<ChannelAndControl, TimestampAndMsb>,
}

impl MidiState {
    fn new() -> Self {
        Self {
            values: HashMap::new(),
            last: HashMap::new(),
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

    fn last(&self, ch_cc: ChannelAndControl) -> Option<TimestampAndMsb> {
        self.last.get(&ch_cc).copied()
    }

    fn set_last(&mut self, ch_cc: ChannelAndControl, ts_msb: TimestampAndMsb) {
        self.last.insert(ch_cc, ts_msb);
    }

    fn remove_last(&mut self, ch_cc: ChannelAndControl) {
        self.last.remove(&ch_cc);
    }
}

#[derive(Clone, Debug)]
pub struct MidiControls {
    configs: HashMap<String, MidiControlConfig>,
    state: Arc<Mutex<MidiState>>,
    is_active: bool,
}

impl Default for MidiControls {
    fn default() -> Self {
        Self {
            configs: HashMap::new(),
            state: Arc::new(Mutex::new(MidiState::new())),
            is_active: false,
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

        debug!("config_lookup: {:#?}", config_lookup);

        match midi::on_message(
            midi::ConnectionType::Control,
            crate::config::MIDI_CONTROL_IN_PORT,
            #[cfg(feature = "hi_res_cc")]
            move |this_ts, message| {
                if message.len() < 3 || !is_control_change(message[0]) {
                    return;
                }

                trace!("on_message {}", "-".repeat(24));
                trace!("raw: {:?}", message);

                let channel = message[0] & 0x0F;
                let cc = message[1];
                let value = message[2];

                if let Some((name, config)) = config_lookup.get(&(channel, cc))
                {
                    trace!("Config exists for ch: {}, cc: {}", channel, cc);
                    let mut state = state.lock().unwrap();

                    // This is a standard 7bit message
                    if cc > 31 {
                        let mapped_value = (value as f32 / 127.0)
                            * (config.max - config.min)
                            + config.min;

                        state.set(name, mapped_value);
                        trace!(
                            "Setting std 7bit ch: {}, cc: {}, v: {}, mapped: {}",
                            channel,
                            cc,
                            message[2],
                            mapped_value
                        );
                    }
                    // This is the 1st of an MSB/LSB pair (MSB)
                    else {
                        let ch_cc = (channel, cc);

                        if state.last(ch_cc).is_some() {
                            warn!(
                                "Received a 2nd MSB for the same \
                                ch_cc key before receiving a matching LSB. \
                                It's likely your MIDI controller isn't hi-res \
                                and you should run the program without the \
                                hi_res_cc feature enabled"
                            );
                        }

                        let value_14bit = value as u16 * 128;
                        let value_msb = (value_14bit >> 7) as u8;
                        let ts_msb = (this_ts, value_msb);

                        state.set_last(ch_cc, ts_msb);
                        trace!("Storing MSB - {:?}, {:?}", ch_cc, ts_msb);
                    }
                }
                // This might be the 2nd of an MSB/LSB pair (LSB)
                else if cc > 31 && cc <= 63 {
                    let mut state = state.lock().unwrap();
                    let msb_cc = cc - 32;
                    let last = state.last((channel, msb_cc));

                    if last.is_none() {
                        return;
                    }

                    let (last_ts, msb_value) = last.unwrap();

                    state.remove_last((channel, msb_cc));

                    let difference = Duration::from_millis(this_ts)
                        - Duration::from_micros(last_ts);

                    // FIXME: We actually don't know what time units are
                    // being used. Windows may be using millis; CoreMIDI
                    // uses micros...
                    if difference > Duration::from_millis(5) {
                        let (name, config) =
                            config_lookup.get(&(channel, msb_cc)).unwrap();

                        let msb = msb_value as u16;
                        let lsb = value as u16;
                        let value_14bit = (msb << 7) | lsb;
                        let normalized_value = value_14bit as f32 / 16383.0;

                        let mapped_value = normalized_value
                            * (config.max - config.min)
                            + config.min;

                        state.set(name, mapped_value);
                        trace!(
                            "Setting 14-bit for ch: {}, cc: {}, n_value: {}, mapped: {}",
                            channel,
                            msb_cc,
                            normalized_value,
                            mapped_value
                        );
                    } else {
                        trace!(
                            "Timeout for MSB/LSB pair; difference (ms): {}",
                            difference.as_millis()
                        );
                    }
                } else {
                    trace!("?");
                }
            },
            #[cfg(not(feature = "hi_res_cc"))]
            move |_stamp, message| {
                if message.len() < 3 || !is_control_change(message[0]) {
                    return;
                }

                let status = message[0];
                let channel = status & 0x0F;
                let cc = message[1];
                let value = message[2] as f32 / 127.0;

                if let Some((name, config)) = config_lookup.get(&(channel, cc))
                {
                    let mapped_value =
                        value * (config.max - config.min) + config.min;

                    trace!(
                        "Msg ch: {}, cc: {}, v: {}, mapped: {}",
                        channel,
                        cc,
                        message[2],
                        mapped_value
                    );

                    state.lock().unwrap().set(name, mapped_value);
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
