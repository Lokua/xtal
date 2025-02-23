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

use bevy_reflect::{Reflect, ReflectRef};
use serde::{Deserialize, Deserializer};

use super::config::{BreakpointConfig, EffectConfig, TriangleConfig};
use crate::framework::prelude::*;

#[derive(Clone, Debug, Reflect)]
pub enum ParamValue {
    Cold(f32),
    Hot(String),
}

impl ParamValue {
    /// This should only be called after the dep_graph has been resolved and
    /// [`SetFromParam::from_cold_params`] has been called
    pub fn as_float(&self) -> f32 {
        match self {
            ParamValue::Cold(x) => *x,
            ParamValue::Hot(_) => {
                panic!(
                    r#"
                    Cannot get float from ParamValue::Hot. 
                    Make sure Hot values have been resolved into Cold. 
                    ParamValue: {:?}"#,
                    self
                )
            }
        }
    }

    /// Receive the wrapped if [`Self::Cold`], otherwise execute `f` in case of
    /// [`Self::Hot`] with Hot String.
    pub fn cold_or(&self, f: impl Fn(String) -> f32) -> f32 {
        match self {
            Self::Cold(x) => *x,
            Self::Hot(name) => f(name.clone()),
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

//------------------------------------------------------------------------------
// Effects
//------------------------------------------------------------------------------

/// Used for part 1 of an Effect's instantiation phase
pub trait FromColdParams: Default + SetFromParam {
    /// Extract the f32s from [`ParamValue::Cold`] variants and sets them on a
    /// newly created Effect instance. Will use the Effect's default instead of
    /// [`ParamValue::Hot`] since those are swapped in during
    /// [`ControlScript::get`]. Important that this _only_ deals with ParamValue
    /// (f32) - you still need to deal with copying the non-ParamValues from the
    /// EffectConfig to the Effect instance manually
    fn from_cold_params(config: &EffectConfig) -> Self {
        let mut instance = Self::default();

        let kind_reflect: &dyn Reflect = &config.kind;

        if let ReflectRef::Enum(enum_ref) = kind_reflect.reflect_ref() {
            for field_index in 0..enum_ref.field_len() {
                if let Some(field_value) = enum_ref.field_at(field_index) {
                    let field_name =
                        enum_ref.name_at(field_index).unwrap_or("");

                    if let ReflectRef::Enum(inner_enum) =
                        &field_value.reflect_ref()
                    {
                        if inner_enum.variant_name() == "Cold" {
                            if let Some(inner_value) = inner_enum.field_at(0) {
                                if let Some(value) =
                                    inner_value.try_downcast_ref::<f32>()
                                {
                                    instance.set_from_param(field_name, *value);
                                }
                            }
                        }
                    }
                }
            }
        }

        instance
    }
}

impl FromColdParams for Hysteresis {}
impl FromColdParams for Math {}
impl FromColdParams for Quantizer {}
impl FromColdParams for RingModulator {}
impl FromColdParams for Saturator {}
impl FromColdParams for SlewLimiter {}
impl FromColdParams for WaveFolder {}

pub trait SetFromParam {
    fn set_from_param(&mut self, name: &str, value: f32);
}

fn warn_for(effect: &str, field: &str) {
    warn!("{} does not support field: {}", effect, field);
}

impl SetFromParam for Hysteresis {
    fn set_from_param(&mut self, name: &str, value: f32) {
        match name {
            "lower_threshold" => self.lower_threshold = value,
            "upper_threshold" => self.upper_threshold = value,
            "output_low" => self.output_low = value,
            "output_high" => self.output_high = value,
            _ => warn_for("Hysteresis", name),
        }
    }
}

impl SetFromParam for Math {
    fn set_from_param(&mut self, name: &str, value: f32) {
        match name {
            "value" => self.value = value,
            _ => warn_for("Math", name),
        }
    }
}

impl SetFromParam for Quantizer {
    fn set_from_param(&mut self, name: &str, value: f32) {
        match name {
            "step" => self.step = value,
            _ => warn_for("Quantizer", name),
        }
    }
}

impl SetFromParam for RingModulator {
    fn set_from_param(&mut self, name: &str, value: f32) {
        match name {
            "mix" => self.mix = value,
            _ => warn_for("RingModulator", name),
        }
    }
}

impl SetFromParam for Saturator {
    fn set_from_param(&mut self, name: &str, value: f32) {
        match name {
            "drive" => self.drive = value,
            _ => warn_for("Saturator", name),
        }
    }
}

impl SetFromParam for SlewLimiter {
    fn set_from_param(&mut self, name: &str, value: f32) {
        match name {
            "rise" => self.rise = value,
            "fall" => self.fall = value,
            _ => warn_for("SlewLimiter", name),
        }
    }
}

impl SetFromParam for WaveFolder {
    fn set_from_param(&mut self, name: &str, value: f32) {
        match name {
            "gain" => self.gain = value,
            "symmetry" => self.symmetry = value,
            "bias" => self.bias = value,
            "shape" => self.shape = value,
            _ => warn_for("WaveFolder", name),
        }
    }
}

//------------------------------------------------------------------------------
// Animation
//------------------------------------------------------------------------------

impl SetFromParam for TriangleConfig {
    fn set_from_param(&mut self, name: &str, value: f32) {
        match name {
            "beats" => self.beats = ParamValue::Cold(value),
            "phase" => self.phase = ParamValue::Cold(value),
            _ => warn_for("Triangle", name),
        }
    }
}

impl From<BreakpointConfig> for Breakpoint {
    fn from(config: BreakpointConfig) -> Self {
        let mut breakpoint = Breakpoint {
            position: config.position,
            value: match &config.value {
                ParamValue::Cold(v) => *v,
                ParamValue::Hot(_) => 0.0,
            },
            kind: Kind::Step,
        };

        let kind_reflect: &dyn Reflect = &config.kind;
        if let ReflectRef::Enum(enum_ref) = kind_reflect.reflect_ref() {
            let variant_name = enum_ref.variant_name();
            breakpoint.kind = Kind::default_for_variant(variant_name);

            for field_index in 0..enum_ref.field_len() {
                if let Some(field_value) = enum_ref.field_at(field_index) {
                    let field_name =
                        enum_ref.name_at(field_index).unwrap_or("");

                    if let ReflectRef::Enum(param_enum) =
                        field_value.reflect_ref()
                    {
                        if param_enum.variant_name() == "Cold" {
                            if let Some(param_field) = param_enum.field_at(0) {
                                if let Some(value) =
                                    param_field.try_downcast_ref::<f32>()
                                {
                                    breakpoint
                                        .set_from_param(field_name, *value);
                                }
                            }
                        }
                    }
                }
            }
        }

        breakpoint
    }
}

impl SetFromParam for Breakpoint {
    fn set_from_param(&mut self, name: &str, value: f32) {
        if name == "value" {
            self.value = value;
            return;
        }

        let path_segments: Vec<&str> = name.split(".").collect();

        if path_segments.len() == 3 && path_segments[0] == "breakpoints" {
            let key = path_segments[2];

            let result = match self.kind {
                Kind::Step => match key {
                    "value" => Ok(self.value = value),
                    _ => Err(()),
                },
                Kind::Random {
                    ref mut amplitude, ..
                } => match key {
                    "value" => Ok(self.value = value),
                    "amplitude" => Ok(*amplitude = value),
                    _ => Err(()),
                },
                Kind::RandomSmooth {
                    ref mut amplitude,
                    ref mut frequency,
                    ..
                } => match key {
                    "value" => Ok(self.value = value),
                    "amplitude" => Ok(*amplitude = value),
                    "frequency" => Ok(*frequency = value),
                    _ => Err(()),
                },
                Kind::Wave {
                    ref mut amplitude,
                    ref mut frequency,
                    ref mut width,
                    ..
                } => match key {
                    "value" => Ok(self.value = value),
                    "amplitude" => Ok(*amplitude = value),
                    "width" => Ok(*width = value),
                    "frequency" => Ok(*frequency = value),
                    _ => Err(()),
                },
                _ => Err(()),
            };

            if result.is_err() {
                warn_for("Breakpoint", name);
            }
        }
    }
}
