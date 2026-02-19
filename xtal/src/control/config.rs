//! Deserialization types needed for converting the Xtal yaml format into
//! controls

use std::error::Error;
use std::fmt;

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize};

use super::param_mod::ParamValue;
use crate::core::prelude::*;

//------------------------------------------------------------------------------
// Top-level Types
//------------------------------------------------------------------------------

/// Uses [`IndexMap`] so we maintain the exact order of UI controls that are
/// declared in yaml
pub type ConfigFile = IndexMap<String, MaybeControlConfig>;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum MaybeControlConfig {
    Control(ScriptedControlConfig),
    #[allow(dead_code)]
    Other(serde_yml::Value),
}

#[derive(Deserialize, Debug)]
pub struct ScriptedControlConfig {
    #[serde(rename = "type")]
    pub control_type: ControlType,
    #[serde(flatten)]
    pub config: serde_yml::Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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
    #[serde(rename = "midi")]
    Midi,
    #[serde(rename = "osc")]
    Osc,
    #[serde(rename = "audio")]
    Audio,

    // Animation
    #[serde(rename = "automate")]
    Automate,
    #[serde(rename = "ramp")]
    Ramp,
    #[serde(rename = "random")]
    Random,
    #[serde(rename = "random_slewed")]
    RandomSlewed,
    #[serde(rename = "round_robin")]
    RoundRobin,
    #[serde(rename = "triangle")]
    Triangle,
    #[serde(rename = "snapshot_sequence")]
    SnapshotSequence,

    // Modulation & Effects
    #[serde(rename = "mod")]
    Modulation,
    #[serde(rename = "effect")]
    Effects,
}

#[allow(dead_code)]
#[derive(Clone, Deserialize, Debug, Default)]
pub struct Shared {
    #[serde(default, deserialize_with = "deserialize_number_or_none")]
    pub bypass: Option<f32>,
    #[serde(default)]
    pub var: Option<String>,
    // TODO: this really shouldn't be on shared because only UI controls use it
    #[serde(default, deserialize_with = "to_disabled_fn")]
    pub disabled: Option<DisabledConfig>,
}

//------------------------------------------------------------------------------
// UI
//------------------------------------------------------------------------------

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct SliderConfig {
    #[serde(flatten)]
    pub shared: Shared,
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

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct CheckboxConfig {
    #[serde(flatten)]
    pub shared: Shared,
    pub default: bool,
}

#[derive(Deserialize, Debug)]
pub struct SelectConfig {
    #[serde(flatten)]
    pub shared: Shared,
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
pub struct MidiConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    pub channel: u8,
    pub cc: u8,
    pub range: [f32; 2],
    pub default: f32,
}

impl Default for MidiConfig {
    fn default() -> Self {
        Self {
            shared: Shared::default(),
            channel: 0,
            cc: 0,
            range: [0.0, 1.0],
            default: 0.0,
        }
    }
}

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
    Automate(AutomateConfig),
    Ramp(RampConfig),
    Random(RandomConfig),
    RandomSlewed(RandomSlewedConfig),
    RoundRobin(RoundRobinConfig),
    Triangle(TriangleConfig),
}

#[derive(Clone, Debug)]
pub enum KeyframeSequence {
    Breakpoints(Vec<Breakpoint>),
    None,
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

#[derive(Clone, Deserialize, Debug)]
pub struct BreakpointConfig {
    pub position: ParamValue,
    pub value: ParamValue,
    #[serde(flatten)]
    pub kind: KindConfig,
}

#[derive(Clone, Deserialize, Debug)]
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
        #[serde(default = "default_param_value_0_25")]
        amplitude: ParamValue,
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

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct RampConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    pub beats: ParamValue,
    pub range: [f32; 2],
    pub phase: ParamValue,
}

impl Default for RampConfig {
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
pub struct RandomConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    pub beats: ParamValue,
    pub range: [f32; 2],
    pub delay: ParamValue,
    pub bias: ParamValue,
    /// See [`RandomConfig::stem` documentation](Self#stem-resolution).
    ///
    /// # Stem Resolution
    ///
    /// When `None` (omitted from YAML), a deterministic stem is generated by
    /// hashing the mapping's YAML key name during
    /// [`ControlHub::populate_controls`]. This ensures every mapping gets a
    /// unique, stable stem without manual bookkeeping.
    ///
    /// When explicitly provided, the value is used as-is. Note that sequential
    /// stems (e.g. 300, 301) can produce correlated output because the internal
    /// seed formula only shifts by 1 per loop cycle — prefer omitting `stem` or
    /// spacing explicit values well apart.
    pub stem: Option<u64>,
}

impl Default for RandomConfig {
    fn default() -> Self {
        Self {
            shared: Shared::default(),
            beats: ParamValue::Cold(1.0),
            range: [0.0, 1.0],
            delay: ParamValue::Cold(0.0),
            bias: ParamValue::Cold(0.0),
            stem: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct RandomSlewedConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    pub beats: ParamValue,
    pub range: [f32; 2],
    pub slew: ParamValue,
    pub delay: ParamValue,
    pub bias: ParamValue,
    /// See [`RandomConfig`] for stem resolution docs.
    pub stem: Option<u64>,
}

impl Default for RandomSlewedConfig {
    fn default() -> Self {
        Self {
            shared: Shared::default(),
            beats: ParamValue::Cold(1.0),
            range: [0.0, 1.0],
            slew: ParamValue::Cold(0.65),
            delay: ParamValue::Cold(0.0),
            bias: ParamValue::Cold(0.0),
            stem: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct RoundRobinConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    pub values: Vec<f32>,
    pub beats: ParamValue,
    pub slew: ParamValue,
    /// See [`RandomConfig`] for stem resolution docs.
    pub stem: Option<u64>,
}

impl Default for RoundRobinConfig {
    fn default() -> Self {
        Self {
            shared: Shared::default(),
            values: vec![],
            beats: ParamValue::Cold(1.0),
            slew: ParamValue::Cold(0.0),
            stem: None,
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

#[derive(Debug, Clone, Default)]
pub struct SnapshotSequenceConfig {
    pub disabled: Option<DisabledConfig>,
    pub stages: Vec<SnapshotSequenceStageConfig>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct SnapshotSequenceConfigRaw {
    #[serde(default, deserialize_with = "to_disabled_fn")]
    disabled: Option<DisabledConfig>,
    stages: Option<Vec<SnapshotSequenceStageConfig>>,
    beats: Option<f32>,
    snapshots: Option<Vec<SnapshotId>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SnapshotId {
    String(String),
    Int(i64),
    Uint(u64),
    Float(f64),
}

impl SnapshotId {
    fn into_string(self) -> Result<String, String> {
        match self {
            SnapshotId::String(value) => Ok(value),
            SnapshotId::Int(value) => Ok(value.to_string()),
            SnapshotId::Uint(value) => Ok(value.to_string()),
            SnapshotId::Float(value) => {
                if !value.is_finite() {
                    return Err("snapshot must be finite".to_string());
                }

                if value.fract() == 0.0 {
                    Ok(format!("{value:.0}"))
                } else {
                    Ok(value.to_string())
                }
            }
        }
    }
}

impl<'de> Deserialize<'de> for SnapshotSequenceConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = SnapshotSequenceConfigRaw::deserialize(deserializer)?;

        let has_stages = raw.stages.is_some();
        let has_beats = raw.beats.is_some();
        let has_snapshots = raw.snapshots.is_some();

        if has_stages && (has_beats || has_snapshots) {
            return Err(serde::de::Error::custom(
                "snapshot_sequence cannot define both `stages` and \
                 `beats`/`snapshots` shorthand",
            ));
        }

        if has_beats ^ has_snapshots {
            return Err(serde::de::Error::custom(
                "snapshot_sequence shorthand requires both `beats` and \
                 `snapshots`",
            ));
        }

        if let Some(stages) = raw.stages {
            return Ok(Self {
                disabled: raw.disabled,
                stages,
            });
        }

        if let (Some(beats), Some(snapshots)) = (raw.beats, raw.snapshots) {
            if !beats.is_finite() || beats <= 0.0 {
                return Err(serde::de::Error::custom(
                    "snapshot_sequence `beats` must be finite and > 0.0",
                ));
            }

            if snapshots.is_empty() {
                return Err(serde::de::Error::custom(
                    "snapshot_sequence shorthand `snapshots` cannot be empty",
                ));
            }

            let mut stages = Vec::with_capacity(snapshots.len() + 1);
            for (index, snapshot) in snapshots.into_iter().enumerate() {
                let snapshot =
                    snapshot.into_string().map_err(serde::de::Error::custom)?;
                stages.push(SnapshotSequenceStageConfig::Stage {
                    snapshot,
                    position: index as f32 * beats,
                });
            }

            stages.push(SnapshotSequenceStageConfig::End {
                position: stages.len() as f32 * beats,
            });

            return Ok(Self {
                disabled: raw.disabled,
                stages,
            });
        }

        Ok(Self {
            disabled: raw.disabled,
            stages: vec![],
        })
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum SnapshotSequenceStageConfig {
    Stage {
        #[serde(deserialize_with = "deserialize_stage_id")]
        snapshot: String,
        position: f32,
    },
    End {
        position: f32,
    },
}

impl SnapshotSequenceStageConfig {
    pub fn position(&self) -> f32 {
        match self {
            SnapshotSequenceStageConfig::Stage { position, .. } => *position,
            SnapshotSequenceStageConfig::End { position } => *position,
        }
    }

    pub fn snapshot(&self) -> Option<&str> {
        match self {
            SnapshotSequenceStageConfig::Stage { snapshot, .. } => {
                Some(snapshot.as_str())
            }
            SnapshotSequenceStageConfig::End { .. } => None,
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

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EffectKind {
    Constrain {
        #[serde(default = "default_clamp_string")]
        mode: String,
        #[serde(default = "default_normalized_range")]
        range: (f32, f32),
    },

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

    Map {
        domain: (f32, f32),
        range: (f32, f32),
    },

    Math {
        operator: String,
        operand: ParamValue,
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
// Disabled Impl
//------------------------------------------------------------------------------

#[derive(Default, Deserialize)]
pub struct DisabledConfig {
    #[serde(skip)]
    pub disabled_fn: DisabledFn,
}

impl fmt::Debug for DisabledConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DisabledConfig")
            .field(
                "disabled_fn",
                &self.disabled_fn.as_ref().map(|_| "<function>"),
            )
            .finish()
    }
}

impl Clone for DisabledConfig {
    fn clone(&self) -> Self {
        DisabledConfig { disabled_fn: None }
    }
}

fn to_disabled_fn<'de, D>(
    deserializer: D,
) -> Result<Option<DisabledConfig>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DisabledInput {
        String(String),
        Bool(bool),
    }

    match DisabledInput::deserialize(deserializer) {
        Ok(DisabledInput::Bool(value)) => {
            let disabled_fn =
                Some(Box::new(move |_controls: &UiControls| value)
                    as Box<dyn Fn(&UiControls) -> bool + 'static>);
            Ok(Some(DisabledConfig { disabled_fn }))
        }
        Ok(DisabledInput::String(expression)) => {
            if expression.trim().is_empty() {
                return Ok(None);
            }

            match parse_disabled_expression(&expression) {
                Ok(disabled_fn) => Ok(Some(DisabledConfig { disabled_fn })),
                Err(e) => Err(serde::de::Error::custom(e)),
            }
        }
        Err(_) => Ok(None),
    }
}

fn parse_disabled_expression(expr: &str) -> Result<DisabledFn, Box<dyn Error>> {
    if expr.trim().is_empty() {
        return Ok(None);
    }

    let or_conditions: Vec<&str> =
        expr.split(" or ").map(|s| s.trim()).collect();
    let mut condition_closures = Vec::new();

    for or_condition in or_conditions {
        let and_conditions: Vec<&str> =
            or_condition.split(" and ").map(|s| s.trim()).collect();

        if and_conditions.len() == 1 {
            let closure = parse_condition(and_conditions[0])?;
            if let Some(f) = closure {
                condition_closures.push(f);
            }
        } else {
            let mut and_closures = Vec::new();
            for and_condition in and_conditions {
                let closure = parse_condition(and_condition)?;
                if let Some(f) = closure {
                    and_closures.push(f);
                }
            }

            if !and_closures.is_empty() {
                let combined_and = Box::new(move |controls: &UiControls| {
                    and_closures.iter().all(|closure| closure(controls))
                });
                condition_closures.push(combined_and);
            }
        }
    }

    if condition_closures.is_empty() {
        return Ok(None);
    }

    let combined_fn = Box::new(move |controls: &UiControls| {
        condition_closures.iter().any(|closure| closure(controls))
    });

    Ok(Some(combined_fn))
}

type ParseResult =
    Result<Option<Box<dyn Fn(&UiControls) -> bool + 'static>>, Box<dyn Error>>;

fn parse_condition(condition: &str) -> ParseResult {
    let condition = condition.trim();

    if condition.eq_ignore_ascii_case("true") {
        let closure = Box::new(|_controls: &UiControls| true);
        return Ok(Some(closure));
    }

    if condition.eq_ignore_ascii_case("false") {
        let closure = Box::new(|_controls: &UiControls| false);
        return Ok(Some(closure));
    }

    if let Some(inner_condition) = condition.strip_prefix("not ") {
        let inner_closure = parse_condition(inner_condition)?;

        if let Some(f) = inner_closure {
            let negated = Box::new(move |controls: &UiControls| !f(controls));
            return Ok(Some(negated));
        }

        return Ok(None);
    }

    if condition.contains(" is not ") {
        let parts: Vec<&str> = condition.split(" is not ").collect();
        if parts.len() != 2 {
            return Err(
                format!("Invalid condition format: {}", condition).into()
            );
        }

        let field_name = parts[0].trim().to_string();
        let value = parts[1].trim().to_string();

        let closure = Box::new(move |controls: &UiControls| {
            controls.string(&field_name) != value
        });

        return Ok(Some(closure));
    }

    if condition.contains(" is ") {
        let parts: Vec<&str> = condition.split(" is ").collect();
        if parts.len() != 2 {
            return Err(
                format!("Invalid condition format: {}", condition).into()
            );
        }

        let field_name = parts[0].trim().to_string();
        let value = parts[1].trim().to_string();

        let closure = Box::new(move |controls: &UiControls| {
            controls.string(&field_name) == value
        });

        return Ok(Some(closure));
    }

    let field_name = condition.to_string();
    let closure =
        Box::new(move |controls: &UiControls| controls.bool(&field_name));

    Ok(Some(closure))
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

fn deserialize_stage_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StageId {
        String(String),
        Int(i64),
        Uint(u64),
        Float(f64),
    }

    match StageId::deserialize(deserializer)? {
        StageId::String(value) => Ok(value),
        StageId::Int(value) => Ok(value.to_string()),
        StageId::Uint(value) => Ok(value.to_string()),
        StageId::Float(value) => {
            if !value.is_finite() {
                return Err(serde::de::Error::custom("stage must be finite"));
            }

            if value.fract() == 0.0 {
                Ok(format!("{value:.0}"))
            } else {
                Ok(value.to_string())
            }
        }
    }
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
fn default_clamp_string() -> String {
    "clamp".to_string()
}
fn default_false() -> bool {
    false
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_sequence_stage_deserializes_number_to_string() {
        let yaml = r#"
type: snapshot_sequence
stages:
  - kind: stage
    snapshot: 1
    position: 0.0
"#;

        let config: SnapshotSequenceConfig =
            serde_yml::from_str(yaml).expect("Expected valid config");

        assert_eq!(config.stages[0].snapshot(), Some("1"));
    }

    #[test]
    fn test_snapshot_sequence_stage_deserializes_string() {
        let yaml = r#"
type: snapshot_sequence
stages:
  - kind: stage
    snapshot: "1"
    position: 0.0
"#;

        let config: SnapshotSequenceConfig =
            serde_yml::from_str(yaml).expect("Expected valid config");

        assert_eq!(config.stages[0].snapshot(), Some("1"));
    }

    #[test]
    fn test_snapshot_sequence_stage_rejects_missing_stage() {
        let yaml = r#"
type: snapshot_sequence
stages:
  - kind: stage
    position: 0.0
"#;

        let result = serde_yml::from_str::<SnapshotSequenceConfig>(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_snapshot_sequence_disabled_bool_true() {
        let yaml = r#"
type: snapshot_sequence
disabled: true
stages:
  - kind: stage
    snapshot: 1
    position: 0.0
  - kind: end
    position: 1.0
"#;

        let mut config: SnapshotSequenceConfig =
            serde_yml::from_str(yaml).expect("Expected valid config");
        let disabled = config.disabled.take().and_then(|d| d.disabled_fn);
        let controls = UiControls::default();

        assert!(disabled.as_ref().is_some_and(|f| f(&controls)));
    }

    #[test]
    fn test_snapshot_sequence_disabled_string_true() {
        let yaml = r#"
type: snapshot_sequence
disabled: "true"
stages:
  - kind: stage
    snapshot: 1
    position: 0.0
  - kind: end
    position: 1.0
"#;

        let mut config: SnapshotSequenceConfig =
            serde_yml::from_str(yaml).expect("Expected valid config");
        let disabled = config.disabled.take().and_then(|d| d.disabled_fn);
        let controls = UiControls::default();

        assert!(disabled.as_ref().is_some_and(|f| f(&controls)));
    }

    #[test]
    fn test_snapshot_sequence_shorthand_normalizes_to_stages() {
        let yaml = r#"
type: snapshot_sequence
beats: 2
snapshots: [1, 2, "3"]
"#;

        let config: SnapshotSequenceConfig =
            serde_yml::from_str(yaml).expect("Expected valid config");

        assert_eq!(config.stages.len(), 4);
        assert_eq!(config.stages[0].snapshot(), Some("1"));
        assert_eq!(config.stages[0].position(), 0.0);
        assert_eq!(config.stages[1].snapshot(), Some("2"));
        assert_eq!(config.stages[1].position(), 2.0);
        assert_eq!(config.stages[2].snapshot(), Some("3"));
        assert_eq!(config.stages[2].position(), 4.0);
        assert!(matches!(
            config.stages[3],
            SnapshotSequenceStageConfig::End { position: 6.0 }
        ));
    }

    #[test]
    fn test_snapshot_sequence_shorthand_rejects_mixed_forms() {
        let yaml = r#"
type: snapshot_sequence
beats: 2
snapshots: [1, 2]
stages:
  - kind: stage
    snapshot: 1
    position: 0.0
  - kind: end
    position: 4.0
"#;

        let result = serde_yml::from_str::<SnapshotSequenceConfig>(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_snapshot_sequence_shorthand_rejects_partial_form() {
        let yaml = r#"
type: snapshot_sequence
beats: 2
"#;

        let result = serde_yml::from_str::<SnapshotSequenceConfig>(yaml);
        assert!(result.is_err());
    }
}
