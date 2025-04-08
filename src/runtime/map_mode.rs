use std::error::Error;
use std::sync::{Arc, Mutex};

use crate::framework::prelude::*;

pub type Mappings = HashMap<String, ChannelAndController>;

pub struct MapModeState {
    mappings: HashMap<String, ChannelAndController>,
    /// Used to store the MSB of an MSB/LSB pair used in 14bit MIDI (CCs 0-31)
    msb_ccs: Vec<ChannelAndController>,
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
    const PROXY_NAME_SUFFIX: &str = "__slider_proxy";

    /// Mappings are stored as normal [`MidiControlConfig`] instances within a
    /// [`ControlHub`]'s [`MidiControls`] instance. When a [`Slider`] is queried
    /// via [`ControlHub::get`], we first check if there is a "MIDI proxy" for
    /// the slider and if so return the value of the MIDI control instead. For
    /// that reason this method _probably_ shouldn't be here as it's not really
    /// MapMode's concern. Anyway this method just provides a single interface
    /// to make sure every call site is using the same name suffix
    pub fn proxy_name(name: &str) -> String {
        format!("{}{}", name, Self::PROXY_NAME_SUFFIX)
    }

    /// The inverse of [`proxy_name`]
    pub fn unproxied_name(proxy_name: &str) -> Option<String> {
        proxy_name
            .strip_suffix(Self::PROXY_NAME_SUFFIX)
            .map(|s| s.to_string())
    }

    pub fn is_proxy_name(name: &str) -> bool {
        name.ends_with(Self::PROXY_NAME_SUFFIX)
    }

    pub fn mappings(&self) -> Mappings {
        let state = self.state.lock().unwrap();
        state.mappings.clone()
    }

    pub fn mappings_as_vec(&self) -> Vec<(String, ChannelAndController)> {
        self.state
            .lock()
            .unwrap()
            .mappings
            .iter()
            .map(|(k, (ch, cc))| (k.clone(), (*ch, *cc)))
            .collect::<Vec<_>>()
    }

    pub fn set_mappings(&mut self, mappings: Mappings) {
        let mut state = self.state.lock().unwrap();
        state.mappings = mappings;
    }

    pub fn update_from_vec(&mut self, ms: &[(String, ChannelAndController)]) {
        let mut state = self.state.lock().unwrap();
        for m in ms {
            state.mappings.insert(m.0.clone(), m.1);
        }
    }

    pub fn has(&self, name: &str) -> bool {
        self.state.lock().unwrap().mappings.contains_key(name)
    }

    pub fn remove(&mut self, name: &str) {
        self.state.lock().unwrap().mappings.remove(name);
    }

    pub fn clear(&mut self) {
        self.state.lock().unwrap().mappings.clear();
    }

    pub fn start<F>(
        &self,
        name: &str,
        hrcc: bool,
        callback: F,
    ) -> Result<(), Box<dyn Error>>
    where
        F: Fn() + Send + Sync + 'static,
    {
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
                    callback();
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
                    callback();
                    return;
                }

                // This is the LSB of an MSB/LSB pair

                state.mappings.insert(name.clone(), msb_key);
                state.msb_ccs.retain(|k| *k != msb_key);
                callback();
            },
        )
    }

    pub fn stop(&mut self) {
        self.currently_mapping = None;
        midi::disconnect(midi::ConnectionType::Mapping);
    }
}
