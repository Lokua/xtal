//! Types and trait implementations to support parameter modulation.
//!
//! # Example
//!
//! In the following example, a 4 beat triangle wave is used as the value of a
//! wave_folder's `symmetry` param.
//!
//! ```yaml
//! t1:
//!   type: triangle
//!   beats: 4
//!   range: [-1, 1]
//!
//! t2:
//!   type: wave_folder
//!   symmetry: $t1
//! ```
use serde::{Deserialize, Deserializer};

use crate::framework::prelude::*;

#[derive(Clone, Debug)]
pub enum ParamValue {
    Cold(f32),
    Hot(String),
}

impl ParamValue {
    pub fn as_float(&self) -> f32 {
        match self {
            ParamValue::Cold(x) => *x,
            ParamValue::Hot(_) => {
                loud_panic!("Cannot get float from ParamValue::Hot")
            }
        }
    }
}

impl From<ParamValue> for f32 {
    fn from(param: ParamValue) -> f32 {
        match param {
            ParamValue::Cold(x) => x,
            ParamValue::Hot(_) => 0.0,
        }
    }
}

impl<'de> Deserialize<'de> for ParamValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Debug, Deserialize)]
        #[serde(untagged)]
        enum RawParam {
            Number(f32),
            String(String),
        }

        let value = RawParam::deserialize(deserializer)?;
        match value {
            RawParam::Number(n) => Ok(ParamValue::Cold(n)),
            RawParam::String(s) if s.starts_with('$') => {
                Ok(ParamValue::Hot(s[1..].to_string()))
            }
            RawParam::String(s) => Err(serde::de::Error::custom(format!(
                "Expected number or string starting with '$', got '{}'",
                s
            ))),
        }
    }
}

pub trait SetFromParam {
    fn set(&mut self, name: &str, value: f32);
}

impl SetFromParam for Hysteresis {
    fn set(&mut self, name: &str, value: f32) {
        match name {
            "lower_threshold" => self.lower_threshold = value,
            "upper_threshold" => self.upper_threshold = value,
            "output_low" => self.output_low = value,
            "output_high" => self.output_high = value,
            _ => warn!("Hysteresis does not support param name {}", name),
        }
    }
}

impl SetFromParam for Quantizer {
    fn set(&mut self, name: &str, value: f32) {
        match name {
            "step" => self.step = value,
            _ => warn!("Quantizer does not support param name {}", name),
        }
    }
}

impl SetFromParam for RingModulator {
    fn set(&mut self, name: &str, value: f32) {
        match name {
            "mix" => self.mix = value,
            _ => warn!("RingModulator does not support param name {}", name),
        }
    }
}

impl SetFromParam for Saturator {
    fn set(&mut self, name: &str, value: f32) {
        match name {
            "drive" => self.drive = value,
            _ => warn!("Saturator does not support param name {}", name),
        }
    }
}

impl SetFromParam for SlewLimiter {
    fn set(&mut self, name: &str, value: f32) {
        match name {
            "rise" => self.rise = value,
            "fall" => self.fall = value,
            _ => warn!("SlewLimiter does not support param name {}", name),
        }
    }
}

impl SetFromParam for WaveFolder {
    fn set(&mut self, name: &str, value: f32) {
        match name {
            "gain" => self.gain = value,
            "symmetry" => self.symmetry = value,
            "bias" => self.bias = value,
            "shape" => self.shape = value,
            _ => warn!("WaveFolder does not support param name {}", name),
        }
    }
}

impl SetFromParam for TestEffect {
    fn set(&mut self, name: &str, value: f32) {
        match name {
            "param" => self.param = value,
            _ => warn!("TestEffect does not support param name {}", name),
        }
    }
}
