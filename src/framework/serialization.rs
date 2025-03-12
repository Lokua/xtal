use std::collections::HashMap;

use chrono::{DateTime, Utc};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use super::control_hub::control_hub::Snapshots;
use super::prelude::*;

pub const VERSION: &'static str = "1.1";

pub struct ConcreteControls {
    pub ui_controls: UiControls,
    pub midi_controls: MidiControls,
    pub osc_controls: OscControls,
    pub snapshots: Snapshots,
}

#[derive(Serialize, Deserialize)]
pub struct SerializableControls {
    version: String,
    // Backwards compat
    #[serde(rename = "ui_controls", alias = "controls")]
    pub ui_controls: Vec<ControlConfig>,
    pub midi_controls: Vec<BasicNameValueConfig>,
    pub osc_controls: Vec<BasicNameValueConfig>,
    // Backwards compat files that don't have snapshots field
    #[serde(default)]
    pub snapshots: HashMap<String, SerializableSnapshot>,
}

#[derive(Serialize, Deserialize)]
pub struct SerializableSnapshot {
    #[serde(rename = "ui_controls", alias = "controls")]
    pub ui_controls: Vec<ControlConfig>,
    pub midi_controls: Vec<BasicNameValueConfig>,
    pub osc_controls: Vec<BasicNameValueConfig>,
}

#[derive(Serialize, Deserialize)]
pub struct ControlConfig {
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
    #[serde(with = "control_value_format")]
    pub value: ControlValue,
}

#[derive(Serialize, Deserialize)]
pub struct BasicNameValueConfig {
    pub name: String,
    pub value: f32,
}

mod control_value_format {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(
        value: &ControlValue,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(f) = value.as_float() {
            return serializer.serialize_f32(f);
        }
        if let Some(s) = value.as_string() {
            return serializer.serialize_str(s);
        }
        if let Some(b) = value.as_bool() {
            return serializer.serialize_bool(b);
        }

        serializer.serialize_f32(0.0)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<ControlValue, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Value {
            Float(f32),
            String(String),
            Bool(bool),
        }

        let value = Value::deserialize(deserializer)?;
        match value {
            Value::Float(f) => Ok(ControlValue::from(f)),
            Value::String(s) => Ok(ControlValue::from(s)),
            Value::Bool(b) => Ok(ControlValue::from(b)),
        }
    }
}

impl From<&ConcreteControls> for SerializableControls {
    fn from(concretes: &ConcreteControls) -> Self {
        let controls = concretes
            .ui_controls
            .configs()
            .iter()
            .filter_map(|c| {
                if c.is_separator() {
                    None
                } else {
                    let value = concretes.ui_controls.values().get(c.name());
                    Some(ControlConfig {
                        kind: c.variant_string(),
                        name: c.name().to_string(),
                        value: value.unwrap_or(&c.value()).clone(),
                    })
                }
            })
            .collect();

        let midi_controls = concretes
            .midi_controls
            .values()
            .iter()
            .map(|(name, value)| BasicNameValueConfig {
                name: name.clone(),
                value: *value,
            })
            .collect();

        let osc_controls = concretes
            .osc_controls
            .values()
            .iter()
            .map(|(name, value)| BasicNameValueConfig {
                name: name.clone(),
                value: *value,
            })
            .collect();

        let snapshots = concretes
            .snapshots
            .iter()
            .map(|(name, snapshot)| {
                (
                    name.clone(),
                    create_serializable_snapshot(concretes, snapshot),
                )
            })
            .collect();

        Self {
            version: VERSION.to_string(),
            ui_controls: controls,
            midi_controls,
            osc_controls,
            snapshots,
        }
    }
}

fn create_serializable_snapshot(
    concretes: &ConcreteControls,
    snapshot: &FxHashMap<String, ControlValue>,
) -> SerializableSnapshot {
    let mut controls = Vec::new();
    for (name, value) in snapshot {
        if let Some(config) = concretes.ui_controls.config(name) {
            controls.push(ControlConfig {
                kind: config.variant_string(),
                name: name.clone(),
                value: value.clone(),
            });
        }
    }

    let mut midi_controls = Vec::new();
    for (name, value) in snapshot {
        if concretes.midi_controls.has(name) {
            midi_controls.push(BasicNameValueConfig {
                name: name.clone(),
                value: value.as_float().unwrap(),
            });
        }
    }

    let mut osc_controls = Vec::new();
    for (name, value) in snapshot {
        if concretes.osc_controls.has(name) {
            osc_controls.push(BasicNameValueConfig {
                name: name.clone(),
                value: value.as_float().unwrap(),
            });
        }
    }

    SerializableSnapshot {
        ui_controls: controls,
        midi_controls,
        osc_controls,
    }
}

impl ConcreteControls {
    pub fn merge_serializable_values(
        (serializable_controls, concrete_controls): (
            SerializableControls,
            &mut ConcreteControls,
        ),
    ) -> &mut ConcreteControls {
        concrete_controls
            .ui_controls
            .values_mut()
            .iter_mut()
            .for_each(|(name, value)| {
                let s = serializable_controls
                    .ui_controls
                    .iter()
                    .find(|s| s.name == *name)
                    .map(|s| s.value.clone());

                if let Some(s) = s {
                    *value = ControlValue::from(s);
                }
            });

        concrete_controls.midi_controls.with_values_mut(|values| {
            values.iter_mut().for_each(|(name, value)| {
                let s = serializable_controls
                    .midi_controls
                    .iter()
                    .find(|s| s.name == *name)
                    .map(|s| s.value);

                if let Some(s) = s {
                    *value = s
                }
            });
        });

        concrete_controls.osc_controls.with_values_mut(|values| {
            values.iter_mut().for_each(|(name, value)| {
                let s = serializable_controls
                    .osc_controls
                    .iter()
                    .find(|s| s.name == *name)
                    .map(|s| s.value);

                if let Some(s) = s {
                    *value = s
                }
            });
        });

        concrete_controls.snapshots.clear();

        for (snapshot_name, serializable_snapshot) in
            serializable_controls.snapshots
        {
            let mut snapshot_values = FxHashMap::default();

            for control in &serializable_snapshot.ui_controls {
                snapshot_values
                    .insert(control.name.clone(), control.value.clone());
            }

            for midi_control in &serializable_snapshot.midi_controls {
                snapshot_values.insert(
                    midi_control.name.clone(),
                    ControlValue::from(midi_control.value),
                );
            }

            for osc_control in &serializable_snapshot.osc_controls {
                snapshot_values.insert(
                    osc_control.name.clone(),
                    ControlValue::from(osc_control.value),
                );
            }

            concrete_controls
                .snapshots
                .insert(snapshot_name, snapshot_values);
        }

        concrete_controls
    }
}

// -----------------------------------------------------------------------------
// Image Index
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageIndex {
    pub items: Vec<ImageIndexItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageIndexItem {
    pub filename: String,
    pub created_at: String,
}
