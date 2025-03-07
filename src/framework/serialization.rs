use serde::{Deserialize, Serialize};

use super::prelude::*;

pub struct ConcreteControls {
    pub controls: Controls,
    pub midi_controls: MidiControls,
    pub osc_controls: OscControls,
}

#[derive(Serialize, Deserialize)]
pub struct SerializableControls {
    pub version: String,
    pub controls: Vec<ControlConfig>,
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

impl From<ConcreteControls> for SerializableControls {
    fn from(concretes: ConcreteControls) -> Self {
        let controls = concretes
            .controls
            .items()
            .iter()
            .map(|c| {
                let value = concretes.controls.values().get(c.name());
                ControlConfig {
                    kind: c.variant_string(),
                    name: c.name().to_string(),
                    value: value.unwrap_or(&c.value()).clone(),
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
