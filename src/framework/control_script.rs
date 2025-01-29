use notify::{Event, RecursiveMode, Watcher};
use serde::Deserialize;
use std::{
    collections::HashMap,
    error::Error,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

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
}

type ConfigFile = HashMap<String, MaybeControlConfig>;

struct UpdateState {
    _watcher: notify::RecommendedWatcher,
    state: Arc<Mutex<Option<ConfigFile>>>,
}

pub struct ControlScript<T: TimingSource> {
    /// The raw yaml representation that represents the last valid parsed state
    control_configs: Option<ConfigFile>,
    pub controls: Controls,
    osc_controls: OscControls,
    animation: Animation<T>,
    keyframe_sequences: HashMap<String, (LerpAbsConfig, Vec<Keyframe>)>,
    update_state: UpdateState,
}

impl<T: TimingSource> ControlScript<T> {
    pub fn new(path: PathBuf, timing: T) -> Self {
        let state = Arc::new(Mutex::new(None));
        let state_clone = state.clone();

        let mut script = Self {
            control_configs: None,
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

        if self.keyframe_sequences.contains_key(name) {
            let (config, keyframes) =
                self.keyframe_sequences.get(name).unwrap();
            return self.animation.lerp(keyframes.clone(), config.delay);
        }

        error!("No control named {}", name);
        panic!()
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
            if let Err(e) = self.apply_config(config) {
                error!("Failed to apply new configuration: {:?}", e);
            }
        }
    }

    fn import_script(&mut self, path: &PathBuf) -> Result<(), Box<dyn Error>> {
        let config = Self::parse_config(path)?;
        self.apply_config(config)
    }

    fn parse_config(path: &PathBuf) -> Result<ConfigFile, Box<dyn Error>> {
        let file_content = fs::read_to_string(path)?;
        let config = serde_yml::from_str(&file_content)?;
        Ok(config)
    }

    fn apply_config(
        &mut self,
        config: ConfigFile,
    ) -> Result<(), Box<dyn Error>> {
        self.control_configs = Some(config);

        self.controls = Controls::with_previous(vec![]);
        self.osc_controls = OscControls::new();
        self.keyframe_sequences.clear();

        self.populate_controls()?;
        self.osc_controls.start()?;

        Ok(())
    }

    fn populate_controls(&mut self) -> Result<(), Box<dyn Error>> {
        let control_configs = match &self.control_configs {
            Some(configs) => configs,
            None => return Ok(()),
        };

        for (id, maybe_config) in control_configs {
            let config = match maybe_config {
                MaybeControlConfig::Control(config) => config,
                MaybeControlConfig::Other(_) => continue,
            };

            match config.control_type {
                ControlType::Slider => {
                    let conf: SliderConfig =
                        serde_yml::from_value(config.config.clone())?;

                    let slider = Control::slider(
                        id.as_str(),
                        conf.default,
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

                    self.keyframe_sequences
                        .insert(id.to_string(), (conf, keyframes));
                }
            }
        }

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
                        info!("Loaded new configuration.");
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

#[derive(Clone, Deserialize, Debug)]
struct LerpAbsConfig {
    #[serde(default)]
    delay: f32,
    keyframes: Vec<(String, f32)>,
}

impl Default for LerpAbsConfig {
    fn default() -> Self {
        Self {
            delay: 0.0,
            keyframes: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct ParsedKeyframe {
    beats: f32,
    value: f32,
}
