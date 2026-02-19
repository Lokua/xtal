use std::error::Error;
use std::sync::{Arc, Mutex};

use super::control_traits::{ControlCollection, ControlConfig};
use crate::framework::midi::{self, is_control_change};
use crate::framework::prelude::*;

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
    configs: HashMap<String, MidiControlConfig>,
    state: Arc<Mutex<State>>,
    port: Option<String>,
    is_active: bool,
}

impl MidiControls {
    pub fn set_port(&mut self, port: String) {
        self.port = if port.is_empty() { None } else { Some(port) };
    }

    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let Some(midi_control_in_port) = self.port.clone() else {
            warn!(
                "Skipping {} listener setup; no MIDI port.",
                midi::ConnectionType::Control
            );
            return Ok(());
        };

        let state = self.state.clone();
        let config_lookup = self.configs_by_channel_and_cc();
        let hrcc = self.hrcc;

        trace!("config_lookup: {:#?}", config_lookup);

        match midi::on_message(
            midi::ConnectionType::Control,
            &midi_control_in_port,
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
                debug!(
                    "MIDI CC input: channel={}, cc={}, value={}, hrcc={}",
                    channel, cc, value, hrcc
                );

                if !hrcc || cc > 63 {
                    if let Some((name, config)) = config_lookup.get(&ch_cc) {
                        let value = value as f32 / 127.0;
                        let mapped_value =
                            value * (config.max - config.min) + config.min;

                        state.lock().unwrap().set(name, mapped_value);

                        trace!("Storing regular 7bit (!hrcc || cc > 63 block)");
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

                let msb = last.unwrap();

                let (name, config) =
                    config_lookup.get(&(channel, msb_cc)).unwrap();

                let msb = msb as u16;
                let lsb = value as u16;
                let value_14bit = (msb << 7) | lsb;
                let normalized_value = value_14bit as f32 / 16_383.0;

                let mapped_value =
                    normalized_value * (config.max - config.min) + config.min;

                state.set(name, mapped_value);
                state.remove_last((channel, msb_cc));

                trace!(
                    "Storing 14bit value. value: {}, norm: {}, mapped: {}",
                    value_14bit, normalized_value, mapped_value
                );
            },
        ) {
            Ok(_) => {
                self.is_active = true;
                info!("Started");
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
        info!("Restarting...");
        self.start()
    }

    pub fn is_active(&self) -> bool {
        self.is_active
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

    pub fn messages_hrcc(&self) -> Vec<[u8; 3]> {
        let values = self.values();
        let mut messages: Vec<[u8; 3]> = vec![];
        debug!("values: {:?}, configs: {:?}", values, self.configs());
        for (name, value) in values.iter() {
            let config = self.configs.get(name).unwrap();
            let status = 0xB0 | config.channel;

            if config.cc < 32 {
                let value_14bit =
                    map_range(*value, config.min, config.max, 0.0, 16_383.0);
                let value_14bit =
                    constrain::clamp(value_14bit, 0.0, 16_383.0) as u16;

                let msb = ((value_14bit >> 7) & 0x7F) as u8;
                let lsb = (value_14bit & 0x7F) as u8;

                messages.push([status, config.cc, msb]);
                messages.push([status, config.cc + 32, lsb]);
            } else {
                let value =
                    map_range(*value, config.min, config.max, 0.0, 127.0);
                let value = constrain::clamp(value, 0.0, 127.0) as u8;
                messages.push([status, config.cc, value]);
            }
        }
        messages
    }

    fn configs_by_channel_and_cc(
        &self,
    ) -> HashMap<ChannelAndController, (String, MidiControlConfig)> {
        self.configs
            .iter()
            .map(|(name, config)| {
                ((config.channel, config.cc), (name.clone(), config.clone()))
            })
            .collect()
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
        self.state.lock().unwrap().set(name, config.value);
        self.configs.insert(name.to_string(), config);
    }

    fn config(&self, name: &str) -> Option<MidiControlConfig> {
        self.configs.get(name).cloned()
    }

    fn configs(&self) -> HashMap<String, MidiControlConfig> {
        self.configs.clone()
    }

    fn get(&self, name: &str) -> f32 {
        self.state.lock().unwrap().get(name)
    }

    fn get_optional(&self, name: &str) -> Option<f32> {
        self.state.lock().unwrap().get_optional(name).copied()
    }

    fn has(&self, name: &str) -> bool {
        self.state.lock().unwrap().has(name)
    }

    fn remove(&mut self, name: &str) {
        self.state.lock().unwrap().remove(name);
        self.configs.remove(name);
    }

    fn set(&mut self, name: &str, value: f32) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_encode_standard_7bit_cc() {
        let mut controls = MidiControls::default();
        controls.add("cutoff", MidiControlConfig::new((0, 74), (0.0, 1.0), 0.0));
        controls.set("cutoff", 1.0);

        let messages = controls.messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], [176, 74, 127]);
    }

    #[test]
    fn messages_hrcc_encode_msb_lsb_for_cc_under_32() {
        let mut controls = MidiControls::default();
        controls.add("fine", MidiControlConfig::new((1, 10), (0.0, 1.0), 0.0));
        controls.add("coarse", MidiControlConfig::new((1, 40), (0.0, 1.0), 0.0));

        controls.set("fine", 1.0);
        controls.set("coarse", 1.0);

        let mut messages = controls.messages_hrcc();
        messages.sort_by_key(|m| (m[1], m[2]));

        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0], [177, 10, 127]);
        assert_eq!(messages[1], [177, 40, 127]);
        assert_eq!(messages[2], [177, 42, 127]);
    }

    #[test]
    fn start_without_port_is_noop() {
        let mut controls = MidiControls::default();
        let result = controls.start();
        assert!(result.is_ok());
        assert!(!controls.is_active());
    }
}

pub type ChannelAndController = (u8, u8);
type Msb = u8;

#[derive(Debug, Default)]
struct State {
    values: HashMap<String, f32>,
    last: HashMap<ChannelAndController, Msb>,
}

impl State {
    fn get(&self, name: &str) -> f32 {
        *self.values.get(name).unwrap_or(&0.0)
    }

    fn get_optional(&self, name: &str) -> Option<&f32> {
        self.values.get(name)
    }

    fn has(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }

    fn remove(&mut self, name: &str) {
        self.values.remove(name);
    }

    fn set(&mut self, name: &str, value: f32) {
        self.values.insert(name.to_string(), value);
    }

    fn values(&self) -> HashMap<String, f32> {
        self.values.clone()
    }

    fn last(&self, ch_cc: ChannelAndController) -> Option<Msb> {
        self.last.get(&ch_cc).copied()
    }

    fn set_last(&mut self, ch_cc: ChannelAndController, msb: Msb) {
        self.last.insert(ch_cc, msb);
    }

    fn remove_last(&mut self, ch_cc: ChannelAndController) {
        self.last.remove(&ch_cc);
    }
}
