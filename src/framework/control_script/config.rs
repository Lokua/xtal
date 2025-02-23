//! Deserialization types needed for converting the Lattice yaml format into
//! controls

use bevy_reflect::Reflect;
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer};

use super::param_mod::ParamValue;
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

    // Modulation & Effects
    #[serde(rename = "mod")]
    Modulation,
    #[serde(rename = "effect")]
    Effects,
}

#[allow(dead_code)]
#[derive(Clone, Deserialize, Debug, Default, Reflect)]
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

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Clone, Deserialize, Debug, Reflect)]
pub struct BreakpointConfig {
    pub position: f32,
    pub value: ParamValue,
    #[serde(flatten)]
    pub kind: KindConfig,
}

#[derive(Clone, Deserialize, Debug, Reflect)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum KindConfig {
    Step,
    Ramp {
        #[serde(default = "default_easing")]
        easing: String,
    },
    Wave {
        #[serde(default = "default_shape")]
        shape: String,
        #[serde(default = "default_param_value_0_25")]
        frequency: ParamValue,
        #[serde(default = "default_param_value_0_25")]
        amplitude: ParamValue,
        #[serde(default = "default_param_value_0_5")]
        width: ParamValue,
        #[serde(default = "default_easing")]
        easing: String,
        #[serde(default = "default_none_string")]
        constrain: String,
    },
    Random {
        #[serde(default = "default_f32_0_25")]
        amplitude: f32,
    },
    RandomSmooth {
        #[serde(default = "default_param_value_0_25")]
        frequency: ParamValue,
        #[serde(default = "default_param_value_0_25")]
        amplitude: ParamValue,
        #[serde(default = "default_easing")]
        easing: String,
        #[serde(default = "default_none_string")]
        constrain: String,
    },
    End,
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

#[derive(Clone, Deserialize, Debug, Reflect)]
pub struct EffectConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    #[serde(flatten)]
    pub kind: EffectKind,
}

#[derive(Clone, Deserialize, Debug, Reflect)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EffectKind {
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

    Math {
        op: String,
        value: ParamValue,
    },

    Quantizer {
        #[serde(default = "default_param_value_0_25")]
        step: ParamValue,
        #[serde(default = "default_normalized_range")]
        range: (f32, f32),
    },

    RingModulator {
        #[serde(default = "default_param_value_0")]
        mix: ParamValue,
        #[serde(default = "default_normalized_range")]
        range: (f32, f32),
        modulator: String,
    },

    Saturator {
        #[serde(default = "default_param_value_1")]
        drive: ParamValue,
        #[serde(default = "default_normalized_range")]
        range: (f32, f32),
    },

    SlewLimiter {
        #[serde(default = "default_param_value_0")]
        rise: ParamValue,
        #[serde(default = "default_param_value_0")]
        fall: ParamValue,
    },

    #[serde()]
    WaveFolder {
        #[serde(default = "default_param_value_1")]
        gain: ParamValue,
        #[serde(default = "default_iterations")]
        iterations: usize,
        #[serde(default = "default_param_value_1")]
        symmetry: ParamValue,
        #[serde(default = "default_param_value_0")]
        bias: ParamValue,
        #[serde(default = "default_param_value_1")]
        shape: ParamValue,
        // TODO: make Option and consider None to mean "adaptive range"?
        #[serde(default = "default_normalized_range")]
        range: (f32, f32),
    },
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
fn default_param_value_0_25() -> ParamValue {
    ParamValue::Cold(0.25)
}
fn default_param_value_0_3() -> ParamValue {
    ParamValue::Cold(0.3)
}
fn default_param_value_0_5() -> ParamValue {
    ParamValue::Cold(0.5)
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
