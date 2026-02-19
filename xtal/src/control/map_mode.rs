//! Provides runtime mapping of MIDI CCs to UI sliders, AKA "MIDI learn".
use std::error::Error;
use std::fmt;
use std::sync::{Arc, Mutex};

use crate::core::prelude::*;
use crate::io::midi;

pub type ChannelAndController = (usize, usize);
pub type Mappings = HashMap<String, ChannelAndController>;

#[derive(Debug)]
pub struct MapModeState {
    mappings: Mappings,
    /// Stores MSB keys for pending HRCC MSB/LSB pairs.
    msb_ccs: Vec<ChannelAndController>,
}

/// Live MIDI-learn state and runtime mapping storage.
pub struct MapMode {
    /// Name of slider currently selected for live mapping.
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

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.state.lock().unwrap().mappings.clear();
    }

    /// Start listening for Control Change messages to learn one mapping target.
    pub fn start<F>(
        &self,
        name: &str,
        midi_control_in_port: &str,
        hrcc: bool,
        callback: F,
    ) -> Result<(), Box<dyn Error>>
    where
        F: Fn(Result<(), MappingError>) + Send + Sync + 'static,
    {
        if midi_control_in_port.is_empty() {
            warn!(
                "Skipping {} listener setup; no MIDI port.",
                midi::ConnectionType::Mapping
            );
            return Ok(());
        }

        let state = self.state.clone();
        let name = name.to_owned();
        let midi_control_in_port = midi_control_in_port.to_string();

        midi::on_message(
            midi::ConnectionType::Mapping,
            &midi_control_in_port,
            move |_, msg| {
                if msg.len() < 3 || !midi::is_control_change(msg[0]) {
                    return;
                }

                let mut state = state.lock().unwrap();

                let status = msg[0];
                let ch = (status & 0x0F) as usize;
                let cc = msg[1] as usize;

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
            Self::DuplicateMappings(removed_mappings) => write!(
                f,
                "Mapping the same MIDI controller to multiple destinations is not supported. Removed: {:?}",
                removed_mappings
            ),
            Self::ConsecutiveHrccMsb => {
                write!(f, "Received consecutive MSB without matching LSB")
            }
        }
    }
}

impl std::error::Error for MappingError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_conflicts_keeps_last_mapping() {
        let mut mappings = Mappings::default();
        mappings.insert("a".to_string(), (0, 1));
        mappings.insert("b".to_string(), (0, 1));
        mappings.insert("c".to_string(), (0, 2));

        let removed = MapMode::remove_conflicts(&mut mappings, "c", (0, 1));

        assert_eq!(removed.len(), 2);
        assert!(!mappings.contains_key("a"));
        assert!(!mappings.contains_key("b"));
        assert!(mappings.contains_key("c"));
    }

    #[test]
    fn start_without_port_is_noop() {
        let mode = MapMode::default();
        let result = mode.start("ax", "", false, |_| {});
        assert!(result.is_ok());
    }
}
