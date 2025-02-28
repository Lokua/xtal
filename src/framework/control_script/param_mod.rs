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

use std::str::FromStr;

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
    /// [`FromColdParams::from_cold_params`] has been called
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

    /// Receive the wrapped float if [`Self::Cold`], otherwise execute `f` in
    /// case of [`Self::Hot`] with Hot String.
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
    /// EffectConfig to the Effect instance manually.
    ///
    /// # IMPORTANT!
    /// This _may_ suffer from the same issue that was addressed in [this
    /// commit](https://github.com/Lokua/lattice/commit/05f5aa6) (non-param
    /// fields being stuck with their defaults instead of the config values)
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
    warn_once!("{} does not support field: {}", effect, field);
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
            "operand" => self.operand = value,
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

/// See the [parameter handling documentation](../docs/parameter_handling.md)
/// for details on how different parameter types are processed.
impl From<BreakpointConfig> for Breakpoint {
    fn from(config: BreakpointConfig) -> Self {
        let kind_reflect: &dyn Reflect = &config.kind;
        let variant_name =
            if let ReflectRef::Enum(enum_ref) = kind_reflect.reflect_ref() {
                enum_ref.variant_name()
            } else {
                "Step"
            };

        let mut breakpoint = Breakpoint {
            position: config.position,
            value: match &config.value {
                ParamValue::Cold(v) => *v,
                ParamValue::Hot(_) => 0.0,
            },
            kind: Kind::default_for_variant_str(variant_name),
        };

        if let ReflectRef::Enum(enum_ref) = kind_reflect.reflect_ref() {
            for field_index in 0..enum_ref.field_len() {
                if let Some(field_value) = enum_ref.field_at(field_index) {
                    if let Some(field_name) = enum_ref.name_at(field_index) {
                        if let ReflectRef::Enum(inner_enum) =
                            field_value.reflect_ref()
                        {
                            let inner_variant = inner_enum.variant_name();

                            if inner_variant == "Cold" {
                                if let Some(param_field) =
                                    inner_enum.field_at(0)
                                {
                                    if let Some(value) =
                                        param_field.try_downcast_ref::<f32>()
                                    {
                                        breakpoint
                                            .set_field(field_name, *value);
                                    }
                                }
                            } else if inner_variant == "Hot" {
                                // Hot params get skipped and live with their
                                // defaults until they are replaced at "get"
                                // time from the dep_graph and `set_from_param`
                            }
                        } else {
                            // non-ParamValue fields like Easing, Constrain,
                            // Shape, etc.
                            if let Some(reflect_value) =
                                field_value.try_as_reflect()
                            {
                                breakpoint.set_non_param_field(
                                    field_name,
                                    reflect_value,
                                );
                            }
                        }
                    }
                }
            }
        }

        breakpoint
    }
}

impl Breakpoint {
    /// See the [parameter handling
    /// documentation](../docs/parameter_handling.md) for details on how
    /// different parameter types are processed.
    fn set_field(&mut self, name: &str, value: f32) {
        if name == "value" {
            self.value = value;
            return;
        }

        match self.kind {
            Kind::Step => {}
            Kind::Random {
                ref mut amplitude, ..
            } => {
                if name == "amplitude" {
                    *amplitude = value;
                }
            }
            Kind::RandomSmooth {
                ref mut amplitude,
                ref mut frequency,
                ..
            } => match name {
                "amplitude" => *amplitude = value,
                "frequency" => *frequency = value,
                _ => {}
            },
            Kind::Wave {
                ref mut amplitude,
                ref mut frequency,
                ref mut width,
                ..
            } => match name {
                "amplitude" => *amplitude = value,
                "frequency" => *frequency = value,
                "width" => *width = value,
                _ => {}
            },
            _ => {
                warn_for("Breakpoint", name);
            }
        }
    }

    /// See the [parameter handling
    /// documentation](../docs/parameter_handling.md) for details on how
    /// different parameter types are processed.
    fn set_non_param_field(&mut self, name: &str, value: &dyn Reflect) {
        match self.kind {
            Kind::Ramp { ref mut easing } => {
                if name == "easing" {
                    if let Some(str_value) = value.downcast_ref::<String>() {
                        if let Ok(parsed_easing) = Easing::from_str(str_value) {
                            *easing = parsed_easing;
                        }
                    }
                }
            }
            _ => {
                warn!("No handler for non-param field: {}", name);
            }
        }
    }
}

impl SetFromParam for Breakpoint {
    /// See the [parameter handling
    /// documentation](../docs/parameter_handling.md) for details on how
    /// different parameter types are processed.
    fn set_from_param(&mut self, name: &str, value: f32) {
        let path_segments: Vec<&str> = name.split('.').collect();

        match path_segments.len() {
            1 => {
                self.set_field(path_segments[0], value);
            }
            3 if path_segments[0] == "breakpoints" => {
                self.set_field(path_segments[2], value);
            }
            _ => {
                warn_for("Breakpoint", name);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framework::control_script::config::KindConfig;

    #[test]
    fn test_breakpoint_ramp_conversion() {
        let config = BreakpointConfig {
            position: 0.0,
            value: ParamValue::Cold(100.0),
            kind: KindConfig::Ramp {
                easing: "ease_in".into(),
            },
        };

        let breakpoint = Breakpoint::from(config);

        assert_eq!(breakpoint.position, 0.0);
        assert_eq!(breakpoint.value, 100.0);

        if let Kind::Ramp { easing } = breakpoint.kind {
            assert_eq!(easing, Easing::EaseIn);
        } else {
            panic!("Expected Kind::Ramp");
        }
    }

    #[test]
    fn test_breakpoint_random_conversion() {
        let config = BreakpointConfig {
            position: 0.0,
            value: ParamValue::Cold(100.0),
            kind: KindConfig::Random {
                amplitude: ParamValue::Cold(50.0),
            },
        };

        let breakpoint = Breakpoint::from(config);

        assert_eq!(breakpoint.position, 0.0);
        assert_eq!(breakpoint.value, 100.0);

        if let Kind::Random { amplitude } = breakpoint.kind {
            assert_eq!(amplitude, 50.0);
        } else {
            panic!("Expected Kind::Random");
        }
    }
}
