//! Provides a means of controlling sketch parameters with the various Lattice
//! control systems from an external yaml file that can be hot-reloaded.

use indexmap::IndexMap;
use notify::{Event, RecursiveMode, Watcher};
use serde::{Deserialize, Deserializer};
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    error::Error,
    fmt, fs,
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
};
use yaml_merge_keys::merge_keys_serde_yml;

use crate::framework::frame_controller;

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
    effects: RefCell<HashMap<String, (EffectConfig, Effect)>>,
    aliases: HashMap<String, String>,
    persisted_config_fields: HashMap<String, PersistedConfigFields>,
    node_graph: NodeGraph,
    node_execution_order: Option<Vec<String>>,
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
            persisted_config_fields: HashMap::new(),
            node_graph: NodeGraph::new(),
            node_execution_order: None,
            eval_cache: RefCell::new(HashMap::new()),
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

        if let Some(bypass) = self
            .persisted_config_fields
            .get(name)
            .map(|p| p.bypass)
            .flatten()
        {
            return bypass;
        }

        if self.node_execution_order.is_some()
            && self.node_graph.contains_key(name)
        {
            let frame_count = frame_controller::frame_count();

            if let Some(order) = &self.node_execution_order {
                for node in order.iter() {
                    if node == name {
                        debug!("get; node == name {}; breaking", node);
                        break;
                    }
                    {
                        let eval_cache = self.eval_cache.borrow();
                        if eval_cache.contains_key(node) {
                            let cached_frame_count =
                                eval_cache.get(node).unwrap().0;
                            if cached_frame_count == frame_count {
                                debug!("get; {} is cached. moving along", node);
                                continue;
                            }
                        }
                    }
                    let value = self.get_raw(node);
                    debug!("get; node={} raw_value: {}", node, value);
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
            Effect::Hysteresis(m) => m.apply(value),
            Effect::Quantizer(m) => m.apply(value),
            Effect::Saturator(m) => m.apply(value),
            Effect::SlewLimiter(m) => m.apply(value),
            Effect::WaveFolder(m) => m.apply(value),
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
        effect: &mut impl ParamHost,
        node_name: &str,
    ) {
        if let Some(params) = self.node_graph.get(node_name) {
            for (param_name, param_value) in params.iter() {
                let value = match param_value {
                    ParamValue::Cold(value) => *value,
                    ParamValue::Hot(name) => self.get_raw(name),
                };
                effect.set_param(param_name, value);
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

    fn resolve_animation_config_params<P: ParamHost + Clone>(
        &self,
        config: &P,
        node_name: &str,
    ) -> P {
        debug!("Resolving params for {}", node_name);
        let mut config = config.clone();
        if let Some(params) = self.node_graph.get(node_name) {
            for (param_name, param_value) in params.iter() {
                let value = match param_value {
                    ParamValue::Cold(value) => *value,
                    ParamValue::Hot(name) => self.get_raw(name),
                };
                config.set_param(param_name, value);
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
        self.persisted_config_fields.clear();
        self.node_graph.clear();
        self.node_execution_order = None;
        self.eval_cache.borrow_mut().clear();

        for (id, maybe_config) in control_configs {
            let config = match maybe_config {
                MaybeControlConfig::Control(config) => config,
                MaybeControlConfig::Other(_) => continue,
            };

            let hot_params = self.find_hot_params(&config.config);

            if !hot_params.is_empty() {
                self.node_graph.insert(id.to_string(), hot_params);
            }

            if let Some(v) = config.config.get("var").and_then(|v| v.as_str()) {
                self.aliases.insert(v.to_string(), id.to_string());
            }

            let bypass = config
                .config
                .get("bypass")
                .and_then(|b| b.as_f64())
                .map(|b| b as f32);

            if bypass.is_some()
            /* || possible_future_field.etc.is_some() */
            {
                self.persisted_config_fields
                    .insert(id.to_string(), PersistedConfigFields { bypass });
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

        self.node_execution_order = self.sort_dep_graph();

        trace!("node_graph: {:#?}", self.node_graph);

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

    fn sort_dep_graph(&self) -> Option<Vec<String>> {
        let (graph, mut in_degree) = self.build_dep_graph();

        let mut queue: VecDeque<String> = VecDeque::new();
        let mut sorted_order: Vec<String> = Vec::new();

        for (node, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node.clone());
            }
        }

        while let Some(node) = queue.pop_front() {
            sorted_order.push(node.clone());

            if let Some(deps) = graph.get(&node) {
                for dep in deps {
                    if let Some(count) = in_degree.get_mut(dep) {
                        *count -= 1;
                        if *count == 0 {
                            queue.push_back(dep.clone());
                        }
                    }
                }
            }
        }

        if sorted_order.len() == in_degree.len() {
            Some(sorted_order)
        } else {
            warn!(
                "cycle detected. sorted_order: {:?}, in_degree: {:?}",
                sorted_order, in_degree
            );
            None
        }
    }

    fn build_dep_graph(
        &self,
    ) -> (HashMap<String, Vec<String>>, HashMap<String, usize>) {
        // { dependency: [dependents] }
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        // node_graph = { "hot_effect": { param: Hot("hot_anim") }, ... }
        // node_name = "hot_effect"
        // node = { param: Hot("hot_anim") }
        for (node_name, params) in self.node_graph.iter() {
            // value = Hot("hot_anim")
            for (_, value) in params.iter() {
                // hot_value = "hot_anim"
                if let ParamValue::Hot(hot_value) = value {
                    in_degree.entry(hot_value.clone()).or_insert(0);

                    // graph = { "hot_anim": ["hot_effect"] }
                    // "hot_effect depends on hot_anim"
                    graph
                        .entry(hot_value.clone())
                        .or_default()
                        .push(node_name.clone());

                    *in_degree.entry(node_name.clone()).or_insert(0) += 1;
                }
            }
        }

        (graph, in_degree)
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
    bypass: Option<f32>,
    #[serde(default)]
    var: Option<String>,
}

struct PersistedConfigFields {
    bypass: Option<f32>,
}

#[derive(Clone, Debug)]
enum ParamValue {
    Cold(f32),
    Hot(String),
}

impl ParamValue {
    fn as_float(&self) -> f32 {
        match self {
            ParamValue::Cold(x) => *x,
            ParamValue::Hot(_) => {
                loud_panic!("Cannot get float from ParamValue::Hot")
            }
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

// { "symmetry": Param::Hot("t1") }
type Node = HashMap<String, ParamValue>;

// { "object_name": { "symmetry": Param::Hot("t1") } } }
type NodeGraph = HashMap<String, Node>;

trait ParamHost {
    fn set_param(&mut self, name: &str, value: f32);
}

impl ParamHost for TestEffect {
    fn set_param(&mut self, name: &str, value: f32) {
        match name {
            "param" => self.param = value,
            _ => {
                warn!("TestEffect does not support param name {}", name)
            }
        }
    }
}

//------------------------------------------------------------------------------
// Animation Types
//------------------------------------------------------------------------------

#[derive(Debug)]
enum AnimationConfig {
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
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    range: [f32; 2],
    default: f32,
    step: f32,
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
struct CheckboxConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    default: bool,
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
struct SelectConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    options: Vec<String>,
    default: String,
}

#[derive(Deserialize, Debug)]
struct Separator {}

// --- External Controls

#[derive(Deserialize, Debug)]
#[serde(default)]
struct OscConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    range: [f32; 2],
    default: f32,
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
struct AudioConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
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

// --- Animation Controls

#[derive(Clone, Deserialize, Debug)]
struct LerpAbsConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    #[serde(default)]
    delay: f32,
    keyframes: Vec<(String, f32)>,
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
struct LerpRelConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    #[serde(default)]
    delay: f32,
    keyframes: Vec<(f32, f32)>,
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
struct RRampRelConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    #[serde(default)]
    delay: f32,
    #[serde(default)]
    ramp_time: f32,
    #[serde(default = "default_ramp")]
    ramp: String,
    keyframes: Vec<(f32, (f32, f32))>,
}

fn default_ramp() -> String {
    "linear".to_string()
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
struct TriangleConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    beats: ParamValue,
    range: [f32; 2],
    phase: ParamValue,
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

impl ParamHost for TriangleConfig {
    fn set_param(&mut self, name: &str, value: f32) {
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
struct AutomateConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    breakpoints: Vec<BreakpointConfig>,
    #[serde(default = "default_mode")]
    mode: String,
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

#[derive(Clone, Deserialize, Debug)]
struct TestAnimConfig {
    field: ParamValue,
}

impl ParamHost for TestAnimConfig {
    fn set_param(&mut self, name: &str, value: f32) {
        match name {
            "field" => self.field = ParamValue::Cold(value),
            _ => {
                warn!("TestAnimConfig does not support param name {}", name)
            }
        }
    }
}

// --- Modulation & Effects

#[derive(Clone, Deserialize, Debug)]
struct ModulationConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    source: String,
    modulators: Vec<String>,
}

#[derive(Clone, Deserialize, Debug)]
struct EffectConfig {
    #[allow(dead_code)]
    #[serde(flatten)]
    shared: Shared,
    #[serde(flatten)]
    kind: EffectKind,
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
            EffectKind::Saturator { drive, range } => {
                Effect::Saturator(Saturator::new(drive, range))
            }
            EffectKind::SlewLimiter { rise, fall } => {
                Effect::SlewLimiter(SlewLimiter::new(rise, fall))
            }
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
            EffectKind::Test { param } => Effect::TestEffect(TestEffect {
                param: f32::from(param),
            }),
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

    #[serde(alias = "test")]
    Test { param: ParamValue },
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
