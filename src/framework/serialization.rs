use serde::{Deserialize, Serialize};

use super::prelude::*;

pub struct ConcreteControls {
    controls: Controls,
    midi_controls: MidiControls,
    osc_controls: OscControls,
}

#[derive(Serialize, Deserialize)]
pub struct SerializableControls {
    version: String,
    controls: Vec<ControlConfig>,
    midi_controls: Vec<BasicNameValueConfig>,
    osc_controls: Vec<BasicNameValueConfig>,
}

#[derive(Serialize, Deserialize)]
struct ControlConfig {
    #[serde(rename = "type")]
    kind: String,
    name: String,
    value: ControlValue,
}

#[derive(Serialize, Deserialize)]
struct BasicNameValueConfig {
    name: String,
    value: f32,
}

impl From<ConcreteControls> for SerializableControls {
    fn from(concretes: ConcreteControls) -> Self {
        let controls = concretes
            .controls
            .items()
            .iter()
            .map(|c| ControlConfig {
                kind: c.variant_string(),
                name: c.name().to_string(),
                value: c.value(),
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

        Self {
            version: "1".to_string(),
            controls,
            midi_controls,
            osc_controls,
        }
    }
}

impl ConcreteControls {
    pub fn merge_serializable_values(
        (serializable_controls, concrete_controls): (
            SerializableControls,
            &mut ConcreteControls,
        ),
    ) -> &mut ConcreteControls {
        concrete_controls.controls.values_mut().iter_mut().for_each(
            |(name, value)| {
                let s = serializable_controls
                    .controls
                    .iter()
                    .find(|s| s.name == *name)
                    .map(|s| s.value.clone());

                if let Some(s) = s {
                    *value = ControlValue::from(s);
                }
            },
        );

        concrete_controls.midi_controls.with_values_mut(|values| {
            values.iter_mut().for_each(|(name, value)| {
                let s = serializable_controls
                    .controls
                    .iter()
                    .find(|s| s.name == *name)
                    .map(|s| s.value.as_float().unwrap_or(0.0));

                if let Some(s) = s {
                    *value = s
                }
            });
        });

        concrete_controls.osc_controls.with_values_mut(|values| {
            values.iter_mut().for_each(|(name, value)| {
                let s = serializable_controls
                    .controls
                    .iter()
                    .find(|s| s.name == *name)
                    .map(|s| s.value.as_float().unwrap_or(0.0));

                if let Some(s) = s {
                    *value = s
                }
            });
        });

        concrete_controls
    }
}
