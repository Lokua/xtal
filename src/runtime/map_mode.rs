use std::error::Error;
use std::sync::{Arc, Mutex};

use crate::framework::prelude::*;

pub struct MapModeState {
    mappings: HashMap<String, ChannelAndControl>,
    /// Used to store the MSB of an MSB/LSB pair used in 14bit MIDI (CCs 0-31)
    msb_ccs: Vec<ChannelAndControl>,
}

/// Provides live MIDI mapping functionality
pub struct MapMode {
    /// The name of the current slider that has been selected for live mapping
    pub currently_mapping: Option<String>,
    pub state: Arc<Mutex<MapModeState>>,
}

impl Default for MapMode {
    fn default() -> Self {
        Self {
            currently_mapping: None,
            state: Arc::new(Mutex::new(MapModeState {
                mappings: HashMap::default(),
                msb_ccs: vec![],
            })),
        }
    }
}

impl MapMode {
    pub fn mapped(&self, name: &str) -> bool {
        self.state.lock().unwrap().mappings.contains_key(name)
    }

    pub fn formatted_mapping(&self, name: &str) -> String {
        self.state
            .lock()
            .unwrap()
            .mappings
            .get(name)
            .map(|(ch, cc)| format!("{}/{}", ch, cc))
            .unwrap_or_default()
    }

    pub fn proxy_name(name: &str) -> String {
        format!("{}__slider_proxy", name)
    }

    pub fn mappings_as_vec(&self) -> Vec<(String, ChannelAndControl)> {
        self.state
            .lock()
            .unwrap()
            .mappings
            .iter()
            .map(|(k, (ch, cc))| (k.clone(), (*ch, *cc)))
            .collect::<Vec<_>>()
    }

    pub fn has(&self, name: &str) -> bool {
        self.state.lock().unwrap().mappings.contains_key(name)
    }

    pub fn remove(&mut self, name: &str) {
        self.state.lock().unwrap().mappings.remove(name);
    }

    pub fn listen_for_midi(
        &self,
        name: &str,
        hrcc: bool,
    ) -> Result<(), Box<dyn Error>> {
        let state = self.state.clone();
        let name = name.to_owned();

        midi::on_message(
            midi::ConnectionType::Mapping,
            crate::config::MIDI_CONTROL_IN_PORT,
            move |_, msg| {
                if msg.len() < 3 || !midi::is_control_change(msg[0]) {
                    return;
                }

                let mut state = state.lock().unwrap();

                let status = msg[0];
                let ch = status & 0x0F;
                let cc = msg[1];

                // This is a standard 7bit message
                if !hrcc || cc > 63 {
                    state.mappings.insert(name.clone(), (ch, cc));

                    return;
                }

                // This is first of an MSB/LSB pair
                if cc < 32 {
                    let key = (ch, cc);

                    if state.msb_ccs.contains(&key) {
                        warn!(
                            "Received consecutive MSB \
                          without matching LSB"
                        );
                    } else {
                        state.msb_ccs.push(key);
                    }

                    return;
                }

                let msb_cc = cc - 32;
                let msb_key = (ch, msb_cc);

                // This is a regular 32-63 7bit message
                if !state.msb_ccs.contains(&msb_key) {
                    state.mappings.insert(name.clone(), (ch, cc));

                    return;
                }

                // This is the LSB of an MSB/LSB pair

                state.mappings.insert(name.clone(), msb_key);
                state.msb_ccs.retain(|k| *k != msb_key);
            },
        )
    }
}
