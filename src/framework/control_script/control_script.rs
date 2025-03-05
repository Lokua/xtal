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

use crate::framework::{
    control_script::config::MidiConfig, frame_controller, prelude::*,
};

use super::{
    config::*,
    dep_graph::{DepGraph, Node},
    eval_cache::EvalCache,
    param_mod::{FromColdParams, ParamValue, SetFromParam},
};

struct SnapshotTransition {
    values: HashMap<String, (f32, f32)>,
    start_frame: u32,
    end_frame: u32,
}

pub struct ControlScript<T: TimingSource> {
    pub controls: Controls,
    pub animation: Animation<T>,
    midi_controls: MidiControls,
    osc_controls: OscControls,
    audio_controls: AudioControls,
    animations: HashMap<String, (AnimationConfig, KeyframeSequence)>,
    modulations: HashMap<String, Vec<String>>,
    effects: RefCell<HashMap<String, (EffectConfig, Effect)>>,
    aliases: HashMap<String, String>,
    bypassed: HashMap<String, Option<f32>>,
    dep_graph: DepGraph,
    eval_cache: EvalCache,
    update_state: Option<UpdateState>,
    snapshots: HashMap<String, ControlValues>,
    active_transition: Option<SnapshotTransition>,
    transition_time: f32,
}

impl<T: TimingSource> ControlScript<T> {
    pub fn new(yaml_str: &str, timing: T) -> Self {
        let mut script = Self {
            controls: Controls::with_previous(vec![]),
            midi_controls: MidiControls::new(),
            osc_controls: OscControls::new(),
            audio_controls: AudioControlBuilder::new().build(),
            animation: Animation::new(timing),
            animations: HashMap::new(),
            modulations: HashMap::new(),
            effects: RefCell::new(HashMap::new()),
            aliases: HashMap::new(),
            bypassed: HashMap::new(),
            eval_cache: EvalCache::new(),
            dep_graph: DepGraph::new(),
            update_state: None,
            snapshots: HashMap::new(),
            active_transition: None,
            transition_time: 4.0,
        };

        let config =
            Self::parse_from_str(yaml_str).expect("Unable to parse yaml");

        script
            .populate_controls(&config)
            .expect("Unable to populate controls");

        script
    }

    pub fn from_path(path: PathBuf, timing: T) -> Self {
        let state = Arc::new(Mutex::new(None));
        let state_clone = state.clone();

        let file_content =
            fs::read_to_string(&path).expect("Unable to read file");

        let mut script = Self::new(&file_content, timing);

        script.update_state = Some(UpdateState {
            state: state.clone(),
            _watcher: Self::setup_watcher(path.clone(), state_clone),
        });

        script
    }

    pub fn get(&self, name: &str) -> f32 {
        let name = &self.aliases.get(name).cloned().unwrap_or(name.to_string());

        if let Some(transition) = &self.active_transition {
            if let Some((from, to)) = transition.values.get(name) {
                return self.get_tweened(
                    *from,
                    *to,
                    transition.start_frame,
                    transition.end_frame,
                );
            }
        }

        if let Some(bypass) = self.bypassed.get(name).and_then(|x| *x) {
            return bypass;
        }

        if self.dep_graph.has_dependents(name) {
            self.run_dependencies(name);
        }

        let value = self.get_raw(name);

        match self.modulations.get(name) {
            None => value,
            Some(modulators) => modulators
                .iter()
                .fold(value, |v, modulator| self.apply_modulator(v, modulator)),
        }
    }

    fn get_tweened(
        &self,
        from: f32,
        to: f32,
        start_frame: u32,
        end_frame: u32,
    ) -> f32 {
        let current_frame = frame_controller::frame_count();
        debug!("start_frame == end_frame: {}", start_frame == end_frame);
        if current_frame > end_frame || start_frame == end_frame {
            debug!("returning value: {}", to);
            return to;
        }
        let duration = end_frame - start_frame;
        let progress = current_frame - start_frame;
        let t = progress as f32 / duration as f32;
        lerp(from, to, t)
    }

    fn run_dependencies(&self, target_name: &str) {
        if let Some(order) = &self.dep_graph.order() {
            let frame_count = frame_controller::frame_count();

            for name in order.iter() {
                if name == target_name {
                    break;
                }

                if self.eval_cache.has(name, frame_count) {
                    continue;
                }

                self.get_raw(name);
            }
        }
    }

    fn apply_modulator(&self, value: f32, modulator: &str) -> f32 {
        let mut effects = self.effects.borrow_mut();

        if !effects.contains_key(modulator) {
            return value * self.get_raw(modulator);
        }

        let (config, effect) = effects.get_mut(modulator).unwrap();

        let modulated = if let (
            EffectKind::RingModulator { modulator, .. },
            Effect::RingModulator(m),
        ) = (&config.kind, &mut *effect)
        {
            m.apply(value, self.get_raw(modulator))
        } else {
            match effect {
                Effect::Hysteresis(m) => {
                    self.update_effect_params(m, modulator);
                    m.apply(value)
                }
                Effect::Math(m) => {
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
                Effect::RingModulator(_) => panic!(),
            }
        };

        modulated
    }

    fn update_effect_params(
        &self,
        effect: &mut impl SetFromParam,
        node_name: &str,
    ) {
        if let Some(params) = self.dep_graph.node(node_name) {
            for (param_name, param_value) in params.iter() {
                let value = param_value.cold_or(|name| self.get_raw(&name));
                effect.set_from_param(param_name, value);
            }
        }
    }

    fn get_raw(&self, name: &str) -> f32 {
        let frame_count = frame_controller::frame_count();

        if self.eval_cache.has(name, frame_count) {
            let (_, value) = self.eval_cache.get(name).unwrap();
            return value;
        }

        let mut value = None;

        if self.controls.has(name) {
            value = Some(self.controls.float(name));
        }

        if self.osc_controls.has(name) {
            value = Some(self.osc_controls.get(name));
        }

        if self.midi_controls.has(name) {
            value = Some(self.midi_controls.get(name));
        }

        if self.audio_controls.has(name) {
            value = Some(self.audio_controls.get(name));
        }

        if let Some((config, sequence)) = self.animations.get(name) {
            let v = match (config, sequence) {
                (AnimationConfig::Triangle(conf), KeyframeSequence::None) => {
                    let conf = self.resolve_animation_config_params(conf, name);
                    self.animation.triangle(
                        conf.beats.as_float(),
                        (conf.range[0], conf.range[1]),
                        conf.phase.as_float(),
                    )
                }
                (
                    AnimationConfig::Automate(conf),
                    KeyframeSequence::Breakpoints(breakpoints),
                ) => {
                    let breakpoints =
                        self.resolve_breakpoint_params(name, breakpoints);

                    self.animation.automate(
                        &breakpoints,
                        Mode::from_str(&conf.mode).unwrap(),
                    )
                }
                _ => unimplemented!(),
            };

            value = Some(v);
        }

        if value.is_some() {
            let value = value.unwrap();
            self.eval_cache.store(name, frame_count, value);
            return value;
        } else {
            warn_once!("No control named {}. Defaulting to 0.0", name);
            return 0.0;
        }
    }

    fn resolve_breakpoint_params(
        &self,
        node_name: &str,
        breakpoints: &Vec<Breakpoint>,
    ) -> Vec<Breakpoint> {
        let mut breakpoints = breakpoints.clone();

        if let Some(params) = self.dep_graph.node(node_name) {
            for (param_name, param_value) in params.iter() {
                let path_segments: Vec<&str> = param_name.split(".").collect();

                if path_segments.len() < 3 {
                    error!("Unrecognized keypath format: {}", param_name);
                    continue;
                }

                if let Some(index) = path_segments[1].parse::<usize>().ok() {
                    let value = param_value.cold_or(|name| self.get_raw(&name));
                    breakpoints[index].set_from_param(&param_name, value);
                }
            }
        }

        breakpoints
    }

    fn resolve_animation_config_params<
        P: SetFromParam + Clone + std::fmt::Debug,
    >(
        &self,
        config: &P,
        node_name: &str,
    ) -> P {
        let mut config = config.clone();

        if let Some(params) = self.dep_graph.node(node_name) {
            for (param_name, param_value) in params.iter() {
                let value = param_value.cold_or(|name| self.get_raw(&name));
                config.set_from_param(param_name, value);
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
        self.animations
            .get(name)
            .and_then(|(_, sequence)| match sequence {
                KeyframeSequence::Breakpoints(breakpoints) => {
                    Some(breakpoints.clone())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("No breakpoints for name: {}", name))
    }

    pub fn take_snapshot(&mut self, id: &str) {
        let mut snapshot: ControlValues = ControlValues::new();

        snapshot.extend(self.controls.values().clone());
        snapshot.extend(self.midi_controls.values().iter().map(
            |(key, value)| (key.clone(), ControlValue::from(value.clone())),
        ));
        snapshot.extend(self.osc_controls.values().iter().map(
            |(key, value)| (key.clone(), ControlValue::from(value.clone())),
        ));

        self.snapshots.insert(id.to_string(), snapshot);
    }

    pub fn recall_snapshot(&mut self, id: &str) {
        if let Some(snapshot) = self.snapshots.get(id) {
            let frame_count = frame_controller::frame_count();
            let duration =
                self.animation.beats_to_frames(self.transition_time) as u32;

            let mut transition = SnapshotTransition {
                values: HashMap::new(),
                start_frame: frame_count,
                end_frame: frame_count + duration,
            };

            for (name, value) in snapshot {
                if self.controls.has(&name) {
                    match value {
                        ControlValue::Float(v) => {
                            transition.values.insert(
                                name.to_string(),
                                (self.get_raw(name), *v),
                            );
                        }
                        ControlValue::Bool(_) | ControlValue::String(_) => {
                            self.controls.update_value(&name, value.clone());
                        }
                    }
                    continue;
                }

                if self.midi_controls.has(&name) || self.osc_controls.has(&name)
                {
                    transition.values.insert(
                        name.to_string(),
                        (self.get_raw(name), value.as_float().unwrap()),
                    );
                    continue;
                }
            }

            self.active_transition = Some(transition);
        }
    }

    pub fn delete_snapshot(&mut self, id: &str) {
        self.snapshots.remove(id);
    }

    pub fn clear_snapshots(&mut self) {
        self.snapshots.clear()
    }

    pub fn set_transition_time(&mut self, transition_time: f32) {
        self.transition_time = transition_time;
    }

    pub fn update(&mut self) {
        let new_config = {
            if let Some(update_state) = &self.update_state {
                if let Ok(mut guard) = update_state.state.lock() {
                    guard.take()
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(config) = new_config {
            if let Err(e) = self.populate_controls(&config) {
                error!("Failed to apply new configuration: {:?}", e);
            }
        }

        if let Some(transition) = &self.active_transition {
            if frame_controller::frame_count() > transition.end_frame {
                for (name, (_from, to)) in &transition.values {
                    if self.controls.has(&name) {
                        let value = ControlValue::Float(*to);
                        self.controls.update_value(name, value);
                        continue;
                    }
                    if self.midi_controls.has(&name) {
                        self.midi_controls.update_value(name, *to);
                        continue;
                    }
                    if self.osc_controls.has(&name) {
                        self.osc_controls.update_value(name, *to);
                        continue;
                    }
                }
                self.active_transition = None;
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

    fn parse_from_str(yaml_str: &str) -> Result<ConfigFile, Box<dyn Error>> {
        let raw_config = serde_yml::from_str(&yaml_str)?;
        let merged_config = merge_keys_serde_yml(raw_config)?;
        let config: ConfigFile = serde_yml::from_value(merged_config)?;
        Ok(config)
    }

    fn parse_from_path(path: &PathBuf) -> Result<ConfigFile, Box<dyn Error>> {
        let file_content = fs::read_to_string(path)?;
        let config = Self::parse_from_str(&file_content)?;
        Ok(config)
    }

    fn populate_controls(
        &mut self,
        control_configs: &ConfigFile,
    ) -> Result<(), Box<dyn Error>> {
        let current_values: ControlValues = self.controls.values().clone();
        let osc_values: HashMap<String, f32> = self.osc_controls.values();
        let midi_values: HashMap<String, f32> = self.midi_controls.values();

        self.controls = Controls::with_previous(vec![]);
        self.animations.clear();
        self.modulations.clear();
        self.aliases.clear();
        self.bypassed.clear();
        self.dep_graph.clear();
        self.eval_cache.clear();

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

                    let existing_value = if osc_values.contains_key(id) {
                        osc_values.get(id)
                    } else {
                        None
                    };

                    let osc_control = OscControlConfig::new(
                        &id,
                        (conf.range[0], conf.range[1]),
                        conf.default,
                    );

                    self.osc_controls
                        .add(&osc_control.address, osc_control.clone());

                    if let Some(value) = existing_value {
                        self.osc_controls.set(&osc_control.address, *value);
                    }
                }
                ControlType::Midi => {
                    let conf: MidiConfig =
                        serde_yml::from_value(config.config.clone())?;

                    let existing_value = if midi_values.contains_key(id) {
                        midi_values.get(id)
                    } else {
                        None
                    };

                    let midi_control = MidiControlConfig::new(
                        (conf.channel, conf.cc),
                        (conf.range[0], conf.range[1]),
                        conf.default,
                    );

                    self.midi_controls.add(&id, midi_control);

                    if let Some(value) = existing_value {
                        self.midi_controls.set(&id, *value);
                    }
                }
                ControlType::Audio => {
                    let conf: AudioConfig =
                        serde_yml::from_value(config.config.clone())?;

                    let audio_control = AudioControlConfig::new(
                        conf.channel,
                        SlewLimiter::new(conf.slew[0], conf.slew[1]),
                        conf.detect,
                        conf.pre,
                        (conf.range[0], conf.range[1]),
                        0.0,
                    );

                    self.audio_controls.add(id, audio_control);
                }
                ControlType::Triangle => {
                    let conf: TriangleConfig =
                        serde_yml::from_value(config.config.clone())?;

                    self.animations.insert(
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

                    self.animations.insert(
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

                    let effect = match conf.kind {
                        EffectKind::Hysteresis { pass_through, .. } => {
                            let mut effect =
                                Hysteresis::from_cold_params(&conf);
                            effect.pass_through = pass_through;
                            Effect::Hysteresis(effect)
                        }
                        EffectKind::Math {
                            operator: ref op, ..
                        } => {
                            let mut effect = Math::from_cold_params(&conf);
                            effect.operator = Operator::from_str(&op).unwrap();
                            Effect::Math(effect)
                        }
                        EffectKind::Quantizer { range, .. } => {
                            let mut effect = Quantizer::from_cold_params(&conf);
                            effect.set_range(range);
                            Effect::Quantizer(effect)
                        }
                        EffectKind::RingModulator { range, .. } => {
                            let mut effect =
                                RingModulator::from_cold_params(&conf);
                            effect.set_range(range);
                            Effect::RingModulator(effect)
                        }
                        EffectKind::Saturator { range, .. } => {
                            let mut effect = Saturator::from_cold_params(&conf);
                            effect.set_range(range);
                            Effect::Saturator(effect)
                        }
                        EffectKind::SlewLimiter { .. } => Effect::SlewLimiter(
                            SlewLimiter::from_cold_params(&conf),
                        ),
                        EffectKind::WaveFolder {
                            iterations, range, ..
                        } => {
                            let mut effect =
                                WaveFolder::from_cold_params(&conf);
                            effect.iterations = iterations;
                            effect.set_range(range);
                            Effect::WaveFolder(effect)
                        }
                    };

                    self.effects
                        .borrow_mut()
                        .insert(id.to_string(), (conf.clone(), effect));
                }
            }
        }

        self.dep_graph.build_graph();
        trace!("node_graph: {:#?}", self.dep_graph);

        if !self.osc_controls.is_active {
            self.osc_controls
                .start()
                .expect("Unable to start OSC receiver");
        }

        if !self.midi_controls.is_active() {
            if let Err(e) = self.midi_controls.start() {
                warn!("Unable to start MIDI receiver. {}", e);
            }
        }

        self.controls.mark_changed();

        info!("Controls populated");

        Ok(())
    }

    fn find_hot_params(&self, raw_config: &serde_yml::Value) -> Node {
        let mut hot_params = Node::new();

        let obj = match raw_config.as_mapping() {
            Some(mapping) => mapping,
            None => return hot_params,
        };

        for (key, value) in obj {
            let key_str = key.as_str().unwrap().to_string();

            if let Some(param) = self.try_parse_hot_param(value) {
                hot_params.insert(key_str, param);
                continue;
            }

            if let Some(sequence) = value.as_sequence() {
                for (index, item) in sequence.iter().enumerate() {
                    let node = self.find_hot_params(item);

                    for (k, value) in node.iter() {
                        let keypath = format!("{}.{}.{}", key_str, index, k);
                        let node = Node::from([(keypath, value.clone())]);
                        hot_params.extend(node);
                    }
                }
            }
        }

        hot_params
    }

    fn try_parse_hot_param(
        &self,
        value: &serde_yml::Value,
    ) -> Option<ParamValue> {
        serde_yml::from_value::<ParamValue>(value.clone())
            .ok()
            .filter(|param| matches!(param, ParamValue::Hot(_)))
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

            match Self::parse_from_path(&path) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    // 1 frame = 1/16; 4 frames per beat; 16 frames per bar
    use crate::framework::animation::animation_tests::{init, BPM};

    fn create_instance(yaml: &str) -> ControlScript<FrameTiming> {
        ControlScript::new(yaml, FrameTiming::new(Bpm::new(BPM)))
    }

    #[test]
    #[serial]
    fn test_parameter_modulation() {
        let controls = create_instance(
            r#"
slider:
  type: slider
  default: 0.5

triangle:
  type: triangle
  beats: 4
  phase: $slider

                "#,
        );

        init(0);
        assert_eq!(
            controls.get("triangle"),
            0.5,
            "[slider->0.5] * [triangle->1.0]"
        );
    }

    #[test]
    #[serial]
    fn test_parameter_modulation_effect() {
        let controls = create_instance(
            r#"
triangle:
  type: triangle
  beats: 4

slider:
  type: slider
  default: 0.33

effect:
  type: effect
  kind: hysteresis
  upper_threshold: 0.55
  lower_threshold: 0.1
  output_low: 0
  output_high: $slider

test_mod:
  type: mod 
  source: triangle 
  modulators:
    - effect

            "#,
        );

        init(6);
        assert_eq!(
            controls.get("triangle"),
            0.33,
            "[triangle->0.75] -> [slider->effect.hi]"
        );
    }

    #[test]
    #[serial]
    fn test_parameter_modulation_breakpoint() {
        let controls = create_instance(
            r#"
slider: 
  type: slider 
  default: 40

automate:
  type: automate 
  breakpoints:
    - position: 0
      value: $slider
      kind: step 
      
            "#,
        );

        init(0);
        assert_eq!(
            controls.get("automate"),
            40.0,
            "[automate.0.value]<-[$slider@40]"
        );
    }

    #[test]
    #[serial]
    fn test_snapshot() {
        let mut controls = create_instance(
            r#"
a: 
  type: slider 
  default: 10
b: 
  type: midi
  default: 20
c: 
  type: osc 
  default: 30
      
            "#,
        );

        controls.set_transition_time(0.0);
        controls.take_snapshot("foo");

        controls
            .controls
            .update_value("a", ControlValue::Float(100.0));
        controls.midi_controls.update_value("b", 200.0);
        controls.osc_controls.update_value("c", 300.0);
        controls.take_snapshot("bar");

        init(0);
        controls.recall_snapshot("bar");
        controls.update();
        assert_eq!(controls.get("a"), 100.0);
        assert_eq!(controls.get("b"), 200.0);
        assert_eq!(controls.get("c"), 300.0);

        init(1);
        controls.update();
        controls.recall_snapshot("foo");
        assert_eq!(controls.get("a"), 10.0);
        assert_eq!(controls.get("b"), 20.0);
        assert_eq!(controls.get("c"), 30.0);
    }
}
