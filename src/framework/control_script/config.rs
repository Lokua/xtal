//! Deserialization types needed for converting the Lattice yaml format into
//! controls

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer};
use std::str::FromStr;

use super::param_mod::{ParamValue, SetFromParam};
use crate::framework::prelude::*;

//------------------------------------------------------------------------------
// Top-level Types
//------------------------------------------------------------------------------

pub type ConfigFile = IndexMap<String, MaybeControlConfig>;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum MaybeControlConfig {
    Control(ControlConfig),
    #[allow(dead_code)]
    Other(serde_yml::Value),
}

#[derive(Deserialize, Debug)]
pub struct ControlConfig {
    #[serde(rename = "type")]
    pub control_type: ControlType,
    #[serde(flatten)]
    pub config: serde_yml::Value,
}

#[derive(Deserialize, Debug)]
pub enum ControlType {
    // UI controls
    #[serde(rename = "slider")]
    Slider,
    #[serde(rename = "checkbox")]
    Checkbox,
    #[serde(rename = "select")]
    Select,
    #[serde(rename = "separator")]
    Separator,

    // External control
    #[serde(rename = "osc")]
    Osc,
    #[serde(rename = "audio")]
    Audio,

    // Animation
    #[serde(rename = "lerp_abs")]
    LerpAbs,
    #[serde(rename = "lerp_rel")]
    LerpRel,
    #[serde(rename = "r_ramp_rel")]
    RRampRel,
    #[serde(rename = "triangle")]
    Triangle,
    #[serde(rename = "automate")]
    Automate,
    #[serde(rename = "test_anim")]
    TestAnim,

    // Modulation & Effects
    #[serde(rename = "mod")]
    Modulation,
    #[serde(rename = "effect")]
    Effects,
}

#[allow(dead_code)]
#[derive(Clone, Deserialize, Debug, Default)]
struct Shared {
    #[serde(default, deserialize_with = "deserialize_number_or_none")]
    pub bypass: Option<f32>,
    #[serde(default)]
    pub var: Option<String>,
}

//------------------------------------------------------------------------------
// UI
//------------------------------------------------------------------------------

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct SliderConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    pub range: [f32; 2],
    pub default: f32,
    pub step: f32,
}

impl Default for SliderConfig {
    fn default() -> Self {
        Self {
            shared: Shared::default(),
            range: [0.0, 1.0],
            default: 0.0,
            step: 0.000_1,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct CheckboxConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    pub default: bool,
}

impl Default for CheckboxConfig {
    fn default() -> Self {
        Self {
            shared: Shared::default(),
            default: false,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct SelectConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    pub options: Vec<String>,
    pub default: String,
}

#[derive(Deserialize, Debug)]
struct Separator {}

//------------------------------------------------------------------------------
// External
//------------------------------------------------------------------------------

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct OscConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    pub range: [f32; 2],
    pub default: f32,
}

impl Default for OscConfig {
    fn default() -> Self {
        Self {
            shared: Shared::default(),
            range: [0.0, 1.0],
            default: 0.0,
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
#[serde(default)]
pub struct AudioConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    pub channel: usize,
    pub slew: [f32; 2],
    pub pre: f32,
    pub detect: f32,
    pub range: [f32; 2],
    pub bypass: Option<f32>,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            shared: Shared::default(),
            channel: 0,
            slew: [0.0, 0.0],
            pre: 0.0,
            detect: 0.0,
            range: [0.0, 1.0],
            bypass: None,
        }
    }
}

//------------------------------------------------------------------------------
// Animation
//------------------------------------------------------------------------------

#[derive(Debug)]
pub enum AnimationConfig {
    LerpRel(LerpRelConfig),
    LerpAbs(LerpAbsConfig),
    RRampRel(RRampRelConfig),
    Triangle(TriangleConfig),
    Automate(AutomateConfig),
    TestAnim(TestAnimConfig),
}

impl AnimationConfig {
    pub fn delay(&self) -> f32 {
        match self {
            AnimationConfig::LerpRel(x) => x.delay,
            AnimationConfig::LerpAbs(x) => x.delay,
            AnimationConfig::RRampRel(x) => x.delay,
            _ => 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub enum KeyframeSequence {
    Linear(Vec<Keyframe>),
    Random(Vec<KeyframeRandom>),
    Breakpoints(Vec<Breakpoint>),
    None,
}

#[derive(Clone, Deserialize, Debug)]
pub struct LerpAbsConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    #[serde(default)]
    pub delay: f32,
    pub keyframes: Vec<(String, f32)>,
}

impl Default for LerpAbsConfig {
    fn default() -> Self {
        Self {
            shared: Shared::default(),
            delay: 0.0,
            keyframes: Vec::new(),
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
pub struct LerpRelConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    #[serde(default)]
    pub delay: f32,
    pub keyframes: Vec<(f32, f32)>,
}

impl Default for LerpRelConfig {
    fn default() -> Self {
        Self {
            shared: Shared::default(),
            delay: 0.0,
            keyframes: Vec::new(),
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
pub struct RRampRelConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    #[serde(default)]
    pub delay: f32,
    #[serde(default)]
    pub ramp_time: f32,
    #[serde(default = "default_ramp")]
    pub ramp: String,
    pub keyframes: Vec<(f32, (f32, f32))>,
}

impl Default for RRampRelConfig {
    fn default() -> Self {
        Self {
            shared: Shared::default(),
            delay: 0.0,
            ramp: "linear".to_string(),
            ramp_time: 0.25,
            keyframes: Vec::new(),
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
#[serde(default)]
pub struct TriangleConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    pub beats: ParamValue,
    pub range: [f32; 2],
    pub phase: ParamValue,
}

impl Default for TriangleConfig {
    fn default() -> Self {
        Self {
            shared: Shared::default(),
            beats: ParamValue::Cold(1.0),
            range: [0.0, 1.0],
            phase: ParamValue::Cold(0.0),
        }
    }
}

impl SetFromParam for TriangleConfig {
    fn set(&mut self, name: &str, value: f32) {
        match name {
            "beats" => self.beats = ParamValue::Cold(value),
            "phase" => self.phase = ParamValue::Cold(value),
            _ => {
                warn!("{} is not a supported ParamValue", name)
            }
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct AutomateConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    pub breakpoints: Vec<BreakpointConfig>,
    #[serde(default = "default_mode")]
    pub mode: String,
}

impl Default for AutomateConfig {
    fn default() -> Self {
        Self {
            shared: Shared::default(),
            breakpoints: Vec::new(),
            mode: "loop".to_string(),
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
pub struct BreakpointConfig {
    #[serde(alias = "pos", alias = "x")]
    pub position: f32,
    #[serde(alias = "val", alias = "y")]
    pub value: f32,
    #[serde(flatten)]
    pub kind: KindConfig,
}

impl From<BreakpointConfig> for Breakpoint {
    fn from(config: BreakpointConfig) -> Self {
        match config.kind {
            KindConfig::Step => Breakpoint::step(config.position, config.value),
            KindConfig::Ramp { easing } => Breakpoint::ramp(
                config.position,
                config.value,
                Easing::from_str(&easing).unwrap(),
            ),
            KindConfig::Wave {
                shape,
                frequency,
                amplitude,
                width,
                easing,
                constrain,
            } => Breakpoint::wave(
                config.position,
                config.value,
                Shape::from_str(shape.as_str()).unwrap(),
                frequency,
                width,
                amplitude,
                Easing::from_str(&easing).unwrap(),
                Constrain::try_from((constrain.as_str(), 0.0, 1.0)).unwrap(),
            ),
            KindConfig::Random { amplitude } => {
                Breakpoint::random(config.position, config.value, amplitude)
            }
            KindConfig::RandomSmooth {
                frequency,
                amplitude,
                easing,
                constrain,
            } => Breakpoint::random_smooth(
                config.position,
                config.value,
                frequency,
                amplitude,
                Easing::from_str(&easing).unwrap(),
                Constrain::try_from((constrain.as_str(), 0.0, 1.0)).unwrap(),
            ),
            KindConfig::End => Breakpoint::end(config.position, config.value),
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum KindConfig {
    Step,
    Ramp {
        #[serde(default = "default_easing", alias = "ease")]
        easing: String,
    },
    Wave {
        #[serde(alias = "shape", default = "default_shape")]
        shape: String,
        #[serde(alias = "freq", default = "default_f32_0_25")]
        frequency: f32,
        #[serde(alias = "amp", default = "default_f32_0_25")]
        amplitude: f32,
        #[serde(alias = "width", default = "default_f32_0_5")]
        width: f32,
        #[serde(alias = "ease", default = "default_easing")]
        easing: String,
        #[serde(alias = "cons", default = "default_none_string")]
        constrain: String,
    },
    Random {
        #[serde(alias = "amp", default = "default_f32_0_25")]
        amplitude: f32,
    },
    RandomSmooth {
        #[serde(alias = "freq", default = "default_f32_0_25")]
        frequency: f32,
        #[serde(alias = "amp", default = "default_f32_0_25")]
        amplitude: f32,
        #[serde(alias = "ease", default = "default_easing")]
        easing: String,
        #[serde(alias = "cons", default = "default_none_string")]
        constrain: String,
    },
    End,
}

#[derive(Clone, Deserialize, Debug)]
pub struct TestAnimConfig {
    pub field: ParamValue,
}

impl SetFromParam for TestAnimConfig {
    fn set(&mut self, name: &str, value: f32) {
        match name {
            "field" => self.field = ParamValue::Cold(value),
            _ => {
                warn!("TestAnimConfig does not support param name {}", name)
            }
        }
    }
}

//------------------------------------------------------------------------------
// Modulation & Effects
//------------------------------------------------------------------------------

#[derive(Clone, Deserialize, Debug)]
pub struct ModulationConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    pub source: String,
    pub modulators: Vec<String>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct EffectConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    #[serde(flatten)]
    pub kind: EffectKind,
}

impl From<EffectConfig> for Effect {
    fn from(config: EffectConfig) -> Self {
        match config.kind {
            EffectKind::Hysteresis {
                lower_threshold,
                upper_threshold,
                output_low,
                output_high,
                pass_through,
            } => Effect::Hysteresis(Hysteresis::new(
                lower_threshold.as_float(),
                upper_threshold.as_float(),
                output_low.as_float(),
                output_high.as_float(),
                pass_through,
            )),
            EffectKind::Quantizer { step, range } => {
                Effect::Quantizer(Quantizer::new(step.as_float(), range))
            }
            EffectKind::RingModulator { mix, range, .. } => {
                Effect::RingModulator(RingModulator::new(mix.as_float(), range))
            }
            EffectKind::Saturator { drive, range } => {
                Effect::Saturator(Saturator::new(drive.as_float(), range))
            }
            EffectKind::SlewLimiter { rise, fall } => Effect::SlewLimiter(
                SlewLimiter::new(rise.as_float(), fall.as_float()),
            ),
            EffectKind::WaveFolder {
                gain,
                iterations,
                symmetry,
                bias,
                shape,
                range,
            } => Effect::WaveFolder(WaveFolder::new(
                gain.as_float(),
                iterations,
                symmetry.as_float(),
                bias.as_float(),
                shape.as_float(),
                range,
            )),
            EffectKind::Test { param } => Effect::TestEffect(TestEffect {
                param: f32::from(param),
            }),
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EffectKind {
    #[serde(alias = "hyst", alias = "hys")]
    Hysteresis {
        #[serde(default = "default_param_value_0_3")]
        lower_threshold: ParamValue,
        #[serde(default = "default_param_value_0_7")]
        upper_threshold: ParamValue,
        #[serde(default = "default_param_value_0")]
        output_low: ParamValue,
        #[serde(default = "default_param_value_1")]
        output_high: ParamValue,
        #[serde(default = "default_false")]
        pass_through: bool,
    },

    #[serde(alias = "quant")]
    Quantizer {
        #[serde(default = "default_param_value_0_25")]
        step: ParamValue,
        #[serde(default = "default_normalized_range")]
        range: (f32, f32),
    },

    #[serde(alias = "rm", alias = "ring")]
    RingModulator {
        #[serde(default = "default_param_value_0")]
        mix: ParamValue,
        #[serde(default = "default_normalized_range")]
        range: (f32, f32),
        modulator: String,
    },

    #[serde(alias = "saturate", alias = "sat")]
    Saturator {
        #[serde(default = "default_param_value_1")]
        drive: ParamValue,
        #[serde(default = "default_normalized_range")]
        range: (f32, f32),
    },

    #[serde(alias = "slew")]
    SlewLimiter {
        #[serde(default = "default_param_value_0")]
        rise: ParamValue,
        #[serde(default = "default_param_value_0")]
        fall: ParamValue,
    },

    #[serde(alias = "fold")]
    WaveFolder {
        #[serde(default = "default_param_value_1")]
        gain: ParamValue,
        #[serde(alias = "iter", default = "default_iterations")]
        iterations: usize,
        #[serde(alias = "sym", default = "default_param_value_1")]
        symmetry: ParamValue,
        #[serde(default = "default_param_value_0")]
        bias: ParamValue,
        #[serde(default = "default_param_value_1")]
        shape: ParamValue,
        // TODO: make Option and consider None to mean "adaptive range"?
        #[serde(default = "default_normalized_range")]
        range: (f32, f32),
    },

    #[serde(alias = "test")]
    Test { param: ParamValue },
}

//------------------------------------------------------------------------------
// Helper Types & Functions
//------------------------------------------------------------------------------

fn deserialize_number_or_none<'de, D>(
    deserializer: D,
) -> Result<Option<f32>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumericOrOther {
        Num(f32),
        Other(()),
    }

    match NumericOrOther::deserialize(deserializer) {
        Ok(NumericOrOther::Num(n)) => Ok(Some(n)),
        _ => Ok(None),
    }
}

fn default_ramp() -> String {
    "linear".to_string()
}
fn default_iterations() -> usize {
    1
}
fn default_normalized_range() -> (f32, f32) {
    (0.0, 1.0)
}
fn default_mode() -> String {
    "loop".to_string()
}
fn default_easing() -> String {
    "linear".to_string()
}
fn default_shape() -> String {
    "sine".to_string()
}
fn default_none_string() -> String {
    "none".to_string()
}
fn default_false() -> bool {
    false
}
fn default_f32_0_25() -> f32 {
    0.25
}
fn default_f32_0_5() -> f32 {
    0.5
}
fn default_param_value_0_3() -> ParamValue {
    ParamValue::Cold(0.3)
}
fn default_param_value_0_7() -> ParamValue {
    ParamValue::Cold(0.7)
}
fn default_param_value_0() -> ParamValue {
    ParamValue::Cold(0.0)
}
fn default_param_value_1() -> ParamValue {
    ParamValue::Cold(1.0)
}
fn default_param_value_0_25() -> ParamValue {
    ParamValue::Cold(0.25)
}
