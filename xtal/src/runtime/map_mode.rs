//! Provides runtime mapping of MIDI CCs to UI sliders, AKA "MIDI learn"
use std::error::Error;
use std::fmt;
use std::sync::{Arc, Mutex};

use crate::framework::prelude::*;

pub type Mappings = HashMap<String, ChannelAndController>;

pub struct MapModeState {
    mappings: Mappings,
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
    /// [`ControlHub`]'s [`ControlHub::midi_controls`] field. When a Slider
    /// is queried via [`ControlHub::get`], we first check if there is a "MIDI
    /// proxy" for the slider within the midi_controls and if so return the
    /// value of the MIDI control instead. This name is how we determine that.
    pub fn proxy_name(name: &str) -> String {
        format!("{}{}", name, Self::PROXY_NAME_SUFFIX)
    }

    /// The inverse of [`Self::proxy_name`]
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

    pub fn set_mappings(&mut self, mappings: Mappings) {
        let mut state = self.state.lock().unwrap();
        state.mappings = mappings;
    }

    pub fn remove(&mut self, name: &str) {
        self.state.lock().unwrap().mappings.remove(name);
    }

    pub fn clear(&mut self) {
        self.state.lock().unwrap().mappings.clear();
    }

    /// Start listening for Control Change messages. When a message is deemed
    /// complete,`callback` will be called with any removed mappings that shared
    /// the same channel and CC as the one it just received since we don't
    /// support mapping the same controller to multiple destinations
    pub fn start<F>(
        &self,
        name: &str,
        hrcc: bool,
        callback: F,
    ) -> Result<(), Box<dyn Error>>
    where
        F: Fn(Result<(), MappingError>) + Send + Sync + 'static,
    {
        let state = self.state.clone();
        let name = name.to_owned();

        midi::on_message(
            midi::ConnectionType::Mapping,
            &crate::global::midi_control_in_port(),
            move |_, msg| {
                if !midi::is_control_change(msg[0]) {
                    return;
                }

                let mut state = state.lock().unwrap();

                let status = msg[0];
                let ch = status & 0x0F;
                let cc = msg[1];

                // This is a standard 7bit message
                if !hrcc || cc > 63 {
                    let removed_mappings = Self::remove_conflicts(
                        &mut state.mappings,
                        &name,
                        (ch, cc),
                    );
                    state.mappings.insert(name.clone(), (ch, cc));
                    if removed_mappings.is_empty() {
                        callback(Ok(()));
                    } else {
                        callback(Err(MappingError::DuplicateMappings(
                            removed_mappings,
                        )));
                    }
                    return;
                }

                // This is first of an MSB/LSB pair
                if cc < 32 {
                    let key = (ch, cc);

                    if state.msb_ccs.contains(&key) {
                        callback(Err(MappingError::ConsecutiveHrccMsb));
                    } else {
                        state.msb_ccs.push(key);
                        callback(Ok(()));
                    }

                    return;
                }

                let msb_cc = cc - 32;
                let msb_key = (ch, msb_cc);

                // This is a regular 32-63 7bit message
                if !state.msb_ccs.contains(&msb_key) {
                    let removed_mappings = Self::remove_conflicts(
                        &mut state.mappings,
                        &name,
                        (ch, cc),
                    );
                    state.mappings.insert(name.clone(), (ch, cc));
                    if removed_mappings.is_empty() {
                        callback(Ok(()));
                    } else {
                        callback(Err(MappingError::DuplicateMappings(
                            removed_mappings,
                        )));
                    }
                    return;
                }

                // This is the LSB of an MSB/LSB pair

                let removed_mappings =
                    Self::remove_conflicts(&mut state.mappings, &name, msb_key);

                state.mappings.insert(name.clone(), msb_key);
                state.msb_ccs.retain(|k| *k != msb_key);

                if removed_mappings.is_empty() {
                    callback(Ok(()));
                } else {
                    callback(Err(MappingError::DuplicateMappings(
                        removed_mappings,
                    )));
                }
            },
        )
    }

    pub fn stop(&mut self) {
        self.currently_mapping = None;
        midi::disconnect(midi::ConnectionType::Mapping);
    }

    /// Helper used to prevent mapping the same (ch, cc) pair to more than one
    /// control which we don't and likely will never support - last one wins
    fn remove_conflicts(
        mappings: &mut Mappings,
        name: &str,
        ch_cc: ChannelAndController,
    ) -> Vec<String> {
        let keys_to_remove: Vec<String> = mappings
            .iter()
            .filter(|(n, (ch, cc))| {
                *n != name && *ch == ch_cc.0 && *cc == ch_cc.1
            })
            .map(|(key, _)| key.clone())
            .collect();

        for key in &keys_to_remove {
            mappings.remove(key);
        }

        keys_to_remove
    }
}

#[derive(Debug)]
pub enum MappingError {
    DuplicateMappings(Vec<String>),
    ConsecutiveHrccMsb,
}

impl fmt::Display for MappingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateMappings(removed_mappings) => {
                write!(
                    f,
                    "Mapping the same MIDI controller to multiple destinations \
                    is not supported. Removed: {:?}",
                    removed_mappings
                )
            }
            Self::ConsecutiveHrccMsb => {
                write!(f, "Received consecutive MSB without matching LSB")
            }
        }
    }
}

impl std::error::Error for MappingError {}
