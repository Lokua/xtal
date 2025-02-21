//! Provides a means of controlling sketch parameters with the various Lattice
//! control systems from an external yaml file that can be hot-reloaded.

use notify::{Event, RecursiveMode, Watcher};
use std::{
    cell::RefCell,
    collections::HashMap,
    error::Error,
    fs,
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
};
use yaml_merge_keys::merge_keys_serde_yml;

use crate::framework::frame_controller;
use crate::framework::{control_script::config::SliderConfig, prelude::*};

use super::{
    config::{
        AnimationConfig, AudioConfig, AutomateConfig, CheckboxConfig,
        ConfigFile, ControlType, EffectConfig, EffectKind, KeyframeSequence,
        LerpAbsConfig, LerpRelConfig, MaybeControlConfig, ModulationConfig,
        OscConfig, RRampRelConfig, SelectConfig, TestAnimConfig,
        TriangleConfig,
    },
    dep_graph::{DepGraph, Node},
    param_mod::{ParamValue, SetFromParam},
};

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
    effects: RefCell<HashMap<String, (EffectConfig, Effect)>>,
    aliases: HashMap<String, String>,
    bypassed: HashMap<String, Option<f32>>,
    dep_graph: DepGraph,
    eval_cache: RefCell<HashMap<String, (u32, f32)>>,
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
            effects: RefCell::new(HashMap::new()),
            aliases: HashMap::new(),
            bypassed: HashMap::new(),
            eval_cache: RefCell::new(HashMap::new()),
            dep_graph: DepGraph::new(),
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
        let name = &self.resolve_name(name);

        if let Some(bypass) = self.bypassed.get(name).and_then(|x| *x) {
            return bypass;
        }

        if self.dep_graph.has_dependents(name) {
            let frame_count = frame_controller::frame_count();

            if let Some(order) = &self.dep_graph.order() {
                for node in order.iter() {
                    if node == name {
                        break;
                    }
                    {
                        let eval_cache = self.eval_cache.borrow();
                        if eval_cache.contains_key(node) {
                            let cached_frame_count =
                                eval_cache.get(node).unwrap().0;
                            if cached_frame_count == frame_count {
                                continue;
                            }
                        }
                    }
                    let value = self.get_raw(node);
                    self.eval_cache
                        .borrow_mut()
                        .insert(node.clone(), (frame_count, value));
                }
            }
        }

        let value = self.get_raw(name);

        match self.modulations.get(name) {
            None => value,
            Some(modulators) => self.apply_modulators(value, modulators),
        }
    }

    fn resolve_name(&self, name: &str) -> String {
        self.aliases.get(name).cloned().unwrap_or(name.to_string())
    }

    fn apply_modulators(
        &self,
        initial_value: f32,
        modulators: &Vec<String>,
    ) -> f32 {
        modulators.iter().fold(initial_value, |value, modulator| {
            self.apply_modulator(value, modulator)
        })
    }

    fn apply_modulator(&self, value: f32, modulator: &str) -> f32 {
        let mut effects = self.effects.borrow_mut();

        if !effects.contains_key(modulator) {
            return value * self.get_raw(modulator);
        }

        let (config, effect) = effects.get_mut(modulator).unwrap();

        if let (
            EffectKind::RingModulator { modulator, .. },
            Effect::RingModulator(m),
        ) = (&config.kind, &mut *effect)
        {
            return m.apply(value, self.get_raw(modulator));
        }

        match effect {
            Effect::Hysteresis(m) => {
                self.update_effect_params(m, modulator);
                m.apply(value)
            }
            Effect::Quantizer(m) => {
                self.update_effect_params(m, modulator);
                m.apply(value)
            }
            Effect::Saturator(m) => {
                self.update_effect_params(m, modulator);
                m.apply(value)
            }
            Effect::SlewLimiter(m) => {
                self.update_effect_params(m, modulator);
                m.apply(value)
            }
            Effect::WaveFolder(m) => {
                self.update_effect_params(m, modulator);
                m.apply(value)
            }
            Effect::TestEffect(m) => {
                self.update_effect_params(m, modulator);
                m.apply(value)
            }
            // special case already handled above
            Effect::RingModulator(_) => panic!(),
        }
    }

    fn update_effect_params(
        &self,
        effect: &mut impl SetFromParam,
        node_name: &str,
    ) {
        if let Some(params) = self.dep_graph.node(node_name) {
            for (param_name, param_value) in params.iter() {
                let value = match param_value {
                    ParamValue::Cold(value) => *value,
                    ParamValue::Hot(name) => self.get_raw(name),
                };
                effect.set(param_name, value);
            }
        }
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
                    AnimationConfig::LerpRel(_),
                    KeyframeSequence::Linear(kfs),
                ) => self.animation.lerp(&kfs.clone(), config.delay()),
                (
                    AnimationConfig::LerpAbs(_),
                    KeyframeSequence::Linear(kfs),
                ) => self.animation.lerp(&kfs.clone(), config.delay()),
                (
                    AnimationConfig::RRampRel(conf),
                    KeyframeSequence::Random(kfs),
                ) => self.animation.r_ramp(
                    kfs,
                    conf.delay,
                    conf.ramp_time,
                    Easing::from_str(conf.ramp.as_str()).unwrap(),
                ),
                (AnimationConfig::Triangle(conf), KeyframeSequence::None) => {
                    let conf = self.resolve_animation_config_params(conf, name);
                    debug!("Triangle animation with beats: {:?}, range: {:?}, phase: {:?}", 
                        conf.beats, conf.range, conf.phase);
                    self.animation.triangle(
                        conf.beats.as_float(),
                        (conf.range[0], conf.range[1]),
                        conf.phase.as_float(),
                    )
                }
                (
                    AnimationConfig::Automate(conf),
                    KeyframeSequence::Breakpoints(breakpoints),
                ) => self
                    .animation
                    .automate(breakpoints, Mode::from_str(&conf.mode).unwrap()),
                (AnimationConfig::TestAnim(conf), KeyframeSequence::None) => {
                    let conf = self.resolve_animation_config_params(conf, name);
                    conf.field.as_float() + 100.0
                }
                _ => unimplemented!(),
            };
        }

        warn_once!("No control named {}. Defaulting to 0.0", name);
        0.0
    }

    fn resolve_animation_config_params<P: SetFromParam + Clone>(
        &self,
        config: &P,
        node_name: &str,
    ) -> P {
        debug!("Resolving params for {}", node_name);
        let mut config = config.clone();
        if let Some(params) = self.dep_graph.node(node_name) {
            for (param_name, param_value) in params.iter() {
                let value = match param_value {
                    ParamValue::Cold(value) => *value,
                    ParamValue::Hot(name) => self.get_raw(name),
                };
                config.set(param_name, value);
            }
        }
        config
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
        self.aliases.clear();
        self.bypassed.clear();
        self.dep_graph.clear();
        self.eval_cache.borrow_mut().clear();

        for (id, maybe_config) in control_configs {
            let config = match maybe_config {
                MaybeControlConfig::Control(config) => config,
                MaybeControlConfig::Other(_) => continue,
            };

            let hot_params = self.find_hot_params(&config.config);
            if !hot_params.is_empty() {
                self.dep_graph.insert_node(id, hot_params);
            }

            if let Some(v) = config.config.get("var").and_then(|v| v.as_str()) {
                self.aliases.insert(v.to_string(), id.to_string());
            }

            let bypass = config
                .config
                .get("bypass")
                .and_then(|b| b.as_f64())
                .map(|b| b as f32);

            if bypass.is_some() {
                self.bypassed.insert(id.to_string(), bypass);
            }

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
                            AnimationConfig::LerpAbs(conf),
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
                            AnimationConfig::LerpRel(conf),
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

                    self.effects.borrow_mut().insert(
                        id.to_string(),
                        (conf.clone(), Effect::from(conf)),
                    );
                }
                ControlType::TestAnim => {
                    let conf: TestAnimConfig =
                        serde_yml::from_value(config.config.clone())?;

                    self.keyframe_sequences.insert(
                        id.to_string(),
                        (
                            AnimationConfig::TestAnim(conf),
                            KeyframeSequence::None,
                        ),
                    );
                }
            }
        }

        self.dep_graph.build_graph();
        trace!("node_graph: {:#?}", self.dep_graph.graph());

        if !self.osc_controls.is_active {
            self.osc_controls
                .start()
                .expect("Unable to start OSC receiver");
        }

        self.controls.mark_changed();

        info!("Controls populated");

        Ok(())
    }

    fn find_hot_params(&self, raw_config: &serde_yml::Value) -> Node {
        let mut node_values = Node::new();

        if let Some(obj) = raw_config.as_mapping() {
            for (key, value) in obj {
                if let Ok(param) =
                    serde_yml::from_value::<ParamValue>(value.clone())
                {
                    if let ParamValue::Hot(_) = param {
                        let k = key.as_str().unwrap().to_string();
                        node_values.insert(k, param);
                    }
                }
                // here is we'd recursively check for nested hots if we
                // ever want to support that in the future
            }
        }

        node_values
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

struct UpdateState {
    _watcher: notify::RecommendedWatcher,
    state: Arc<Mutex<Option<ConfigFile>>>,
}

#[derive(Debug)]
struct ParsedKeyframe {
    beats: f32,
    value: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    // 1 frame = 1/16; 4 frames per beat; 16 frames per bar
    use crate::framework::animation::animation_tests::{init, BPM};

    fn create_instance() -> ControlScript<FrameTiming> {
        ControlScript::new(
            to_absolute_path(file!(), "control_script.yaml"),
            FrameTiming::new(BPM),
        )
    }

    #[test]
    #[serial]
    fn test_parameter_modulation() {
        let controls = create_instance();

        init(0);
        assert_eq!(controls.get("test_triangle"), 0.5);
    }
}
