//! Provides a means of controlling sketch parameters with the various Lattice
//! control systems from an external yaml file that can be hot-reloaded.

use indexmap::IndexMap;
use notify::{Event, RecursiveMode, Watcher};
use serde::{Deserialize, Deserializer};
use std::{
    collections::HashMap,
    error::Error,
    fmt, fs,
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
};
use yaml_merge_keys::merge_keys_serde_yml;

use super::prelude::*;

//------------------------------------------------------------------------------
// Core Types & Implementations
//------------------------------------------------------------------------------

pub struct ControlScript<T: TimingSource> {
    pub controls: Controls,
    pub animation: Animation<T>,
    osc_controls: OscControls,
    audio_controls: AudioControls,
    keyframe_sequences: HashMap<String, (AnimationConfig, KeyframeSequence)>,
    modulations: HashMap<String, Vec<String>>,
    effects: HashMap<String, (EffectConfig, Effect)>,
    update_state: UpdateState,
}

impl<T: TimingSource> ControlScript<T> {
    pub fn new(path: PathBuf, timing: T) -> Self {
        let state = Arc::new(Mutex::new(None));
        let state_clone = state.clone();

        let mut script = Self {
            controls: Controls::with_previous(vec![]),
            osc_controls: OscControls::new(),
            audio_controls: AudioControlBuilder::new().build(),
            animation: Animation::new(timing),
            keyframe_sequences: HashMap::new(),
            modulations: HashMap::new(),
            effects: HashMap::new(),
            update_state: UpdateState {
                state: state.clone(),
                _watcher: Self::setup_watcher(path.clone(), state_clone),
            },
        };

        script
            .import_script(&path)
            .expect("Unable to import script");

        script
    }

    pub fn get(&self, name: &str) -> f32 {
        let value = self.get_raw(name);

        self.modulations.get(name).map_or(value, |modulators| {
            modulators
                .into_iter()
                .fold(value, |modulated_value, modulator| {
                    if self.effects.contains_key(modulator) {
                        let (effect_config, effect) =
                            self.effects.get(modulator).unwrap();

                        if let (
                            EffectConfig {
                                kind:
                                    EffectKind::RingModulator { modulator, .. },
                                ..
                            },
                            Effect::RingModulator(m),
                        ) = (effect_config, effect)
                        {
                            let modulator_signal = self.get_raw(modulator);
                            m.apply(modulated_value, modulator_signal)
                        } else {
                            match effect {
                                Effect::Hysteresis(m) => {
                                    m.apply(modulated_value)
                                }
                                Effect::Quantizer(m) => {
                                    m.apply(modulated_value)
                                }
                                Effect::RingModulator(_) => unreachable!(),
                                Effect::Saturator(m) => {
                                    m.apply(modulated_value)
                                }
                                Effect::SlewLimiter(m) => {
                                    m.apply(modulated_value)
                                }
                                Effect::WaveFolder(m) => {
                                    m.apply(modulated_value)
                                }
                            }
                        }
                    } else {
                        modulated_value * self.get_raw(modulator)
                    }
                })
        })
    }

    fn get_raw(&self, name: &str) -> f32 {
        if self.controls.has(name) {
            return self.controls.float(name);
        }

        let osc_name = format!("/{}", name);
        if self.osc_controls.has(&osc_name) {
            return self.osc_controls.get(&osc_name);
        }

        if self.audio_controls.has(name) {
            return self.audio_controls.get(name);
        }

        if let Some((config, sequence)) = self.keyframe_sequences.get(name) {
            return match (config, sequence) {
                (
                    AnimationConfig::Lerp(conf),
                    KeyframeSequence::Linear(kfs),
                ) => {
                    if let Some(bypass) = conf.bypass {
                        bypass
                    } else {
                        self.animation.lerp(&kfs.clone(), config.delay())
                    }
                }
                (
                    AnimationConfig::RRampRel(conf),
                    KeyframeSequence::Random(kfs),
                ) => {
                    if let Some(bypass) = conf.bypass {
                        bypass
                    } else {
                        self.animation.r_ramp(
                            kfs,
                            conf.delay,
                            conf.ramp_time,
                            Easing::from_str(conf.ramp.as_str()).unwrap(),
                        )
                    }
                }
                (AnimationConfig::Triangle(conf), KeyframeSequence::None) => {
                    if let Some(bypass) = conf.bypass {
                        bypass
                    } else {
                        self.animation.triangle(
                            conf.beats,
                            (conf.range[0], conf.range[1]),
                            conf.phase,
                        )
                    }
                }
                (
                    AnimationConfig::Automate(conf),
                    KeyframeSequence::Breakpoints(breakpoints),
                ) => {
                    if let Some(bypass) = conf.bypass {
                        bypass
                    } else {
                        self.animation.automate(
                            breakpoints,
                            Mode::from_str(&conf.mode).unwrap(),
                        )
                    }
                }
                _ => unimplemented!(),
            };
        }

        warn_once!("No control named {}. Defaulting to 0.0", name);
        0.0
    }

    pub fn bool(&self, name: &str) -> bool {
        return self.controls.bool(name);
    }

    pub fn string(&self, name: &str) -> String {
        return self.controls.string(name);
    }

    pub fn breakpoints(&self, name: &str) -> Vec<Breakpoint> {
        self.keyframe_sequences
            .get(name)
            .and_then(|(_, sequence)| match sequence {
                KeyframeSequence::Breakpoints(breakpoints) => {
                    Some(breakpoints.clone())
                }
                _ => None,
            })
            .unwrap_or_else(|| loud_panic!("No breakpoints for name: {}", name))
    }

    pub fn update(&mut self) {
        let new_config = {
            if let Ok(mut guard) = self.update_state.state.lock() {
                guard.take()
            } else {
                None
            }
        };

        if let Some(config) = new_config {
            if let Err(e) = self.populate_controls(&config) {
                error!("Failed to apply new configuration: {:?}", e);
            }
        }
    }

    pub fn changed(&self) -> bool {
        self.controls.changed()
    }
    pub fn any_changed_in(&self, names: &[&str]) -> bool {
        self.controls.any_changed_in(names)
    }
    pub fn mark_unchanged(&mut self) {
        self.controls.mark_unchanged();
    }

    fn import_script(&mut self, path: &PathBuf) -> Result<(), Box<dyn Error>> {
        let config = Self::parse_config(path)?;
        self.populate_controls(&config)?;
        Ok(())
    }

    fn parse_config(path: &PathBuf) -> Result<ConfigFile, Box<dyn Error>> {
        let file_content = fs::read_to_string(path)?;
        let raw_config = serde_yml::from_str(&file_content)?;
        let merged_config = merge_keys_serde_yml(raw_config)?;
        let config: ConfigFile = serde_yml::from_value(merged_config)?;
        trace!("Parsed config: {:?}", config);
        Ok(config)
    }

    fn populate_controls(
        &mut self,
        control_configs: &ConfigFile,
    ) -> Result<(), Box<dyn Error>> {
        let current_values: ControlValues = self.controls.values().clone();
        let osc_values: HashMap<String, f32> = self.osc_controls.values();

        self.controls = Controls::with_previous(vec![]);
        self.keyframe_sequences.clear();
        self.modulations.clear();

        for (id, maybe_config) in control_configs {
            let config = match maybe_config {
                MaybeControlConfig::Control(config) => config,
                MaybeControlConfig::Other(_) => continue,
            };

            match config.control_type {
                ControlType::Slider => {
                    let conf: SliderConfig =
                        serde_yml::from_value(config.config.clone())?;

                    let value = current_values
                        .get(id)
                        .and_then(ControlValue::as_float)
                        .unwrap_or(conf.default);

                    let slider = Control::slider(
                        id.as_str(),
                        value,
                        (conf.range[0], conf.range[1]),
                        conf.step,
                    );

                    self.controls.add(slider);
                }
                ControlType::Checkbox => {
                    let conf: CheckboxConfig =
                        serde_yml::from_value(config.config.clone())?;

                    let value = current_values
                        .get(id)
                        .and_then(ControlValue::as_bool)
                        .unwrap_or(conf.default);

                    let checkbox = Control::checkbox(id.as_str(), value);
                    self.controls.add(checkbox);
                }
                ControlType::Select => {
                    let conf: SelectConfig =
                        serde_yml::from_value(config.config.clone())?;

                    let value = current_values
                        .get(id)
                        .and_then(ControlValue::as_string)
                        .unwrap_or(conf.default.as_str());

                    let select =
                        Control::select(id.as_str(), value, &conf.options);
                    self.controls.add(select);
                }
                ControlType::Separator => {
                    self.controls.add(Control::dynamic_separator());
                }
                ControlType::Osc => {
                    let conf: OscConfig =
                        serde_yml::from_value(config.config.clone())?;

                    let address = format!("/{}", id);

                    let existing_value = if osc_values.contains_key(&address) {
                        osc_values.get(&address)
                    } else {
                        None
                    };

                    let osc_control = OscControlConfig::new(
                        &address,
                        (conf.range[0], conf.range[1]),
                        conf.default,
                    );

                    self.osc_controls
                        .add(&osc_control.address, osc_control.clone());

                    if let Some(value) = existing_value {
                        self.osc_controls.set(&osc_control.address, *value);
                    }
                }
                ControlType::Audio => {
                    let conf: AudioConfig =
                        serde_yml::from_value(config.config.clone())?;

                    let audio_control = AudioControlConfig::new(
                        conf.channel,
                        SlewConfig::new(conf.slew[0], conf.slew[1]),
                        conf.detect,
                        conf.pre,
                        (conf.range[0], conf.range[1]),
                        0.0,
                    );

                    self.audio_controls.add(id, audio_control);
                }
                ControlType::LerpAbs => {
                    let conf: LerpAbsConfig =
                        serde_yml::from_value(config.config.clone())?;

                    let mut parsed_keyframes = Vec::new();
                    for (time_str, value) in &conf.keyframes {
                        if let Ok(beats) =
                            parse_bar_beat_16th(time_str.as_str())
                        {
                            parsed_keyframes.push(ParsedKeyframe {
                                beats,
                                value: *value,
                            });
                        }
                    }

                    let mut keyframes = Vec::new();
                    for i in 0..parsed_keyframes.len() {
                        let current = &parsed_keyframes[i];
                        let duration = if i < parsed_keyframes.len() - 1 {
                            parsed_keyframes[i + 1].beats - current.beats
                        } else {
                            0.0
                        };

                        keyframes.push(Keyframe::new(current.value, duration));
                    }

                    self.keyframe_sequences.insert(
                        id.to_string(),
                        (
                            AnimationConfig::Lerp(LerpConfig {
                                delay: conf.delay,
                                bypass: conf.bypass,
                            }),
                            KeyframeSequence::Linear(keyframes),
                        ),
                    );
                }
                ControlType::LerpRel => {
                    let conf: LerpRelConfig =
                        serde_yml::from_value(config.config.clone())?;

                    let mut keyframes =
                        Vec::with_capacity(conf.keyframes.len());

                    for (i, &(beats, value)) in
                        conf.keyframes.iter().enumerate()
                    {
                        let duration = if i < conf.keyframes.len() - 1 {
                            beats
                        } else {
                            0.0
                        };

                        keyframes.push(Keyframe::new(value, duration));
                    }

                    self.keyframe_sequences.insert(
                        id.to_string(),
                        (
                            AnimationConfig::Lerp(LerpConfig {
                                delay: conf.delay,
                                bypass: conf.bypass,
                            }),
                            KeyframeSequence::Linear(keyframes),
                        ),
                    );
                }
                ControlType::RRampRel => {
                    let conf: RRampRelConfig =
                        serde_yml::from_value(config.config.clone())?;

                    let keyframes: Vec<_> = conf
                        .keyframes
                        .iter()
                        .map(|&(beats, range)| {
                            KeyframeRandom::new(range, beats)
                        })
                        .collect();

                    self.keyframe_sequences.insert(
                        id.to_string(),
                        (
                            AnimationConfig::RRampRel(conf),
                            KeyframeSequence::Random(keyframes),
                        ),
                    );
                }
                ControlType::Triangle => {
                    let conf: TriangleConfig =
                        serde_yml::from_value(config.config.clone())?;

                    self.keyframe_sequences.insert(
                        id.to_string(),
                        (
                            AnimationConfig::Triangle(conf),
                            KeyframeSequence::None,
                        ),
                    );
                }
                ControlType::Automate => {
                    let conf: AutomateConfig =
                        serde_yml::from_value(config.config.clone())?;

                    let breakpoints = conf
                        .breakpoints
                        .iter()
                        .cloned()
                        .map(Breakpoint::from)
                        .collect();

                    self.keyframe_sequences.insert(
                        id.to_string(),
                        (
                            AnimationConfig::Automate(conf),
                            KeyframeSequence::Breakpoints(breakpoints),
                        ),
                    );
                }
                ControlType::Modulation => {
                    let conf: ModulationConfig =
                        serde_yml::from_value(config.config.clone())?;

                    self.modulations
                        .entry(conf.source)
                        .or_default()
                        .extend(conf.modulators);
                }
                ControlType::Effects => {
                    let conf: EffectConfig =
                        serde_yml::from_value(config.config.clone())?;

                    self.effects.insert(
                        id.to_string(),
                        (conf.clone(), Effect::from(conf)),
                    );
                }
            }
        }

        trace!("Config populated. controls: {:?}, osc_controls: {:?}, keyframe_sequences: {:?}", 
            self.controls, self.osc_controls, self.keyframe_sequences);

        if !self.osc_controls.is_active {
            self.osc_controls
                .start()
                .expect("Unable to start OSC receiver");
        }

        self.controls.mark_changed();

        info!("Controls populated");

        Ok(())
    }

    fn setup_watcher(
        path: PathBuf,
        state: Arc<Mutex<Option<ConfigFile>>>,
    ) -> notify::RecommendedWatcher {
        let path_to_watch = path.clone();

        let mut watcher = notify::recommended_watcher(move |res| {
            let event: Event = match res {
                Ok(event) => event,
                Err(_) => return,
            };

            if event.kind
                != notify::EventKind::Modify(notify::event::ModifyKind::Data(
                    notify::event::DataChange::Content,
                ))
            {
                return;
            }

            info!("{:?} changed. Attempting to reload configuration.", path);

            match Self::parse_config(&path) {
                Ok(new_config) => {
                    if let Ok(mut guard) = state.lock() {
                        info!("Loaded new configuration");
                        *guard = Some(new_config);
                    }
                }
                Err(e) => {
                    error!("Failed to load updated configuration: {:?}", e)
                }
            }
        })
        .expect("Failed to create watcher");

        watcher
            .watch(&path_to_watch, RecursiveMode::NonRecursive)
            .expect("Failed to start watching file");

        watcher
    }
}

impl<T: TimingSource + fmt::Debug> fmt::Debug for ControlScript<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ControlScript")
            .field("controls", &self.controls)
            .field("osc_controls", &self.osc_controls)
            .field("animation", &self.animation)
            .field("keyframe_sequences", &self.keyframe_sequences)
            .field("update_state", &self.update_state)
            .finish()
    }
}

struct UpdateState {
    _watcher: notify::RecommendedWatcher,
    state: Arc<Mutex<Option<ConfigFile>>>,
}

impl fmt::Debug for UpdateState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UpdateState")
            .field("state", &self.state)
            .finish()
    }
}

//------------------------------------------------------------------------------
// Configuration Types
//------------------------------------------------------------------------------

type ConfigFile = IndexMap<String, MaybeControlConfig>;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum MaybeControlConfig {
    Control(ControlConfig),
    #[allow(dead_code)]
    Other(serde_yml::Value),
}

#[derive(Deserialize, Debug)]
struct ControlConfig {
    #[serde(rename = "type")]
    control_type: ControlType,
    #[serde(flatten)]
    config: serde_yml::Value,
}

#[derive(Deserialize, Debug)]
enum ControlType {
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

//------------------------------------------------------------------------------
// Animation Types
//------------------------------------------------------------------------------

enum AnimationConfig {
    Lerp(LerpConfig),
    RRampRel(RRampRelConfig),
    Triangle(TriangleConfig),
    Automate(AutomateConfig),
}

impl AnimationConfig {
    pub fn delay(&self) -> f32 {
        match self {
            AnimationConfig::Lerp(x) => x.delay,
            AnimationConfig::RRampRel(x) => x.delay,
            _ => 0.0,
        }
    }
}

impl fmt::Debug for AnimationConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnimationConfig::Lerp(x) => {
                f.debug_tuple("AnimationConfig::LerpAbs").field(x).finish()
            }
            AnimationConfig::RRampRel(x) => {
                f.debug_tuple("AnimationConfig::RRampRel").field(x).finish()
            }
            AnimationConfig::Triangle(x) => {
                f.debug_tuple("AnimationConfig::Triangle").field(x).finish()
            }
            AnimationConfig::Automate(x) => {
                f.debug_tuple("AnimationConfig::Automate").field(x).finish()
            }
        }
    }
}

#[derive(Clone, Debug)]
enum KeyframeSequence {
    Linear(Vec<Keyframe>),
    Random(Vec<KeyframeRandom>),
    Breakpoints(Vec<Breakpoint>),
    None,
}

//------------------------------------------------------------------------------
// Configuration Types
//------------------------------------------------------------------------------

// --- UI Controls

#[derive(Deserialize, Debug)]
#[serde(default)]
struct SliderConfig {
    range: [f32; 2],
    default: f32,
    step: f32,
}

impl Default for SliderConfig {
    fn default() -> Self {
        Self {
            range: [0.0, 1.0],
            default: 0.0,
            step: 0.000_1,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(default)]
struct CheckboxConfig {
    default: bool,
}

impl Default for CheckboxConfig {
    fn default() -> Self {
        Self { default: false }
    }
}

#[derive(Deserialize, Debug)]
struct SelectConfig {
    options: Vec<String>,
    default: String,
}

#[derive(Deserialize, Debug)]
struct Separator {}

// --- External Controls

#[derive(Deserialize, Debug)]
#[serde(default)]
struct OscConfig {
    range: [f32; 2],
    default: f32,
}

impl Default for OscConfig {
    fn default() -> Self {
        Self {
            range: [0.0, 1.0],
            default: 0.0,
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
#[serde(default)]
struct AudioConfig {
    channel: usize,
    slew: [f32; 2],
    pre: f32,
    detect: f32,
    range: [f32; 2],
    bypass: Option<f32>,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            channel: 0,
            slew: [0.0, 0.0],
            pre: 0.0,
            detect: 0.0,
            range: [0.0, 1.0],
            bypass: None,
        }
    }
}

// --- Animation Controls

#[derive(Clone, Debug)]
struct LerpConfig {
    delay: f32,
    bypass: Option<f32>,
}

#[derive(Clone, Deserialize, Debug)]
struct LerpAbsConfig {
    #[serde(default)]
    delay: f32,
    keyframes: Vec<(String, f32)>,
    #[serde(default, deserialize_with = "deserialize_number_or_none")]
    bypass: Option<f32>,
}

impl Default for LerpAbsConfig {
    fn default() -> Self {
        Self {
            delay: 0.0,
            keyframes: Vec::new(),
            bypass: None,
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
struct LerpRelConfig {
    #[serde(default)]
    delay: f32,
    keyframes: Vec<(f32, f32)>,
    #[serde(default, deserialize_with = "deserialize_number_or_none")]
    bypass: Option<f32>,
}

impl Default for LerpRelConfig {
    fn default() -> Self {
        Self {
            delay: 0.0,
            keyframes: Vec::new(),
            bypass: None,
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
struct RRampRelConfig {
    #[serde(default)]
    delay: f32,
    #[serde(default)]
    ramp_time: f32,
    #[serde(default = "default_ramp")]
    ramp: String,
    keyframes: Vec<(f32, (f32, f32))>,
    #[serde(default, deserialize_with = "deserialize_number_or_none")]
    bypass: Option<f32>,
}

fn default_ramp() -> String {
    "linear".to_string()
}

impl Default for RRampRelConfig {
    fn default() -> Self {
        Self {
            delay: 0.0,
            ramp: "linear".to_string(),
            ramp_time: 0.25,
            keyframes: Vec::new(),
            bypass: None,
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
#[serde(default)]
struct TriangleConfig {
    beats: f32,
    range: [f32; 2],
    phase: f32,
    #[serde(deserialize_with = "deserialize_number_or_none")]
    bypass: Option<f32>,
}

impl Default for TriangleConfig {
    fn default() -> Self {
        Self {
            beats: 1.0,
            range: [0.0, 1.0],
            phase: 0.0,
            bypass: None,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(default)]
struct AutomateConfig {
    breakpoints: Vec<BreakpointConfig>,
    #[serde(default = "default_mode")]
    mode: String,
    #[serde(deserialize_with = "deserialize_number_or_none")]
    bypass: Option<f32>,
}

impl Default for AutomateConfig {
    fn default() -> Self {
        Self {
            breakpoints: Vec::new(),
            mode: "loop".to_string(),
            bypass: None,
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
struct BreakpointConfig {
    #[serde(alias = "pos", alias = "x")]
    position: f32,
    #[serde(alias = "val", alias = "y")]
    value: f32,
    #[serde(flatten)]
    kind: KindConfig,
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
enum KindConfig {
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

// --- Modulation & Effects

#[derive(Clone, Deserialize, Debug)]
struct ModulationConfig {
    source: String,
    modulators: Vec<String>,
}

#[derive(Clone, Deserialize, Debug)]
struct EffectConfig {
    #[serde(flatten)]
    kind: EffectKind,
}

impl From<EffectConfig> for Effect {
    fn from(config: EffectConfig) -> Self {
        match config.kind {
            EffectKind::WaveFolder {
                gain,
                iterations,
                symmetry,
                bias,
                shape,
                range,
            } => Effect::WaveFolder(WaveFolder::new(
                gain, iterations, symmetry, bias, shape, range,
            )),
            EffectKind::SlewLimiter { rise, fall } => {
                Effect::SlewLimiter(SlewLimiter::new(rise, fall))
            }
            EffectKind::Saturator { drive, range } => {
                Effect::Saturator(Saturator::new(drive, range))
            }
            EffectKind::Hysteresis {
                lower_threshold,
                upper_threshold,
                output_low,
                output_high,
                pass_through,
            } => Effect::Hysteresis(Hysteresis::new(
                lower_threshold,
                upper_threshold,
                output_low,
                output_high,
                pass_through,
            )),
            EffectKind::Quantizer { step, range } => {
                Effect::Quantizer(Quantizer::new(step, range))
            }
            EffectKind::RingModulator { mix, range, .. } => {
                Effect::RingModulator(RingModulator::new(mix, range))
            }
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "kind")]
enum EffectKind {
    #[serde(alias = "hyst", alias = "hys")]
    Hysteresis {
        #[serde(default = "default_f32_0_3")]
        lower_threshold: f32,
        #[serde(default = "default_f32_0_7")]
        upper_threshold: f32,
        #[serde(default = "default_f32_0")]
        output_low: f32,
        #[serde(default = "default_f32_1")]
        output_high: f32,
        #[serde(default = "default_false")]
        pass_through: bool,
    },

    #[serde(alias = "quant")]
    Quantizer {
        #[serde(default = "default_f32_0_25")]
        step: f32,
        #[serde(default = "default_normalized_range")]
        range: (f32, f32),
    },

    #[serde(alias = "rm", alias = "ring")]
    RingModulator {
        #[serde(default = "default_f32_0")]
        mix: f32,
        #[serde(default = "default_normalized_range")]
        range: (f32, f32),
        modulator: String,
    },

    #[serde(alias = "saturate", alias = "sat")]
    Saturator {
        #[serde(default = "default_f32_1")]
        drive: f32,
        #[serde(default = "default_normalized_range")]
        range: (f32, f32),
    },

    #[serde(alias = "slew")]
    SlewLimiter {
        #[serde(default = "default_f32_0")]
        rise: f32,
        #[serde(default = "default_f32_0")]
        fall: f32,
    },

    #[serde(alias = "fold")]
    WaveFolder {
        #[serde(default = "default_f32_1")]
        gain: f32,
        #[serde(alias = "iter", default = "default_iterations")]
        iterations: usize,
        #[serde(alias = "sym", default = "default_f32_1")]
        symmetry: f32,
        #[serde(default = "default_f32_0")]
        bias: f32,
        #[serde(default = "default_f32_1")]
        shape: f32,

        // TODO: make Option and consider None to mean "adaptive range"?
        #[serde(default = "default_normalized_range")]
        range: (f32, f32),
    },
}

//------------------------------------------------------------------------------
// Helper Types & Functions
//------------------------------------------------------------------------------

#[derive(Debug)]
struct ParsedKeyframe {
    beats: f32,
    value: f32,
}

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
fn default_f32_0() -> f32 {
    0.0
}
fn default_f32_1() -> f32 {
    1.0
}
fn default_f32_0_25() -> f32 {
    0.25
}
fn default_f32_0_3() -> f32 {
    0.3
}
fn default_f32_0_7() -> f32 {
    0.7
}
fn default_f32_0_5() -> f32 {
    0.5
}
