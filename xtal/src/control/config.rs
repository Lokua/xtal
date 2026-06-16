//! YAML control script schema and deserialization helpers.
//!
//! These types mirror the control script format before it is expanded into
//! runtime control collections by `ControlHub`. Public fields correspond to
//! YAML keys or normalized schema values used during population.

use std::error::Error;
use std::fmt;

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize};

use super::param_mod::ParamValue;
use super::video_transport::VideoDirection;
use crate::core::prelude::*;

//------------------------------------------------------------------------------
// Top-level Types
//------------------------------------------------------------------------------

/// Parsed control script keyed by YAML control name.
///
/// Uses [`IndexMap`] so UI controls keep declaration order.
pub type ConfigFile = IndexMap<String, MaybeControlConfig>;

/// YAML entry that may or may not be a recognized Xtal control.
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum MaybeControlConfig {
    /// Recognized control entry with a `type` field.
    Control(ScriptedControlConfig),
    /// Unknown or non-control YAML retained only for tolerant parsing.
    #[allow(dead_code)]
    Other(serde_yml::Value),
}

/// Parsed control entry with its type and raw remaining fields.
#[derive(Deserialize, Debug)]
pub struct ScriptedControlConfig {
    /// Control type selected by the YAML `type` field.
    #[serde(rename = "type")]
    pub control_type: ControlType,
    /// Raw config payload deserialized later by concrete config type.
    #[serde(flatten)]
    pub config: serde_yml::Value,
}

/// Supported YAML control `type` values.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ControlType {
    /// UI slider control.
    #[serde(rename = "slider")]
    Slider,
    /// UI checkbox control.
    #[serde(rename = "checkbox")]
    Checkbox,
    /// UI select control.
    #[serde(rename = "select")]
    Select,
    /// UI section separator.
    #[serde(rename = "separator")]
    Separator,

    /// Direct MIDI CC control.
    #[serde(rename = "midi")]
    Midi,
    /// OSC address control.
    #[serde(rename = "osc")]
    Osc,
    /// Audio input control.
    #[serde(rename = "audio")]
    Audio,

    /// Breakpoint automation control.
    #[serde(rename = "automate")]
    Automate,
    /// Repeating ramp animation.
    #[serde(rename = "ramp")]
    Ramp,
    /// Deterministic stepped random animation.
    #[serde(rename = "random")]
    Random,
    /// Deterministic random animation with slew smoothing.
    #[serde(rename = "random_slewed")]
    RandomSlewed,
    /// Beat-stepped sequence animation.
    #[serde(rename = "round_robin")]
    RoundRobin,
    /// Triangle-wave animation.
    #[serde(rename = "triangle")]
    Triangle,
    /// Beat-scheduled snapshot recall sequence.
    #[serde(rename = "snapshot_sequence")]
    SnapshotSequence,
    /// Video transport control.
    #[serde(rename = "video")]
    Video,

    /// Modulation chain control.
    #[serde(rename = "mod")]
    Modulation,
    /// Effect control.
    #[serde(rename = "effect")]
    Effects,
}

/// Fields shared by most YAML control types.
#[allow(dead_code)]
#[derive(Clone, Deserialize, Debug, Default)]
pub struct Shared {
    /// Optional constant value that bypasses the live control.
    #[serde(default, deserialize_with = "deserialize_number_or_none")]
    pub bypass: Option<f32>,
    /// Optional short alias exposed to shader uniform banks.
    #[serde(default)]
    pub var: Option<String>,
    // TODO: this really shouldn't be on shared because only UI controls use it
    #[serde(default, deserialize_with = "to_disabled_fn")]
    pub disabled: Option<DisabledConfig>,
}

//------------------------------------------------------------------------------
// UI
//------------------------------------------------------------------------------

/// YAML config for a UI slider.
#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct SliderConfig {
    /// Shared control fields.
    #[serde(flatten)]
    pub shared: Shared,
    /// Slider range as `[min, max]`.
    pub range: [f32; 2],
    /// Initial slider value.
    pub default: f32,
    /// Slider step size.
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

/// YAML config for a UI checkbox.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct CheckboxConfig {
    /// Shared control fields.
    #[serde(flatten)]
    pub shared: Shared,
    /// Initial checkbox value.
    pub default: bool,
}

/// YAML config for a UI select.
#[derive(Deserialize, Debug)]
pub struct SelectConfig {
    /// Shared control fields.
    #[serde(flatten)]
    pub shared: Shared,
    /// Available option labels.
    pub options: Vec<String>,
    /// Initial selected option.
    pub default: String,
}

//------------------------------------------------------------------------------
// Video
//------------------------------------------------------------------------------

/// YAML config for a video transport control.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct VideoConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    /// Video source identifier or path.
    pub source: String,
    /// Clip/frame index or hot parameter reference.
    pub index: ParamValue,
    /// Start beat or hot parameter reference.
    pub start: ParamValue,
    /// Duration in beats or hot parameter reference.
    pub beats: ParamValue,
    /// Playback speed or hot parameter reference.
    pub speed: ParamValue,
    /// Playback direction.
    pub direction: VideoDirectionConfig,
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            shared: Shared::default(),
            source: String::new(),
            index: ParamValue::Cold(0.0),
            start: ParamValue::Cold(0.0),
            beats: ParamValue::Cold(4.0),
            speed: ParamValue::Cold(1.0),
            direction: VideoDirectionConfig::Forward,
        }
    }
}

/// YAML playback direction for video controls.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoDirectionConfig {
    /// Advance video forward.
    #[default]
    Forward,
    /// Advance video backward.
    Backward,
    /// Alternate direction each cycle.
    PingPong,
}

impl From<VideoDirectionConfig> for VideoDirection {
    fn from(value: VideoDirectionConfig) -> Self {
        match value {
            VideoDirectionConfig::Forward => VideoDirection::Forward,
            VideoDirectionConfig::Backward => VideoDirection::Backward,
            VideoDirectionConfig::PingPong => VideoDirection::PingPong,
        }
    }
}

//------------------------------------------------------------------------------
// External
//------------------------------------------------------------------------------

/// YAML config for a direct MIDI CC control.
#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct MidiConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    /// Zero-based MIDI channel.
    pub channel: u8,
    /// Control Change number.
    pub cc: u8,
    /// Mapped output range as `[min, max]`.
    pub range: [f32; 2],
    /// Initial mapped value.
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

/// YAML config for an OSC control.
#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct OscConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    /// Mapped output range as `[min, max]`.
    pub range: [f32; 2],
    /// Initial mapped value.
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

/// YAML config for an audio input control.
#[derive(Clone, Deserialize, Debug)]
#[serde(default)]
pub struct AudioConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    /// Zero-based audio input channel.
    pub channel: usize,
    /// Slew rise and fall values.
    pub slew: [f32; 2],
    /// Pre-emphasis amount.
    pub pre: f32,
    /// Detection coefficient.
    pub detect: f32,
    /// Mapped output range as `[min, max]`.
    pub range: [f32; 2],
    /// Optional bypass value specific to audio controls.
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

/// Parsed animation config before runtime animation evaluation.
#[derive(Debug)]
pub enum AnimationConfig {
    /// Breakpoint automation.
    Automate(AutomateConfig),
    /// Ramp animation.
    Ramp(RampConfig),
    /// Random step animation.
    Random(RandomConfig),
    /// Slewed random animation.
    RandomSlewed(RandomSlewedConfig),
    /// Round-robin animation.
    RoundRobin(RoundRobinConfig),
    /// Triangle animation.
    Triangle(TriangleConfig),
}

/// Runtime keyframe payload extracted from animation config.
#[derive(Clone, Debug)]
pub enum KeyframeSequence {
    /// Breakpoint list for automate controls.
    Breakpoints(Vec<Breakpoint>),
    /// No keyframe sequence.
    None,
}

/// YAML config for a breakpoint automation control.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct AutomateConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    /// Breakpoints in beat order.
    pub breakpoints: Vec<BreakpointConfig>,
    /// Playback mode, usually `loop` or `once`.
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

/// YAML config for one automation breakpoint.
#[derive(Clone, Deserialize, Debug)]
pub struct BreakpointConfig {
    /// Beat position or hot parameter reference.
    pub position: ParamValue,
    /// Base value or hot parameter reference.
    pub value: ParamValue,
    /// Segment kind and kind-specific fields.
    #[serde(flatten)]
    pub kind: KindConfig,
}

/// YAML segment kind for one automation breakpoint.
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum KindConfig {
    /// Hold the current value until the next breakpoint.
    Step,
    /// Ramp to the next breakpoint value.
    Ramp {
        /// Easing function name.
        #[serde(default = "default_easing")]
        easing: String,
    },
    /// Ramp with periodic modulation.
    Wave {
        /// Wave shape name.
        #[serde(default = "default_shape")]
        shape: String,
        /// Modulation cycle length in beats or hot parameter reference.
        #[serde(default = "default_param_value_0_25")]
        frequency: ParamValue,
        /// Modulation depth or hot parameter reference.
        #[serde(default = "default_param_value_0_25")]
        amplitude: ParamValue,
        /// Wave duty cycle or skew.
        #[serde(default = "default_param_value_0_5")]
        width: ParamValue,
        /// Easing function name for the underlying ramp.
        #[serde(default = "default_easing")]
        easing: String,
        /// Constraint mode applied after modulation.
        #[serde(default = "default_none_string")]
        constrain: String,
    },
    /// Deterministic random value around the breakpoint value.
    Random {
        /// Maximum random deviation or hot parameter reference.
        #[serde(default = "default_param_value_0_25")]
        amplitude: ParamValue,
    },
    /// Smooth noise around the ramped breakpoint value.
    RandomSmooth {
        /// Noise cycle length in beats or hot parameter reference.
        #[serde(default = "default_param_value_0_25")]
        frequency: ParamValue,
        /// Maximum noise deviation or hot parameter reference.
        #[serde(default = "default_param_value_0_25")]
        amplitude: ParamValue,
        /// Easing function name for the underlying ramp.
        #[serde(default = "default_easing")]
        easing: String,
        /// Constraint mode applied after modulation.
        #[serde(default = "default_none_string")]
        constrain: String,
    },
    /// Final endpoint marker.
    End,
}

/// YAML config for a ramp animation.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct RampConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    /// Cycle length in beats or hot parameter reference.
    pub beats: ParamValue,
    /// Mapped output range.
    pub range: [f32; 2],
    /// Phase offset or hot parameter reference.
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

/// YAML config for a deterministic random animation.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct RandomConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    /// Cycle length in beats or hot parameter reference.
    pub beats: ParamValue,
    /// Mapped output range.
    pub range: [f32; 2],
    /// Beat delay before random steps begin.
    pub delay: ParamValue,
    /// Reserved bias parameter used by older control scripts.
    pub bias: ParamValue,
    /// See [`RandomConfig::stem` documentation](Self#stem-resolution).
    ///
    /// # Stem Resolution
    ///
    /// When `None` (omitted from YAML), a deterministic stem is generated by
    /// hashing the mapping's YAML key name during control population. This
    /// ensures every mapping gets a unique, stable stem without manual
    /// bookkeeping.
    ///
    /// When explicitly provided, the value is used as-is. Note that sequential
    /// stems (e.g. 300, 301) can produce correlated output because the internal
    /// seed formula only shifts by 1 per loop cycle. Prefer omitting `stem` or
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

/// YAML config for a deterministic random animation with slew smoothing.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct RandomSlewedConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    /// Cycle length in beats or hot parameter reference.
    pub beats: ParamValue,
    /// Mapped output range.
    pub range: [f32; 2],
    /// Slew amount or hot parameter reference.
    pub slew: ParamValue,
    /// Beat delay before random steps begin.
    pub delay: ParamValue,
    /// Reserved bias parameter used by older control scripts.
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

/// YAML config for a round-robin value sequence.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct RoundRobinConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    /// Values to cycle through.
    pub values: Vec<f32>,
    /// Step length in beats or hot parameter reference.
    pub beats: ParamValue,
    /// Slew amount or hot parameter reference.
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

/// YAML config for a triangle animation.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct TriangleConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    /// Cycle length in beats or hot parameter reference.
    pub beats: ParamValue,
    /// Mapped output range.
    pub range: [f32; 2],
    /// Phase offset or hot parameter reference.
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

/// YAML config for beat-scheduled snapshot recall.
#[derive(Debug, Clone, Default)]
pub struct SnapshotSequenceConfig {
    /// Optional disabled predicate.
    pub disabled: Option<DisabledConfig>,
    /// Ordered snapshot stages plus final end marker.
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

/// One normalized snapshot sequence stage.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum SnapshotSequenceStageConfig {
    /// Recalls `snapshot` when playback crosses `position`.
    Stage {
        /// Snapshot id to recall.
        #[serde(deserialize_with = "deserialize_stage_id")]
        snapshot: String,
        /// Stage position in beats.
        position: f32,
    },
    /// Marks the end position for sequence looping.
    End {
        /// End position in beats.
        position: f32,
    },
}

impl SnapshotSequenceStageConfig {
    /// Returns this stage's beat position.
    pub fn position(&self) -> f32 {
        match self {
            SnapshotSequenceStageConfig::Stage { position, .. } => *position,
            SnapshotSequenceStageConfig::End { position } => *position,
        }
    }

    /// Returns the snapshot id for recall stages.
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

/// YAML config for a modulation control.
#[derive(Clone, Deserialize, Debug)]
pub struct ModulationConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    /// Source control value to modulate.
    pub source: String,
    /// Ordered modulator and effect names applied to the source.
    pub modulators: Vec<String>,
}

/// YAML config for an effect control.
#[derive(Clone, Deserialize, Debug)]
pub struct EffectConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    /// Effect kind and kind-specific parameters.
    #[serde(flatten)]
    pub kind: EffectKind,
}

/// Supported YAML effect kinds.
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EffectKind {
    /// Range constraint effect.
    Constrain {
        /// Constraint mode name.
        #[serde(default = "default_clamp_string")]
        mode: String,
        /// Constraint range.
        #[serde(default = "default_normalized_range")]
        range: (f32, f32),
    },

    /// Hysteresis effect.
    Hysteresis {
        /// Lower threshold or hot parameter reference.
        #[serde(default = "default_param_value_0_3")]
        lower_threshold: ParamValue,
        /// Upper threshold or hot parameter reference.
        #[serde(default = "default_param_value_0_7")]
        upper_threshold: ParamValue,
        /// Low-state output value or hot parameter reference.
        #[serde(default = "default_param_value_0")]
        output_low: ParamValue,
        /// High-state output value or hot parameter reference.
        #[serde(default = "default_param_value_1")]
        output_high: ParamValue,
        /// Whether in-between input values pass through unchanged.
        #[serde(default = "default_false")]
        pass_through: bool,
    },

    /// Linear range mapping effect.
    Map {
        /// Input domain.
        domain: (f32, f32),
        /// Output range.
        range: (f32, f32),
    },

    /// Arithmetic or curve effect.
    Math {
        /// Operator name.
        operator: String,
        /// Operand or hot parameter reference.
        operand: ParamValue,
    },

    /// Step quantization effect.
    Quantizer {
        /// Step size or hot parameter reference.
        #[serde(default = "default_param_value_0_25")]
        step: ParamValue,
        /// Output clamp range.
        #[serde(default = "default_normalized_range")]
        range: (f32, f32),
    },

    /// Ring modulation effect.
    RingModulator {
        /// Ring-mod blend or hot parameter reference.
        #[serde(default = "default_param_value_0")]
        mix: ParamValue,
        /// Signal range.
        #[serde(default = "default_normalized_range")]
        range: (f32, f32),
        /// Named modulator control used as the second input.
        modulator: String,
    },

    /// Saturation effect.
    Saturator {
        /// Drive amount or hot parameter reference.
        #[serde(default = "default_param_value_1")]
        drive: ParamValue,
        /// Signal range.
        #[serde(default = "default_normalized_range")]
        range: (f32, f32),
    },

    /// Slew limiter effect.
    SlewLimiter {
        /// Rise smoothing or hot parameter reference.
        #[serde(default = "default_param_value_0")]
        rise: ParamValue,
        /// Fall smoothing or hot parameter reference.
        #[serde(default = "default_param_value_0")]
        fall: ParamValue,
    },

    /// Wave folder effect.
    #[serde()]
    WaveFolder {
        /// Input gain or hot parameter reference.
        #[serde(default = "default_param_value_1")]
        gain: ParamValue,
        /// Number of fold passes.
        #[serde(default = "default_iterations")]
        iterations: usize,
        /// Fold symmetry or hot parameter reference.
        #[serde(default = "default_param_value_1")]
        symmetry: ParamValue,
        /// Fold bias or hot parameter reference.
        #[serde(default = "default_param_value_0")]
        bias: ParamValue,
        /// Fold shape or hot parameter reference.
        #[serde(default = "default_param_value_1")]
        shape: ParamValue,
        // TODO: make Option and consider None to mean "adaptive range"?
        /// Signal range.
        #[serde(default = "default_normalized_range")]
        range: (f32, f32),
    },
}

//------------------------------------------------------------------------------
// Disabled Impl
//------------------------------------------------------------------------------

/// Compiled disabled predicate for UI controls and snapshot sequences.
#[derive(Default, Deserialize)]
pub struct DisabledConfig {
    /// Predicate evaluated against current UI controls.
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
