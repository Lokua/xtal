//! Provides a means of controlling sketch parameters with the various Lattice
//! control systems from an external yaml file that can be hot-reloaded.
#![doc = include_str!("../../../docs/control_script_reference.md")]

use nannou::rand::{Rng, thread_rng};
use notify::{Event, RecursiveMode, Watcher};
use std::{
    cell::RefCell,
    error::Error,
    fs,
    path::PathBuf,
    str::FromStr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use yaml_merge_keys::merge_keys_serde_yml;

use super::{
    config::*,
    dep_graph::{DepGraph, Node},
    eval_cache::EvalCache,
    param_mod::{FromColdParams, ParamValue, SetFromParam},
};
#[cfg(feature = "instrumentation")]
use crate::framework::instrumentation::Instrumentation;
use crate::{
    framework::{frame_controller, prelude::*},
    runtime::{map_mode::MapMode, serialization::SaveableProgramState},
};

pub const TRANSITION_TIMES: [f32; 15] = [
    32.0, 24.0, 16.0, 12.0, 16.0, 8.0, 6.0, 4.0, 3.0, 2.0, 1.5, 1.0, 0.75, 0.5,
    0.25,
];

#[derive(Debug)]
struct UpdateState {
    #[allow(dead_code)]
    watcher: notify::RecommendedWatcher,
    state: Arc<Mutex<Option<ConfigFile>>>,

    /// Optimization to speed up checking for changes vs having to acquire a
    /// lock on the above state mutex
    has_changes: Arc<AtomicBool>,
}

#[derive(Debug)]
struct SnapshotTransition {
    values: HashMap<String, (f32, f32)>,
    start_frame: u32,
    end_frame: u32,
}

pub type Snapshots = HashMap<String, ControlValues>;

struct Callback(Box<dyn Fn()>);

impl Callback {
    fn call(&self) {
        (self.0)();
    }
}

impl std::fmt::Debug for Callback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Callback")
    }
}

#[derive(Debug)]
pub struct ControlHub<T: TimingSource> {
    pub animation: Animation<T>,
    pub ui_controls: UiControls,
    pub midi_controls: MidiControls,
    pub osc_controls: OscControls,
    pub audio_controls: AudioControls,
    pub snapshots: Snapshots,
    animations: HashMap<String, (AnimationConfig, KeyframeSequence)>,
    modulations: HashMap<String, Vec<String>>,
    effects: RefCell<HashMap<String, (EffectConfig, Effect)>>,

    /// Used to allow `get` to be called with the name used in a YAML `var`
    /// field. See ./docs/control_script_reference.md **Using `var`** section
    /// for more info.
    aliases: HashMap<String, String>,
    bypassed: HashMap<String, Option<f32>>,
    dep_graph: DepGraph,
    eval_cache: EvalCache,
    update_state: Option<UpdateState>,
    active_transition: Option<SnapshotTransition>,
    transition_time: f32,
    snapshot_ended_callbacks: Vec<Callback>,
    populated_callbacks: Vec<Callback>,
    #[cfg(feature = "instrumentation")]
    instrumentation: RefCell<Instrumentation>,
}

impl<T: TimingSource> ControlHub<T> {
    pub fn new(yaml_str: Option<&str>, timing: T) -> Self {
        let mut script = Self {
            ui_controls: UiControls::with_previous(vec![]),
            midi_controls: MidiControls::new(),
            osc_controls: OscControls::new(),
            audio_controls: AudioControlBuilder::new().build(),
            animation: Animation::new(timing),
            animations: HashMap::default(),
            modulations: HashMap::default(),
            effects: RefCell::new(HashMap::default()),
            aliases: HashMap::default(),
            bypassed: HashMap::default(),
            eval_cache: EvalCache::new(),
            dep_graph: DepGraph::new(),
            update_state: None,
            snapshots: HashMap::default(),
            active_transition: None,
            transition_time: 4.0,
            snapshot_ended_callbacks: vec![],
            populated_callbacks: vec![],
            #[cfg(feature = "instrumentation")]
            instrumentation: RefCell::new(Instrumentation::new(
                "ControlScript::get",
            )),
        };

        if let Some(yaml) = yaml_str {
            let config =
                Self::parse_from_str(yaml).expect("Unable to parse yaml");

            script
                .populate_controls(&config)
                .expect("Unable to populate controls");
        }

        script
    }

    pub fn from_path(path: PathBuf, timing: T) -> Self {
        let state = Arc::new(Mutex::new(None));
        let state_clone = state.clone();

        let file_content =
            fs::read_to_string(&path).expect("Unable to read file");

        let mut script = Self::new(Some(&file_content), timing);
        let has_changes = Arc::new(AtomicBool::new(false));

        script.update_state = Some(UpdateState {
            watcher: Self::setup_watcher(
                path.clone(),
                state_clone,
                has_changes.clone(),
            ),
            state: state.clone(),
            has_changes,
        });

        script
    }

    pub fn get(&self, name: &str) -> f32 {
        #[cfg(feature = "instrumentation")]
        let start = self.instrumentation.borrow().start();

        let current_frame = frame_controller::frame_count();

        let mut name = match self.aliases.get(name) {
            Some(alias) => alias,
            None => name,
        };

        let midi_proxy_name = MapMode::proxy_name(name);
        if self.midi_controls.has(&midi_proxy_name) {
            name = &midi_proxy_name;
        }

        if let Some(transition) = &self.active_transition {
            if let Some((from, to)) = transition.values.get(name) {
                let tweened = self.get_tweened(
                    current_frame,
                    *from,
                    *to,
                    transition.start_frame,
                    transition.end_frame,
                );
                return tweened;
            }
        }

        if let Some(Some(bypass)) = self.bypassed.get(name) {
            return *bypass;
        }

        self.run_dependencies(name, current_frame);

        let value = self.get_raw(name, current_frame);

        let result = match self.modulations.get(name) {
            None => value,
            Some(modulators) => {
                modulators.iter().fold(value, |v, modulator| {
                    self.apply_modulators(v, modulator, current_frame)
                })
            }
        };

        #[cfg(feature = "instrumentation")]
        self.instrumentation.borrow_mut().record(start);

        result
    }

    fn get_tweened(
        &self,
        current_frame: u32,
        from: f32,
        to: f32,
        start_frame: u32,
        end_frame: u32,
    ) -> f32 {
        if current_frame > end_frame || start_frame == end_frame {
            return to;
        }
        let duration = end_frame - start_frame;
        let progress = current_frame - start_frame;
        let t = progress as f32 / duration as f32;
        lerp(from, to, t)
    }

    fn run_dependencies(&self, target_name: &str, current_frame: u32) {
        if let Some(order) = &self.dep_graph.order() {
            for name in order.iter() {
                if name == target_name {
                    break;
                }

                if self.eval_cache.has(name, current_frame) {
                    continue;
                }

                self.get_raw(name, current_frame);
            }
        }
    }

    fn apply_modulators(
        &self,
        value: f32,
        modulator: &str,
        current_frame: u32,
    ) -> f32 {
        let mut effects = self.effects.borrow_mut();

        if !effects.contains_key(modulator) {
            return value * self.get_raw(modulator, current_frame);
        }

        let (config, effect) = effects.get_mut(modulator).unwrap();

        if let (
            EffectKind::RingModulator { modulator, .. },
            Effect::RingModulator(m),
        ) = (&config.kind, &mut *effect)
        {
            m.apply(value, self.get_raw(modulator, current_frame))
        } else {
            match effect {
                Effect::Constrain(m) => m.apply(value),
                Effect::Hysteresis(m) => {
                    self.update_effect_params(m, modulator, current_frame);
                    m.apply(value)
                }
                Effect::Map(m) => m.apply(value),
                Effect::Math(m) => {
                    self.update_effect_params(m, modulator, current_frame);
                    m.apply(value)
                }
                Effect::Quantizer(m) => {
                    self.update_effect_params(m, modulator, current_frame);
                    m.apply(value)
                }
                Effect::Saturator(m) => {
                    self.update_effect_params(m, modulator, current_frame);
                    m.apply(value)
                }
                Effect::SlewLimiter(m) => {
                    self.update_effect_params(m, modulator, current_frame);
                    m.apply(value)
                }
                Effect::WaveFolder(m) => {
                    self.update_effect_params(m, modulator, current_frame);
                    m.apply(value)
                }
                Effect::RingModulator(_) => panic!(),
            }
        }
    }

    fn update_effect_params(
        &self,
        effect: &mut impl SetFromParam,
        node_name: &str,
        current_frame: u32,
    ) {
        if let Some(params) = self.dep_graph.node(node_name) {
            for (param_name, param_value) in params.iter() {
                let value = param_value.cold_or(|name| {
                    if let Some(Some(bypass_value)) = self.bypassed.get(&name) {
                        *bypass_value
                    } else {
                        self.get_raw(&name, current_frame)
                    }
                });
                effect.set_from_param(param_name, value);
            }
        }
    }

    fn get_raw(&self, name: &str, current_frame: u32) -> f32 {
        let is_dep = self.dep_graph.is_dependency(name);

        if is_dep {
            if let Some(value) = self.eval_cache.get(name, current_frame) {
                return value;
            }
        }

        let value = if self.ui_controls.has(name) {
            Some(self.ui_controls.get(name))
        } else if self.osc_controls.has(name) {
            Some(self.osc_controls.get(name))
        } else if self.midi_controls.has(name) {
            Some(self.midi_controls.get(name))
        } else if self.audio_controls.has(name) {
            Some(self.audio_controls.get(name))
        } else if let Some((config, sequence)) = self.animations.get(name) {
            Some(match (config, sequence) {
                (AnimationConfig::Triangle(conf), KeyframeSequence::None) => {
                    let conf = self.resolve_animation_config_params(
                        conf,
                        name,
                        current_frame,
                    );
                    self.animation.triangle(
                        conf.beats.as_float(),
                        (conf.range[0], conf.range[1]),
                        conf.phase.as_float(),
                    )
                }
                (AnimationConfig::Random(conf), KeyframeSequence::None) => {
                    let conf = self.resolve_animation_config_params(
                        conf,
                        name,
                        current_frame,
                    );
                    self.animation.random(
                        conf.beats.as_float(),
                        (conf.range[0], conf.range[1]),
                        conf.delay.as_float(),
                        conf.stem,
                    )
                }
                (
                    AnimationConfig::RandomSlewed(conf),
                    KeyframeSequence::None,
                ) => {
                    let conf = self.resolve_animation_config_params(
                        conf,
                        name,
                        current_frame,
                    );
                    self.animation.random_slewed(
                        conf.beats.as_float(),
                        (conf.range[0], conf.range[1]),
                        conf.slew.as_float(),
                        conf.delay.as_float(),
                        conf.stem,
                    )
                }
                (
                    AnimationConfig::Automate(conf),
                    KeyframeSequence::Breakpoints(breakpoints),
                ) => {
                    let breakpoints = self.resolve_breakpoint_params(
                        name,
                        breakpoints,
                        current_frame,
                    );
                    self.animation.automate(
                        &breakpoints,
                        Mode::from_str(&conf.mode).unwrap(),
                    )
                }
                _ => unimplemented!(),
            })
        } else {
            None
        };

        match value {
            Some(value) => {
                if is_dep {
                    self.eval_cache.store(name, current_frame, value);
                }
                value
            }
            None => {
                warn_once!("No control named {}. Defaulting to 0.0", name);
                0.0
            }
        }
    }

    fn resolve_breakpoint_params(
        &self,
        node_name: &str,
        breakpoints: &[Breakpoint],
        current_frame: u32,
    ) -> Vec<Breakpoint> {
        let mut breakpoints = breakpoints.to_vec();

        if let Some(params) = self.dep_graph.node(node_name) {
            for (param_name, param_value) in params.iter() {
                let path_segments: Vec<&str> = param_name.split(".").collect();

                if path_segments.len() < 3 {
                    error!("Unrecognized keypath format: {}", param_name);
                    continue;
                }

                if let Ok(index) = path_segments[1].parse::<usize>() {
                    let value = param_value.cold_or(|name| {
                        if let Some(Some(bypass_value)) =
                            self.bypassed.get(&name)
                        {
                            *bypass_value
                        } else {
                            self.get_raw(&name, current_frame)
                        }
                    });
                    breakpoints[index].set_from_param(param_name, value);
                }
            }
        }

        breakpoints
    }

    fn resolve_animation_config_params<P>(
        &self,
        config: &P,
        node_name: &str,
        current_frame: u32,
    ) -> P
    where
        P: SetFromParam + Clone + std::fmt::Debug,
    {
        let mut config = config.clone();

        if let Some(params) = self.dep_graph.node(node_name) {
            for (param_name, param_value) in params.iter() {
                let value = param_value.cold_or(|name| {
                    if let Some(Some(bypass_value)) = self.bypassed.get(&name) {
                        *bypass_value
                    } else {
                        self.get_raw(&name, current_frame)
                    }
                });
                config.set_from_param(param_name, value);
            }
        }

        config
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

    pub fn bypassed(&self) -> HashMap<String, f32> {
        self.bypassed
            .iter()
            .filter_map(|(k, v)| v.map(|f| (k.clone(), f)))
            .collect()
    }

    /// Helper to create snapshot (values only)
    fn create_snapshot(&mut self) -> HashMap<String, ControlValue> {
        let mut snapshot: ControlValues = ControlValues::default();

        snapshot.extend(
            self.ui_controls
                .values()
                .iter()
                .filter_map(|(name, value)| {
                    if self.ui_controls.config(name).unwrap().is_separator() {
                        None
                    } else {
                        Some((name.clone(), value.clone()))
                    }
                })
                .collect::<ControlValues>(),
        );
        snapshot.extend(
            self.midi_controls
                .values()
                .iter()
                .map(|(key, value)| (key.clone(), ControlValue::from(*value))),
        );
        snapshot.extend(
            self.osc_controls
                .values()
                .iter()
                .map(|(key, value)| (key.clone(), ControlValue::from(*value))),
        );

        snapshot
    }

    /// Create and store a snapshot for later recall
    pub fn take_snapshot(&mut self, id: &str) {
        let snapshot = self.create_snapshot();
        self.snapshots.insert(id.to_string(), snapshot);
    }

    pub fn recall_snapshot(&mut self, id: &str) -> Result<(), String> {
        match self.snapshots.get(id) {
            Some(snapshot) => {
                let current_frame = frame_controller::frame_count();
                let duration =
                    self.animation.beats_to_frames(self.transition_time) as u32;

                let mut transition = SnapshotTransition {
                    values: HashMap::default(),
                    start_frame: current_frame,
                    end_frame: current_frame + duration,
                };

                for (name, value) in snapshot {
                    if self.ui_controls.has(name) {
                        match value {
                            ControlValue::Float(v) => {
                                transition.values.insert(
                                    name.to_string(),
                                    (self.get_raw(name, current_frame), *v),
                                );
                            }
                            ControlValue::Bool(_) | ControlValue::String(_) => {
                                // Just update immediately since we can't
                                // interpolate over a bool and interpolating
                                // over static select options is likely to yield
                                // undesired results
                                self.ui_controls
                                    .update_value(name, value.clone());
                            }
                        }
                        continue;
                    }

                    if self.midi_controls.has(name)
                        || self.osc_controls.has(name)
                    {
                        transition.values.insert(
                            name.to_string(),
                            (
                                self.get_raw(name, current_frame),
                                value.as_float().unwrap(),
                            ),
                        );
                        continue;
                    }
                }

                self.active_transition = Some(transition);

                Ok(())
            }
            None => Err(format!("No snapshot for {}", id)),
        }
    }

    pub fn delete_snapshot(&mut self, id: &str) {
        self.snapshots.remove(id);
    }

    pub fn clear_snapshots(&mut self) {
        self.snapshots.clear()
    }

    pub fn register_snapshot_ended_callback<F>(&mut self, callback: F)
    where
        F: Fn() + 'static,
    {
        self.snapshot_ended_callbacks
            .push(Callback(Box::new(callback)));
    }

    pub fn set_transition_time(&mut self, transition_time: f32) {
        self.transition_time = transition_time;
    }

    pub fn snapshot_keys_sorted(&self) -> Vec<String> {
        let mut keys: Vec<_> = self.snapshots.keys().cloned().collect();
        keys.sort();
        keys
    }

    /// Uses the [`Self::active_transition`] to store a temporary snapshot of
    /// randomized parameter values. See [this commit][commit] for the original
    /// frontend POC (App.tsx)
    ///
    /// [commit]: https://github.com/Lokua/lattice/commit/bcb1328
    pub fn randomize(&mut self) {
        let current_frame = frame_controller::frame_count();
        let duration =
            self.animation.beats_to_frames(self.transition_time) as u32;

        let mut transition = SnapshotTransition {
            values: HashMap::default(),
            start_frame: current_frame,
            end_frame: current_frame + duration,
        };

        for (name, value) in &self.create_snapshot() {
            if self.ui_controls.has(name) {
                match value {
                    ControlValue::Float(_) => {
                        if let Control::Slider { min, max, step, .. } =
                            self.ui_controls.config(name).unwrap()
                        {
                            let from = self.get_raw(name, current_frame);
                            let to =
                                random_within_range_stepped(*min, *max, *step);
                            transition
                                .values
                                .insert(name.to_string(), (from, to));
                        }
                    }
                    ControlValue::Bool(_) => {
                        // Just update immediately since we can't interpolate
                        // over a bool
                        self.ui_controls.update_value(
                            name,
                            ControlValue::from(random_bool()),
                        );
                    }
                    ControlValue::String(_) => {
                        if let Control::Select { options, .. } =
                            self.ui_controls.config(name).unwrap()
                        {
                            // Just update immediately since interpolating over
                            // static select options is likely to yield
                            // undesired results
                            let index =
                                thread_rng().gen_range(0..options.len());

                            self.ui_controls.update_value(
                                name,
                                ControlValue::from(options[index].clone()),
                            );
                        }
                    }
                }
            } else if self.midi_controls.has(name) {
                let config = self.midi_controls.config(name).unwrap();
                transition.values.insert(
                    name.to_string(),
                    (
                        self.get_raw(name, current_frame),
                        thread_rng().gen_range(config.min..=config.max),
                    ),
                );
            } else if self.osc_controls.has(name) {
                let config = self.osc_controls.config(name).unwrap();
                transition.values.insert(
                    name.to_string(),
                    (
                        self.get_raw(name, current_frame),
                        thread_rng().gen_range(config.min..=config.max),
                    ),
                );
            } else {
                error!("Unsupported snapshot value: {} {:?}", name, value);
            }
        }

        // Executes the transition immediately
        self.active_transition = Some(transition);
    }

    pub fn update(&mut self) {
        let new_config = {
            if let Some(update_state) = &self.update_state {
                if update_state.has_changes.load(Ordering::Acquire) {
                    update_state.has_changes.store(false, Ordering::Release);

                    if let Ok(mut guard) = update_state.state.lock() {
                        guard.take()
                    } else {
                        None
                    }
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
                for (name, (from, to)) in &transition.values {
                    if self.ui_controls.has(name) {
                        let value = ControlValue::Float(*to);
                        self.ui_controls.update_value(name, value);
                        continue;
                    } else if self.midi_controls.has(name) {
                        self.midi_controls.update_value(name, *to);
                        continue;
                    } else if self.osc_controls.has(name) {
                        self.osc_controls.update_value(name, *to);
                        continue;
                    }
                }
                self.active_transition = None;
                for callback in &self.snapshot_ended_callbacks {
                    callback.call();
                }
            }
        }
    }

    pub fn merge_program_state(&mut self, state: &SaveableProgramState) {
        for (k, v) in state.ui_controls.values().iter() {
            self.ui_controls.update_value(k, v.clone());
        }

        for (k, v) in state
            .midi_controls
            .configs()
            .iter()
            .filter(|(k, _)| MapMode::is_proxy_name(k))
        {
            // order of operations is important here as `add` sets value to the
            // config's default
            let value = state.midi_controls.get(k);
            self.midi_controls.add(k, v.clone());
            self.midi_controls.update_value(k, value);
        }

        for (k, v) in state.osc_controls.values().iter() {
            self.osc_controls.update_value(k, *v);
        }

        for (k, v) in state.snapshots.clone() {
            self.snapshots.insert(k, v);
        }
    }

    pub fn register_populated_callback<F>(&mut self, callback: F)
    where
        F: Fn() + 'static,
    {
        self.populated_callbacks.push(Callback(Box::new(callback)));
    }

    pub fn add_controls(&mut self, configs: Vec<Control>) {
        self.ui_controls.extend(configs);
    }
    pub fn float(&self, name: &str) -> f32 {
        self.get(name)
    }
    pub fn bool(&self, name: &str) -> bool {
        self.ui_controls.bool(name)
    }
    pub fn bool_as_f32(&self, name: &str) -> f32 {
        self.ui_controls.bool_as_f32(name)
    }
    pub fn string(&self, name: &str) -> String {
        self.ui_controls.string(name)
    }
    pub fn changed(&self) -> bool {
        self.ui_controls.changed()
    }
    pub fn any_changed_in(&self, names: &[&str]) -> bool {
        self.ui_controls.any_changed_in(names)
    }
    pub fn mark_unchanged(&mut self) {
        self.ui_controls.mark_unchanged();
    }
    pub fn hrcc(&mut self, hrcc: bool) {
        self.midi_controls.hrcc = hrcc;
    }

    fn parse_from_str(yaml_str: &str) -> Result<ConfigFile, Box<dyn Error>> {
        let raw_config = serde_yml::from_str(yaml_str)?;
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
        let current_values: ControlValues = self.ui_controls.values().clone();
        let osc_values: HashMap<String, f32> = self
            .osc_controls
            .values()
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();

        let midi_values: HashMap<String, f32> = self
            .midi_controls
            .values()
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();

        self.ui_controls = UiControls::with_previous(vec![]);
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
                    let mut conf: SliderConfig =
                        serde_yml::from_value(config.config.clone())?;

                    let value = current_values
                        .get(id)
                        .and_then(ControlValue::as_float)
                        .unwrap_or(conf.default);

                    let disabled = Self::extract_disabled_fn(&mut conf.shared);

                    let slider = Control::Slider {
                        name: id.to_string(),
                        value,
                        min: conf.range[0],
                        max: conf.range[1],
                        step: conf.step,
                        disabled,
                    };

                    self.ui_controls.add(slider);
                }
                ControlType::Checkbox => {
                    let mut conf: CheckboxConfig =
                        serde_yml::from_value(config.config.clone())?;

                    let value = current_values
                        .get(id)
                        .and_then(ControlValue::as_bool)
                        .unwrap_or(conf.default);

                    let disabled = Self::extract_disabled_fn(&mut conf.shared);

                    let checkbox = Control::Checkbox {
                        name: id.to_string(),
                        value,
                        disabled,
                    };
                    self.ui_controls.add(checkbox);
                }
                ControlType::Select => {
                    let mut conf: SelectConfig =
                        serde_yml::from_value(config.config.clone())?;

                    let value = current_values
                        .get(id)
                        .and_then(ControlValue::as_string)
                        .unwrap_or(conf.default.as_str());

                    let disabled = Self::extract_disabled_fn(&mut conf.shared);

                    let select = Control::Select {
                        name: id.to_string(),
                        value: value.to_string(),
                        options: conf.options,
                        disabled,
                    };

                    self.ui_controls.add(select);
                }
                ControlType::Separator => {
                    self.ui_controls.add(Control::dynamic_separator());
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
                        id,
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

                    self.midi_controls.add(id, midi_control);

                    if let Some(value) = existing_value {
                        self.midi_controls.set(id, *value);
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
                ControlType::Random => {
                    let conf: RandomConfig =
                        serde_yml::from_value(config.config.clone())?;

                    self.animations.insert(
                        id.to_string(),
                        (AnimationConfig::Random(conf), KeyframeSequence::None),
                    );
                }
                ControlType::RandomSlewed => {
                    let conf: RandomSlewedConfig =
                        serde_yml::from_value(config.config.clone())?;

                    self.animations.insert(
                        id.to_string(),
                        (
                            AnimationConfig::RandomSlewed(conf),
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
                        EffectKind::Constrain { ref mode, range } => {
                            Effect::Constrain(
                                Constrain::try_from((
                                    mode.as_str(),
                                    range.0,
                                    range.1,
                                ))
                                .unwrap_or(Constrain::None),
                            )
                        }
                        EffectKind::Hysteresis { pass_through, .. } => {
                            let mut effect =
                                Hysteresis::from_cold_params(&conf);
                            effect.pass_through = pass_through;
                            Effect::Hysteresis(effect)
                        }
                        EffectKind::Map { domain, range } => {
                            Effect::Map(Map::new(domain, range))
                        }
                        EffectKind::Math {
                            operator: ref op, ..
                        } => {
                            let mut effect = Math::from_cold_params(&conf);
                            effect.operator = Operator::from_str(op).unwrap();
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

        for callback in &self.populated_callbacks {
            callback.call();
        }

        self.ui_controls.mark_changed();

        info!("Controls populated");

        Ok(())
    }

    fn extract_disabled_fn(shared: &mut Shared) -> DisabledFn {
        if let Some(disabled_config) = &mut shared.disabled {
            disabled_config.disabled_fn.take()
        } else {
            None
        }
    }

    fn find_hot_params(&self, raw_config: &serde_yml::Value) -> Node {
        let mut hot_params = Node::default();

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
                        let mut node = Node::default();
                        node.insert(keypath, value.clone());
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
        has_changes: Arc<AtomicBool>,
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
                        info!(
                            "Loaded new configuration. \
                            DON'T FORGET TO CALL UPDATE!"
                        );
                        has_changes.store(true, Ordering::Release);
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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    // 1 frame = 1/16; 4 frames per beat; 16 frames per bar
    use crate::framework::motion::animation_tests::{BPM, init};

    fn create_instance(yaml: &str) -> ControlHub<FrameTiming> {
        ControlHub::new(Some(yaml), FrameTiming::new(Bpm::new(BPM)))
    }

    #[test]
    #[serial]
    #[ignore]
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
    #[ignore]
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
    #[ignore]
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
    #[ignore]
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
            .ui_controls
            .update_value("a", ControlValue::Float(100.0));
        controls.midi_controls.update_value("b", 200.0);
        controls.osc_controls.update_value("c", 300.0);
        controls.take_snapshot("bar");

        init(0);
        controls.recall_snapshot("bar").unwrap();
        controls.update();
        assert_eq!(controls.get("a"), 100.0);
        assert_eq!(controls.get("b"), 200.0);
        assert_eq!(controls.get("c"), 300.0);

        init(1);
        controls.update();
        controls.recall_snapshot("foo").unwrap();
        assert_eq!(controls.get("a"), 10.0);
        assert_eq!(controls.get("b"), 20.0);
        assert_eq!(controls.get("c"), 30.0);
    }

    #[test]
    #[serial]
    fn test_disabled() {
        let hub = create_instance(
            r#"
foo:
  type: slider
  disabled: bar is a and baz

bar:
  type: select
  default: a
  options: [a, b, c]

baz:
  type: checkbox
  default: true
            "#,
        );

        assert_eq!(hub.ui_controls.disabled("foo"), Some(true));
    }
}
