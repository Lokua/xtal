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
//!
//! See the [parameter handling documentation](link) for details on how
//! different parameter types are processed.
//!
//! [link]: https://github.com/Lokua/xtal/blob/main/docs/parameter_handling.md

use serde::{Deserialize, Deserializer};
use std::str::FromStr;

use super::config::*;
use crate::framework::prelude::*;

#[derive(Clone, Debug)]
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

pub trait SetFromParam {
    fn set_from_param(&mut self, name: &str, value: f32);
}

fn warn_for(thing: &str, field: &str) {
    warn_once!("{} does not support field: {}", thing, field);
}

//------------------------------------------------------------------------------
// Effects
//------------------------------------------------------------------------------

/// Used for part 1 of an Effect's instantiation phase (TODO: document more
/// specifically how this works in the greater scheme)
pub trait FromColdParams: Default + SetFromParam {
    /// Extract the f32s from [`ParamValue::Cold`] variants and sets them on a
    /// newly created Effect instance. Will use the Effect's default instead of
    /// [`ParamValue::Hot`] since those are swapped in during
    /// [`ControlHub::get`].
    fn from_cold_params(config: &EffectConfig) -> Self;
}

fn apply_if_cold<T: SetFromParam>(
    instance: &mut T,
    param: &ParamValue,
    field: &str,
) {
    if let ParamValue::Cold(value) = param {
        instance.set_from_param(field, *value);
    }
}

/// Generate [`FromColdParams`] and [`SetFromParam`] implementations for an effect
macro_rules! impl_effect_params {
    ($type:ty, $variant:path, $($field:ident),*) => {
        impl FromColdParams for $type {
            fn from_cold_params(config: &EffectConfig) -> Self {
                let mut instance = Self::default();

                if let $variant { $($field),*, .. } = &config.kind {
                    $(apply_if_cold(&mut instance, $field, stringify!($field));)*
                }

                instance
            }
        }

        impl SetFromParam for $type {
            fn set_from_param(&mut self, name: &str, value: f32) {
                match name {
                    $(stringify!($field) => self.$field = value,)*
                    _ => warn_for(stringify!($type), name),
                }
            }
        }
    };
}

impl_effect_params!(
    Hysteresis,
    EffectKind::Hysteresis,
    lower_threshold,
    upper_threshold,
    output_low,
    output_high
);
impl_effect_params!(Math, EffectKind::Math, operand);
impl_effect_params!(Quantizer, EffectKind::Quantizer, step);
impl_effect_params!(RingModulator, EffectKind::RingModulator, mix);
impl_effect_params!(Saturator, EffectKind::Saturator, drive);
impl_effect_params!(SlewLimiter, EffectKind::SlewLimiter, rise, fall);
impl_effect_params!(
    WaveFolder,
    EffectKind::WaveFolder,
    gain,
    symmetry,
    bias,
    shape
);

//------------------------------------------------------------------------------
// Animation
//------------------------------------------------------------------------------

impl SetFromParam for RampConfig {
    fn set_from_param(&mut self, name: &str, value: f32) {
        match name {
            "beats" => self.beats = ParamValue::Cold(value),
            "phase" => self.phase = ParamValue::Cold(value),
            _ => warn_for("Triangle", name),
        }
    }
}

impl SetFromParam for RandomConfig {
    fn set_from_param(&mut self, name: &str, value: f32) {
        match name {
            "beats" => self.beats = ParamValue::Cold(value),
            "delay" => self.delay = ParamValue::Cold(value),
            _ => warn_for("Random", name),
        }
    }
}

impl SetFromParam for RandomSlewedConfig {
    fn set_from_param(&mut self, name: &str, value: f32) {
        match name {
            "beats" => self.beats = ParamValue::Cold(value),
            "delay" => self.delay = ParamValue::Cold(value),
            "slew" => self.slew = ParamValue::Cold(value),
            _ => warn_for("RandomSlewed", name),
        }
    }
}

impl SetFromParam for TriangleConfig {
    fn set_from_param(&mut self, name: &str, value: f32) {
        match name {
            "beats" => self.beats = ParamValue::Cold(value),
            "phase" => self.phase = ParamValue::Cold(value),
            _ => warn_for("Triangle", name),
        }
    }
}

fn cold_or_default(param: &ParamValue, default: f32) -> f32 {
    match param {
        ParamValue::Cold(v) => *v,
        ParamValue::Hot(_) => default,
    }
}

impl From<BreakpointConfig> for Breakpoint {
    fn from(config: BreakpointConfig) -> Self {
        let position = cold_or_default(&config.position, 0.0);
        let value = cold_or_default(&config.value, 0.0);

        let mut breakpoint = Breakpoint {
            position,
            value,
            kind: Kind::Step,
        };

        match &config.kind {
            KindConfig::Step => {
                breakpoint.kind = Kind::Step;
            }
            KindConfig::Ramp { easing } => {
                let easing = Easing::from_str(easing).unwrap_or(Easing::Linear);
                breakpoint.kind = Kind::Ramp { easing };
            }
            KindConfig::Random { amplitude } => {
                let amplitude = cold_or_default(amplitude, 0.0);
                breakpoint.kind = Kind::Random { amplitude };
            }
            KindConfig::RandomSmooth {
                amplitude,
                frequency,
                easing,
                constrain,
            } => {
                let amplitude = cold_or_default(amplitude, 0.0);
                let frequency = cold_or_default(frequency, 0.0);
                let easing = Easing::from_str(easing).unwrap_or(Easing::Linear);
                let constrain =
                    Constrain::try_from((constrain.as_str(), 0.0, 1.0))
                        .unwrap_or(Constrain::None);

                breakpoint.kind = Kind::RandomSmooth {
                    amplitude,
                    frequency,
                    easing,
                    constrain,
                };
            }
            KindConfig::Wave {
                amplitude,
                frequency,
                width,
                easing,
                shape,
                constrain,
            } => {
                let amplitude = cold_or_default(amplitude, 0.0);
                let frequency = cold_or_default(frequency, 0.0);
                let width = cold_or_default(width, 0.5);
                let easing = Easing::from_str(easing).unwrap_or(Easing::Linear);
                let shape = Shape::from_str(shape).unwrap_or(Shape::Sine);
                let constrain =
                    Constrain::try_from((constrain.as_str(), 0.0, 1.0))
                        .unwrap_or(Constrain::None);

                breakpoint.kind = Kind::Wave {
                    amplitude,
                    frequency,
                    width,
                    easing,
                    shape,
                    constrain,
                };
            }
            KindConfig::End => {
                breakpoint.kind = Kind::End;
            }
        }

        breakpoint
    }
}

impl Breakpoint {
    fn set_field(&mut self, name: &str, value: f32) {
        if name == "value" {
            self.value = value;
            return;
        }
        if name == "position" {
            self.position = value;
            return;
        }

        match self.kind {
            Kind::Step => {}
            Kind::Random { ref mut amplitude } => {
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
}

impl SetFromParam for Breakpoint {
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

    #[test]
    fn test_breakpoint_ramp_conversion() {
        let config = BreakpointConfig {
            position: ParamValue::Cold(0.0),
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
            position: ParamValue::Cold(0.0),
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
