use indexmap::IndexMap;
use notify::{Event, RecursiveMode, Watcher};
use serde::{Deserialize, Deserializer};
use std::{
    collections::HashMap,
    error::Error,
    fmt, fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use yaml_merge_keys::merge_keys_serde_yml;

use super::prelude::*;

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
    #[serde(rename = "slider")]
    Slider,
    #[serde(rename = "osc")]
    Osc,
    #[serde(rename = "lerp_abs")]
    LerpAbs,
    #[serde(rename = "lerp_rel")]
    LerpRel,
    #[serde(rename = "r_ramp_rel")]
    RRampRel,
    #[serde(rename = "triangle")]
    Triangle,
}

type ConfigFile = IndexMap<String, MaybeControlConfig>;

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

enum AnimationConfig {
    Lerp(LerpConfig),
    RRampRel(RRampRelConfig),
    Triangle(TriangleConfig),
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
        }
    }
}

#[derive(Clone, Debug)]
enum KeyframeSequence {
    Linear(Vec<Keyframe>),
    Random(Vec<KeyframeRandom>),
    None,
}

pub struct ControlScript<T: TimingSource> {
    pub controls: Controls,
    osc_controls: OscControls,
    animation: Animation<T>,
    keyframe_sequences: HashMap<String, (AnimationConfig, KeyframeSequence)>,
    update_state: UpdateState,
}

impl<T: TimingSource> ControlScript<T> {
    pub fn new(path: PathBuf, timing: T) -> Self {
        let state = Arc::new(Mutex::new(None));
        let state_clone = state.clone();

        let mut script = Self {
            controls: Controls::with_previous(vec![]),
            osc_controls: OscControls::new(),
            animation: Animation::new(timing),
            keyframe_sequences: HashMap::new(),
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
        if name.starts_with("/") {
            return self.osc_controls.get(name);
        }

        if self.controls.has(name) {
            return self.controls.float(name);
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
                        self.animation.lerp(kfs.clone(), config.delay())
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
                            str_to_fn_unary(conf.ramp.as_str()),
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
                _ => unimplemented!(),
            };
        }

        loud_panic!("No control named {}", name);
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

        self.controls = Controls::with_previous(vec![]);
        self.osc_controls = OscControls::new();
        self.keyframe_sequences.clear();

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
                ControlType::Osc => {
                    let conf: SliderConfig =
                        serde_yml::from_value(config.config.clone())?;

                    let osc_control = OscControlConfig::new(
                        format!("/{}", id).as_str(),
                        (conf.range[0], conf.range[1]),
                        conf.default,
                    );

                    self.osc_controls
                        .add(&osc_control.address, osc_control.clone());
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

                    trace!("raw_conf Triangle: {:?}", conf);

                    self.keyframe_sequences.insert(
                        id.to_string(),
                        (
                            AnimationConfig::Triangle(conf),
                            KeyframeSequence::None,
                        ),
                    );
                }
            }
        }

        trace!("Config populated. controls: {:?}, osc_controls: {:?}, keyframe_sequences: {:?}", 
            self.controls, self.osc_controls, self.keyframe_sequences);

        self.osc_controls
            .start()
            .expect("Unable to start OSC receiver");

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
            default: 0.5,
            step: 0.000_1,
        }
    }
}

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
            default: 0.5,
        }
    }
}

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
