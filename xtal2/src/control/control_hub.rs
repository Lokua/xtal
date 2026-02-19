//! Provides a means of controlling sketch parameters with the various Xtal
//! control systems from an external yaml file that can be hot-reloaded. See the
//! [Control Script Reference][ref]
//!
//! [ref]: https://github.com/Lokua/xtal/blob/main/docs/control_script_reference.md

use log::{debug, error, info, trace, warn};
use notify::{Event, RecursiveMode, Watcher};
use rand::Rng;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use yaml_merge_keys::merge_keys_serde_yml;

use super::config::*;
use super::dep_graph::{DepGraph, Node};
use super::eval_cache::EvalCache;
use super::map_mode::MapMode;
use super::param_mod::{FromColdParams, ParamValue, SetFromParam};

use crate::framework::{frame_controller, prelude::*};
use crate::{ternary, warn_once};

pub const TRANSITION_TIMES: [f32; 16] = [
    32.0, 24.0, 16.0, 12.0, 16.0, 8.0, 6.0, 4.0, 3.0, 2.0, 1.5, 1.0, 0.75, 0.5,
    0.25, 0.0,
];

const WATCHER_CHANGE_INFO_DEBOUNCE: Duration = Duration::from_millis(150);

#[derive(Debug)]
struct UpdateState {
    #[allow(dead_code)]
    watcher: notify::RecommendedWatcher,
    path: PathBuf,
    state: Arc<Mutex<Option<ConfigFile>>>,

    /// Optimization to speed up checking for changes vs having to acquire a
    /// lock on the above state mutex
    has_changes: Arc<AtomicBool>,
}

#[derive(Debug)]
struct SnapshotTransition {
    values: HashMap<String, (f32, f32)>,
    start_beat: f32,
    end_beat: f32,
}

struct SnapshotSequenceRuntime {
    sequence_length: f32,
    disabled: DisabledFn,
    last_phase: Option<f32>,
}

impl Default for SnapshotSequenceRuntime {
    fn default() -> Self {
        Self {
            sequence_length: 0.0,
            disabled: None,
            last_phase: None,
        }
    }
}

impl std::fmt::Debug for SnapshotSequenceRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SnapshotSequenceRuntime")
    }
}

pub type Snapshots = HashMap<String, ControlValues>;

pub type Exclusions = Vec<String>;

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

/// The single point of entry for all Xtal controls and animations. When
/// declaring controls and animations in Rust code, use the
/// [`crate::prelude::ControlHubBuilder`], otherwise if using a [Control
/// Script][script-ref], see [`Self::from_path`].
///
/// [script-ref]: https://github.com/Lokua/xtal/blob/main/docs/control_script_reference.md
#[derive(Debug)]
pub struct ControlHub<T: TimingSource> {
    pub animation: Animation<T>,
    pub ui_controls: UiControls,
    pub midi_controls: MidiControls,
    pub osc_controls: OscControls,
    pub audio_controls: AudioControls,
    pub snapshots: Snapshots,
    pub midi_proxies_enabled: bool,
    animations: HashMap<String, (AnimationConfig, KeyframeSequence)>,
    modulations: HashMap<String, Vec<String>>,
    effects: RefCell<HashMap<String, (EffectConfig, Effect)>>,

    /// Map of `var => name` Used to allow `get` to be called with the name used
    /// in a YAML `var` field. See ./docs/control_script_reference.md **Using
    /// `var`** section for more info.
    vars: HashMap<String, String>,
    bypassed: HashMap<String, Option<f32>>,
    dep_graph: DepGraph,
    eval_cache: EvalCache,
    update_state: Option<UpdateState>,
    active_transition: Option<SnapshotTransition>,
    transition_time: f32,
    snapshot_sequence: Option<SnapshotSequenceConfig>,
    snapshot_sequence_runtime: SnapshotSequenceRuntime,
    snapshot_ended_callbacks: Vec<Callback>,
    populated_callbacks: Vec<Callback>,
    preserve_values_on_reload: bool,
}

impl<T: TimingSource> ControlHub<T> {
    pub fn new(yaml_str: Option<&str>, timing: T) -> Self {
        let mut script = Self {
            ui_controls: UiControls::default(),
            midi_controls: MidiControls::default(),
            osc_controls: OscControls::default(),
            audio_controls: AudioControlBuilder::new().build(),
            animation: Animation::new(timing),
            animations: HashMap::default(),
            modulations: HashMap::default(),
            effects: RefCell::new(HashMap::default()),
            vars: HashMap::default(),
            bypassed: HashMap::default(),
            eval_cache: EvalCache::default(),
            dep_graph: DepGraph::default(),
            update_state: None,
            snapshots: HashMap::default(),
            active_transition: None,
            transition_time: 4.0,
            snapshot_sequence: None,
            snapshot_sequence_runtime: SnapshotSequenceRuntime::default(),
            snapshot_ended_callbacks: vec![],
            populated_callbacks: vec![],
            midi_proxies_enabled: true,
            preserve_values_on_reload: true,
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

    /// Instantiate a hub instance from a YAML control script. It is recommended
    /// to place your script next to your sketch.rs file:
    ///
    /// # Example
    /// ```rs
    /// // my_sketch.rs
    /// pub fn init(app: &App, ctx: &Context) -> MySketch {
    ///     let hub = ControlHub::from_path(
    ///         to_absolute_path(file!(), "my_sketch.yaml"),
    ///         Timing::new(ctx.bpm()),
    ///     );
    ///
    ///     MySketch { hub }
    /// }
    /// ```
    pub fn from_path(path: PathBuf, timing: T) -> Self {
        let state = Arc::new(Mutex::new(None));
        let state_clone = state.clone();

        let file_content =
            fs::read_to_string(&path).expect("Unable to read file");
        let initial_content_hash = content_hash(&file_content);

        let mut script = Self::new(Some(&file_content), timing);
        let has_changes = Arc::new(AtomicBool::new(false));

        script.update_state = Some(UpdateState {
            watcher: Self::setup_watcher(
                path.clone(),
                state_clone,
                has_changes.clone(),
                Some(initial_content_hash),
            ),
            path,
            state: state.clone(),
            has_changes,
        });

        script
    }

    pub fn get(&self, name: &str) -> f32 {
        let current_frame = frame_controller::frame_count();
        let current_beat = self.animation.beats();

        let mut name = match self.vars.get(name) {
            Some(alias) => alias,
            None => name,
        };

        let midi_proxy_name = MapMode::proxy_name(name);
        if self.midi_proxies_enabled && self.midi_controls.has(&midi_proxy_name)
        {
            name = &midi_proxy_name;
        }

        if let Some(Some(bypass)) = self.bypassed.get(name) {
            return *bypass;
        }

        self.run_dependencies(name, current_frame);

        let value = if let Some(value) = self
            .active_transition
            .as_ref()
            .and_then(|t| self.get_transition_value(current_beat, name, t))
        {
            value
        } else {
            self.get_raw(name, current_frame)
        };

        let result = self.modulations.get(name).map_or(value, |modulators| {
            modulators.iter().fold(value, |v, modulator| {
                self.apply_modulator(v, modulator, current_frame)
            })
        });

        result
    }

    fn get_transition_value(
        &self,
        current_beat: f32,
        name: &str,
        transition: &SnapshotTransition,
    ) -> Option<f32> {
        let (from, to) = *transition.values.get(name)?;
        if current_beat < transition.start_beat {
            return None;
        }
        if current_beat >= transition.end_beat
            || transition.start_beat == transition.end_beat
        {
            return Some(to);
        }
        let duration = transition.end_beat - transition.start_beat;
        let progress = current_beat - transition.start_beat;
        let t = (progress / duration).clamp(0.0, 1.0);
        Some(lerp(from, to, t))
    }

    fn run_dependencies(&self, target_name: &str, current_frame: u32) {
        if let Some(order) = &self.dep_graph.order() {
            for name in order.iter() {
                let midi_proxy_name = MapMode::proxy_name(name);

                let name = if self.midi_proxies_enabled
                    && self.midi_controls.has(&midi_proxy_name)
                {
                    &midi_proxy_name
                } else {
                    name
                };

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

    fn apply_modulator(
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
            EffectKind::RingModulator {
                modulator: modulation_source,
                ..
            },
            Effect::RingModulator(m),
        ) = (&config.kind, &mut *effect)
        {
            let carrier = modulator;
            self.update_effect_params(&mut *m, carrier, current_frame);
            m.apply(
                value,
                self.get_raw(modulation_source.as_str(), current_frame),
            )
        } else {
            match effect {
                Effect::Constrain(m) => m.apply(value),
                Effect::Hysteresis(m) => {
                    self.update_effect_params(
                        &mut *m,
                        modulator,
                        current_frame,
                    );
                    m.apply(value)
                }
                Effect::Map(m) => m.apply(value),
                Effect::Math(m) => {
                    self.update_effect_params(
                        &mut *m,
                        modulator,
                        current_frame,
                    );
                    m.apply(value)
                }
                Effect::Quantizer(m) => {
                    self.update_effect_params(
                        &mut *m,
                        modulator,
                        current_frame,
                    );
                    m.apply(value)
                }
                Effect::Saturator(m) => {
                    self.update_effect_params(
                        &mut *m,
                        modulator,
                        current_frame,
                    );
                    m.apply(value)
                }
                Effect::SlewLimiter(m) => {
                    self.update_effect_params(
                        &mut *m,
                        modulator,
                        current_frame,
                    );
                    m.apply(value)
                }
                Effect::WaveFolder(m) => {
                    self.update_effect_params(
                        &mut *m,
                        modulator,
                        current_frame,
                    );
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
                let value = param_value.cold_or(|name: String| {
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
        let is_proxy = MapMode::is_proxy_name(name);
        let unproxied_name = &MapMode::unproxied_name(name).unwrap_or_default();

        let is_dep = self.dep_graph.is_prerequisite(if is_proxy {
            unproxied_name
        } else {
            name
        });

        if is_dep {
            if let Some(value) = self.eval_cache.get(name, current_frame) {
                return value;
            }
        }

        let value = self
            .ui_controls
            .get_optional(name)
            .or_else(|| self.midi_controls.get_optional(name))
            .or_else(|| self.audio_controls.get_optional(name))
            .or_else(|| self.osc_controls.get_optional(name))
            .or_else(|| {
                self.animations.get(name).map(|(config, sequence)| {
                    match (config, sequence) {
                        (
                            AnimationConfig::Automate(conf),
                            KeyframeSequence::Breakpoints(breakpoints),
                        ) => {
                            let breakpoints = self.resolve_breakpoint_params(
                                name,
                                &breakpoints,
                                current_frame,
                            );
                            self.animation.automate(
                                &breakpoints,
                                Mode::from_str(&conf.mode).unwrap(),
                            )
                        }
                        (
                            AnimationConfig::Ramp(conf),
                            KeyframeSequence::None,
                        ) => {
                            let conf = self.resolve_animation_config_params(
                                conf,
                                name,
                                current_frame,
                            );
                            self.animation.ramp_plus(
                                conf.beats.as_float(),
                                (conf.range[0], conf.range[1]),
                                conf.phase.as_float(),
                            )
                        }
                        (
                            AnimationConfig::Random(conf),
                            KeyframeSequence::None,
                        ) => {
                            let conf = self.resolve_animation_config_params(
                                conf,
                                name,
                                current_frame,
                            );
                            let value = self.animation.random(
                                conf.beats.as_float(),
                                (conf.range[0], conf.range[1]),
                                conf.delay.as_float(),
                                conf.stem.unwrap(),
                            );
                            apply_bias(value, conf.bias.as_float(), conf.range)
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
                            let value = self.animation.random_slewed(
                                conf.beats.as_float(),
                                (conf.range[0], conf.range[1]),
                                conf.slew.as_float(),
                                conf.delay.as_float(),
                                conf.stem.unwrap(),
                            );
                            apply_bias(value, conf.bias.as_float(), conf.range)
                        }
                        (
                            AnimationConfig::RoundRobin(conf),
                            KeyframeSequence::None,
                        ) => {
                            let conf = self.resolve_animation_config_params(
                                conf,
                                name,
                                current_frame,
                            );
                            self.animation.round_robin(
                                conf.beats.as_float(),
                                &conf.values,
                                conf.slew.as_float(),
                                conf.stem.unwrap(),
                            )
                        }
                        (
                            AnimationConfig::Triangle(conf),
                            KeyframeSequence::None,
                        ) => {
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
                        _ => unimplemented!(),
                    }
                })
            });

        match value {
            Some(value) => {
                if is_dep {
                    let name = ternary!(is_proxy, unproxied_name, name);
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
                    let value = param_value.cold_or(|name: String| {
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
                let value = param_value.cold_or(|name: String| {
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
    fn create_snapshot(
        &mut self,
        exclusions: Exclusions,
    ) -> HashMap<String, ControlValue> {
        let mut snapshot: ControlValues = ControlValues::default();

        snapshot.extend(self.ui_controls.values().iter().filter_map(
            |(name, value)| {
                if self.ui_controls.config(name).unwrap().is_separator()
                    || exclusions.contains(&name.to_string())
                {
                    None
                } else {
                    Some((name.clone(), value.clone()))
                }
            },
        ));

        snapshot.extend(self.midi_controls.values().iter().filter_map(
            |(name, value)| {
                if exclusions.contains(&name.to_string())
                    || exclusions.contains(
                        &MapMode::unproxied_name(name).unwrap_or_default(),
                    )
                {
                    None
                } else {
                    Some((name.clone(), ControlValue::from(*value)))
                }
            },
        ));

        snapshot.extend(self.osc_controls.values().iter().filter_map(
            |(name, value)| {
                if exclusions.contains(&name.to_string()) {
                    None
                } else {
                    Some((name.clone(), ControlValue::from(*value)))
                }
            },
        ));

        snapshot
    }

    /// Create and store a snapshot for later recall
    pub fn take_snapshot(&mut self, id: &str) {
        let snapshot = self.create_snapshot(Vec::new());
        self.snapshots.insert(id.to_string(), snapshot);
    }

    pub fn recall_snapshot(&mut self, id: &str) -> Result<(), String> {
        match self.snapshots.get(id) {
            Some(snapshot) => {
                let current_frame = frame_controller::frame_count();
                let current_beat = self.animation.beats();
                let transition_beats = self.transition_time.max(0.0);

                let mut transition = SnapshotTransition {
                    values: HashMap::default(),
                    start_beat: current_beat,
                    end_beat: current_beat + transition_beats,
                };

                for (name, value) in snapshot {
                    if self.ui_controls.has(name) {
                        match value {
                            ControlValue::Float(v) => {
                                let from = self.current_snapshot_value(
                                    name,
                                    current_frame,
                                    current_beat,
                                );
                                transition
                                    .values
                                    .insert(name.to_string(), (from, *v));
                            }
                            ControlValue::Bool(_) | ControlValue::String(_) => {
                                // Just update immediately since we can't
                                // interpolate over a bool and interpolating
                                // over static select options is likely to yield
                                // undesired results
                                self.ui_controls.set(name, value.clone());
                            }
                        }
                        continue;
                    }

                    if self.midi_controls.has(name)
                        || self.osc_controls.has(name)
                    {
                        let from = self.current_snapshot_value(
                            name,
                            current_frame,
                            current_beat,
                        );
                        transition.values.insert(
                            name.to_string(),
                            (from, value.as_float().unwrap()),
                        );
                        continue;
                    }
                }

                self.active_transition = Some(transition);

                info!("Snapshot \"{}\" recalled", id);
                Ok(())
            }
            None => Err(format!("No snapshot \"{}\"", id)),
        }
    }

    fn current_snapshot_value(
        &self,
        name: &str,
        current_frame: u32,
        current_beat: f32,
    ) -> f32 {
        self.active_transition
            .as_ref()
            .and_then(|transition| {
                self.get_transition_value(current_beat, name, transition)
            })
            .unwrap_or_else(|| self.get_raw(name, current_frame))
    }

    pub fn delete_snapshot(&mut self, id: &str) {
        self.snapshots.remove(id);
    }

    pub fn clear_snapshots(&mut self) {
        self.snapshots.clear()
    }

    pub fn snapshot_sequence_enabled(&self) -> bool {
        if self.snapshot_sequence.is_none() {
            return false;
        }

        self.snapshot_sequence_runtime
            .disabled
            .as_ref()
            .is_none_or(|disabled| !disabled(&self.ui_controls))
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

    #[allow(rustdoc::private_intra_doc_links)]
    /// Uses the [`Self::active_transition`] to store a temporary snapshot of
    /// randomized parameter values. See [this commit][commit] for the original
    /// frontend POC (App.tsx)
    ///
    /// [commit]: https://github.com/Lokua/xtal/commit/bcb1328
    pub fn randomize(&mut self, exclusions: Exclusions) {
        let current_frame = frame_controller::frame_count();
        let current_beat = self.animation.beats();
        let transition_beats = self.transition_time.max(0.0);

        let mut transition = SnapshotTransition {
            values: HashMap::default(),
            start_beat: current_beat,
            end_beat: current_beat + transition_beats,
        };

        for (name, value) in &self.create_snapshot(exclusions) {
            if self.ui_controls.has(name) {
                match value {
                    ControlValue::Float(_) => {
                        if let UiControlConfig::Slider {
                            min, max, step, ..
                        } = self.ui_controls.config(name).unwrap()
                        {
                            let from = self.get_raw(name, current_frame);
                            let to =
                                random_within_range_stepped(min, max, step);
                            transition
                                .values
                                .insert(name.to_string(), (from, to));
                        }
                    }
                    ControlValue::Bool(_) => {
                        // Just update immediately since we can't interpolate
                        // over a bool
                        self.ui_controls
                            .set(name, ControlValue::from(random_bool()));
                    }
                    ControlValue::String(_) => {
                        if let UiControlConfig::Select { options, .. } =
                            self.ui_controls.config(name).unwrap()
                        {
                            // Just update immediately since interpolating over
                            // static select options is likely to yield
                            // undesired results
                            let index =
                                rand::rng().random_range(0..options.len());

                            self.ui_controls.set(
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
                        rand::rng().random_range(config.min..=config.max),
                    ),
                );
            } else if self.osc_controls.has(name) {
                let config = self.osc_controls.config(name).unwrap();
                transition.values.insert(
                    name.to_string(),
                    (
                        self.get_raw(name, current_frame),
                        rand::rng().random_range(config.min..=config.max),
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
        let new_config = self.update_state.as_ref().and_then(|update_state| {
            if !update_state.has_changes.load(Ordering::Acquire) {
                return None;
            }
            update_state.has_changes.store(false, Ordering::Release);
            let state = update_state.state.lock();
            state.ok().and_then(|mut guard| guard.take())
        });

        if let Some(config) = new_config {
            if let Err(e) = self.populate_controls(&config) {
                error!("Failed to apply new configuration: {:?}", e);
            }
        }

        let sequence_disabled = self
            .snapshot_sequence_runtime
            .disabled
            .as_ref()
            .is_some_and(|disabled| disabled(&self.ui_controls));

        let current_beat = self.animation.beats();
        if self
            .active_transition
            .as_ref()
            .is_some_and(|transition| current_beat < transition.start_beat)
        {
            self.active_transition = None;
            self.snapshot_sequence_runtime.last_phase = None;
        }

        if let Some(transition) = &self.active_transition {
            if current_beat >= transition.end_beat {
                for (name, (_from, to)) in &transition.values {
                    if self.ui_controls.has(name) {
                        let value = ControlValue::Float(*to);
                        self.ui_controls.set(name, value);
                        continue;
                    } else if self.midi_controls.has(name) {
                        self.midi_controls.set(name, *to);
                        continue;
                    } else if self.osc_controls.has(name) {
                        self.osc_controls.set(name, *to);
                        continue;
                    }
                }
                self.active_transition = None;
                for callback in &self.snapshot_ended_callbacks {
                    callback.call();
                }
            }
        }

        if !sequence_disabled {
            self.update_snapshot_sequences();
        } else {
            self.snapshot_sequence_runtime.last_phase = None;
        }
    }

    fn update_snapshot_sequences(&mut self) {
        let current_beat = self.animation.beats();
        let beat_epsilon =
            (1.0 / self.animation.beats_to_frames(1.0)).max(0.000_001);

        let Some(sequence) = self.snapshot_sequence.as_ref() else {
            return;
        };

        let sequence_length = self.snapshot_sequence_runtime.sequence_length;
        if sequence_length <= 0.0 {
            self.snapshot_sequence_runtime.last_phase = None;
            return;
        }

        let phase = current_beat % sequence_length;
        let previous_phase = self.snapshot_sequence_runtime.last_phase;
        self.snapshot_sequence_runtime.last_phase = Some(phase);

        // Last stage is always kind:end (validated), so we evaluate only
        // stage entries here.
        let end = sequence.stages.len().saturating_sub(1);
        let stages = &sequence.stages[..end];

        if previous_phase.is_none() {
            for stage in stages {
                let stage_position = stage.position();
                let should_fire = Self::is_within_forward_window(
                    phase,
                    stage_position,
                    beat_epsilon,
                );

                if should_fire {
                    if let Some(stage_id) = stage.snapshot() {
                        let stage_id = stage_id.to_string();
                        if let Err(e) = self.recall_snapshot(&stage_id) {
                            warn!(
                                "snapshot_sequence stage {} failed: {}",
                                stage_id, e
                            );
                        }
                    }
                    return;
                }
            }

            return;
        }

        let previous_phase = previous_phase.unwrap_or(phase);
        for stage in stages {
            let stage_position = stage.position();
            let should_fire = Self::is_stage_crossed(
                previous_phase,
                phase,
                stage_position,
                beat_epsilon,
            );

            if should_fire {
                if let Some(stage_id) = stage.snapshot() {
                    let stage_id = stage_id.to_string();
                    if let Err(e) = self.recall_snapshot(&stage_id) {
                        warn!(
                            "snapshot_sequence stage {} failed: {}",
                            stage_id, e
                        );
                    }
                }
                return;
            }
        }
    }

    fn is_stage_crossed(
        previous_phase: f32,
        phase: f32,
        stage_position: f32,
        _beat_epsilon: f32,
    ) -> bool {
        if phase == previous_phase {
            return false;
        }

        if previous_phase <= phase {
            stage_position > previous_phase && stage_position <= phase
        } else {
            stage_position > previous_phase || stage_position <= phase
        }
    }

    fn is_within_forward_window(
        phase: f32,
        stage_position: f32,
        beat_epsilon: f32,
    ) -> bool {
        phase >= stage_position && phase < stage_position + beat_epsilon
    }

    pub fn register_populated_callback<F>(&mut self, callback: F)
    where
        F: Fn() + 'static,
    {
        self.populated_callbacks.push(Callback(Box::new(callback)));
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

    pub fn beats(&self) -> f32 {
        self.animation.beats()
    }

    pub fn var_values(&self) -> HashMap<String, f32> {
        self.vars
            .keys()
            .map(|var| (var.clone(), self.get(var)))
            .collect()
    }

    pub fn request_reload(&self) {
        if let Some(update_state) = self.update_state.as_ref() {
            info!(
                "manual control config reload requested: {}",
                update_state.path.display()
            );
            if let Ok(config) = Self::parse_from_path(&update_state.path) {
                if let Ok(mut guard) = update_state.state.lock() {
                    *guard = Some(config);
                }
            } else {
                warn!(
                    "manual control config reload failed to parse: {}",
                    update_state.path.display()
                );
            }
            update_state.has_changes.store(true, Ordering::Release);
        }
    }

    pub fn set_preserve_values_on_reload(&mut self, preserve: bool) {
        self.preserve_values_on_reload = preserve;
    }

    /// Abstracts around a common pattern where you have a checkbox, slider, and
    /// animation that are all connected as follows:
    ///
    /// ```yaml,ignore
    /// animate_radius:
    ///   type: checkbox
    ///
    /// radius:
    ///   type: slider
    ///   disabled: animate_radius
    ///
    /// radius_animation:
    ///   type: triangle
    /// ```
    ///
    /// When `animate_radius` is true, the above only results in the `radius`
    /// slider appearing disabled in the UI, but you still need to implement
    /// that on the Rust side:
    ///
    /// ```rust
    /// let radius = if self.hub.bool("animate_radius") {
    ///     self.hub.get("radius_animation")
    /// } else {
    ///     self.hub.get("radius")
    /// }
    /// ```
    ///
    /// This method just eases that boilerplate slightly:
    ///
    /// ```rust
    /// let radius = self.hub.select(
    ///     "animate_radius",
    ///     "radius_animation",
    ///     "radius"
    /// );
    /// ```
    pub fn select(
        &self,
        predicate: &str,
        name_if_true: &str,
        name_if_false: &str,
    ) -> f32 {
        ternary!(
            self.bool(predicate),
            self.get(name_if_true),
            self.get(name_if_false)
        )
    }

    fn parse_from_str(yaml_str: &str) -> Result<ConfigFile, Box<dyn Error>> {
        let raw_config = serde_yml::from_str(yaml_str)?;
        let merged_config = merge_keys_serde_yml(raw_config)?;
        let config: ConfigFile = serde_yml::from_value(merged_config)?;
        Self::validate_config_file(&config)?;
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
        let current_values: ControlValues = if self.preserve_values_on_reload {
            self.ui_controls.values().clone()
        } else {
            ControlValues::default()
        };

        let osc_values: HashMap<String, f32> = if self.preserve_values_on_reload
        {
            self.osc_controls
                .values()
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect()
        } else {
            HashMap::default()
        };

        let midi_values: HashMap<String, f32> =
            if self.preserve_values_on_reload {
                self.midi_controls
                    .values()
                    .iter()
                    .map(|(k, v)| (k.clone(), *v))
                    .collect()
            } else {
                HashMap::default()
            };

        self.ui_controls = UiControls::default();
        self.animations.clear();
        self.snapshot_sequence = None;
        self.snapshot_sequence_runtime = SnapshotSequenceRuntime::default();
        self.modulations.clear();
        self.vars.clear();
        self.bypassed.clear();
        self.dep_graph.clear();
        self.eval_cache.clear();
        self.active_transition = None;

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
                self.vars.insert(v.to_string(), id.to_string());
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

                    let slider = UiControlConfig::Slider {
                        name: id.to_string(),
                        value,
                        min: conf.range[0],
                        max: conf.range[1],
                        step: conf.step,
                        disabled,
                    };

                    self.ui_controls.add(id, slider);
                }
                ControlType::Checkbox => {
                    let mut conf: CheckboxConfig =
                        serde_yml::from_value(config.config.clone())?;

                    let value = current_values
                        .get(id)
                        .and_then(ControlValue::as_bool)
                        .unwrap_or(conf.default);

                    let disabled = Self::extract_disabled_fn(&mut conf.shared);

                    let checkbox = UiControlConfig::Checkbox {
                        name: id.to_string(),
                        value,
                        disabled,
                    };

                    self.ui_controls.add(id, checkbox);
                }
                ControlType::Select => {
                    let mut conf: SelectConfig =
                        serde_yml::from_value(config.config.clone())?;

                    let value = current_values
                        .get(id)
                        .and_then(ControlValue::as_string)
                        .unwrap_or(conf.default.as_str());

                    let disabled = Self::extract_disabled_fn(&mut conf.shared);

                    let select = UiControlConfig::Select {
                        name: id.to_string(),
                        value: value.to_string(),
                        options: conf.options,
                        disabled,
                    };

                    self.ui_controls.add(id, select);
                }
                ControlType::Separator => {
                    self.ui_controls.add(
                        id,
                        UiControlConfig::Separator {
                            name: id.to_string(),
                        },
                    );
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
                ControlType::Ramp => {
                    let conf: RampConfig =
                        serde_yml::from_value(config.config.clone())?;

                    self.animations.insert(
                        id.to_string(),
                        (AnimationConfig::Ramp(conf), KeyframeSequence::None),
                    );
                }
                ControlType::Random => {
                    let mut conf: RandomConfig =
                        serde_yml::from_value(config.config.clone())?;
                    conf.stem =
                        Some(conf.stem.unwrap_or_else(|| hash_stem(id)));

                    self.animations.insert(
                        id.to_string(),
                        (AnimationConfig::Random(conf), KeyframeSequence::None),
                    );
                }
                ControlType::RandomSlewed => {
                    let mut conf: RandomSlewedConfig =
                        serde_yml::from_value(config.config.clone())?;
                    conf.stem =
                        Some(conf.stem.unwrap_or_else(|| hash_stem(id)));

                    self.animations.insert(
                        id.to_string(),
                        (
                            AnimationConfig::RandomSlewed(conf),
                            KeyframeSequence::None,
                        ),
                    );
                }
                ControlType::RoundRobin => {
                    let mut conf: RoundRobinConfig =
                        serde_yml::from_value(config.config.clone())?;
                    conf.stem =
                        Some(conf.stem.unwrap_or_else(|| hash_stem(id)));

                    self.animations.insert(
                        id.to_string(),
                        (
                            AnimationConfig::RoundRobin(conf),
                            KeyframeSequence::None,
                        ),
                    );
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
                ControlType::SnapshotSequence => {
                    let mut conf: SnapshotSequenceConfig =
                        serde_yml::from_value(config.config.clone())?;

                    self.snapshot_sequence_runtime.disabled =
                        Self::extract_snapshot_sequence_disabled_fn(
                            &mut conf.disabled,
                        );
                    self.snapshot_sequence_runtime.sequence_length = conf
                        .stages
                        .last()
                        .map_or(0.0, |stage| stage.position());
                    self.snapshot_sequence = Some(conf);
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

    fn extract_snapshot_sequence_disabled_fn(
        disabled: &mut Option<DisabledConfig>,
    ) -> DisabledFn {
        if let Some(disabled_config) = disabled {
            disabled_config.disabled_fn.take()
        } else {
            None
        }
    }

    fn validate_snapshot_sequence_config(
        name: &str,
        conf: &SnapshotSequenceConfig,
    ) -> Result<(), Box<dyn Error>> {
        if conf.stages.len() < 2 {
            return Err(format!(
                "snapshot_sequence {} must contain at least one stage and one end",
                name
            )
            .into());
        }

        if conf.stages[0].position() != 0.0 {
            return Err(format!(
                "snapshot_sequence {} first stage must be at position 0.0",
                name
            )
            .into());
        }

        let mut previous_position = -1.0;
        for (index, stage) in conf.stages.iter().enumerate() {
            let position = stage.position();

            if !position.is_finite() || position < 0.0 {
                return Err(format!(
                    "snapshot_sequence {} stage {} has invalid position {}",
                    name, index, position
                )
                .into());
            }

            if position <= previous_position {
                return Err(format!(
                    "snapshot_sequence {} stages must have strictly increasing \
                    positions",
                    name
                )
                .into());
            }

            previous_position = position;
        }

        if !matches!(
            conf.stages.last(),
            Some(SnapshotSequenceStageConfig::End { .. })
        ) {
            return Err(format!(
                "snapshot_sequence {} must end with kind: end",
                name
            )
            .into());
        }

        if conf.stages[..conf.stages.len() - 1].iter().any(|stage| {
            !matches!(stage, SnapshotSequenceStageConfig::Stage { .. })
        }) {
            return Err(format!(
                "snapshot_sequence {} entries before the final end must be kind: stage",
                name
            )
            .into());
        }

        Ok(())
    }

    fn validate_config_file(config: &ConfigFile) -> Result<(), Box<dyn Error>> {
        let mut sequence_count = 0;

        for (id, maybe_config) in config {
            let maybe_config = match maybe_config {
                MaybeControlConfig::Control(config) => config,
                MaybeControlConfig::Other(_) => continue,
            };

            if !matches!(
                maybe_config.control_type,
                ControlType::SnapshotSequence
            ) {
                continue;
            }

            let conf: SnapshotSequenceConfig =
                serde_yml::from_value(maybe_config.config.clone())?;
            Self::validate_snapshot_sequence_config(id, &conf)?;
            sequence_count += 1;
        }

        if sequence_count > 1 {
            return Err(
                "Only one snapshot_sequence mapping is supported for now"
                    .into(),
            );
        }

        Ok(())
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
        initial_content_hash: Option<u64>,
    ) -> notify::RecommendedWatcher {
        let path_to_watch = path.clone();
        let watch_dir = path_to_watch
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let last_loaded_hash = Arc::new(Mutex::new(initial_content_hash));
        let last_change_info_log_at = Arc::new(Mutex::new(None::<Instant>));
        let last_unchanged_info_log_at = Arc::new(Mutex::new(None::<Instant>));
        info!(
            "watching control config '{}' via directory '{}'",
            path_to_watch.display(),
            watch_dir.display()
        );

        let mut watcher = notify::recommended_watcher(move |res| {
            let event: Event = match res {
                Ok(event) => event,
                Err(err) => {
                    warn!(
                        "control config watcher failed for '{}': {}",
                        path.display(),
                        err
                    );
                    return;
                }
            };

            trace!(
                "control config watcher event for '{}': {:?} {:?}",
                path.display(),
                event.kind,
                event.paths
            );

            if !config_file_changed(&event, &path) {
                return;
            }
            debug!(
                "control config fs event matched '{}': {:?}",
                path.display(),
                event.kind
            );

            let file_content = match fs::read_to_string(&path) {
                Ok(content) => content,
                Err(err) => {
                    trace!(
                        "control config change event before readable file '{}': {}",
                        path.display(),
                        err
                    );
                    return;
                }
            };

            let new_hash = content_hash(&file_content);
            if let Ok(mut guard) = last_loaded_hash.lock() {
                if guard.is_some_and(|existing_hash| existing_hash == new_hash)
                {
                    debug!(
                        "control config content unchanged; skipping reload: {}",
                        path.display()
                    );
                    let should_log_info = if let Ok(mut guard) =
                        last_unchanged_info_log_at.lock()
                    {
                        let now = Instant::now();
                        let suppressed = guard.is_some_and(|last| {
                            now.duration_since(last)
                                < WATCHER_CHANGE_INFO_DEBOUNCE
                        });
                        if !suppressed {
                            *guard = Some(now);
                        }
                        !suppressed
                    } else {
                        true
                    };
                    if should_log_info {
                        info!(
                            "control config unchanged; skipped reload: {}",
                            path.display()
                        );
                    }
                    return;
                }
                *guard = Some(new_hash);
            }

            match Self::parse_from_str(&file_content) {
                Ok(new_config) => {
                    if let Ok(mut guard) = state.lock() {
                        *guard = Some(new_config);
                        let already_pending =
                            has_changes.swap(true, Ordering::AcqRel);

                        if already_pending {
                            debug!(
                                "loaded new control configuration while pending: {}",
                                path.display()
                            );
                            return;
                        }

                        let should_log_info = if let Ok(mut guard) =
                            last_change_info_log_at.lock()
                        {
                            let now = Instant::now();
                            let suppressed = guard.is_some_and(|last| {
                                now.duration_since(last)
                                    < WATCHER_CHANGE_INFO_DEBOUNCE
                            });
                            if !suppressed {
                                *guard = Some(now);
                            }
                            !suppressed
                        } else {
                            true
                        };

                        if should_log_info {
                            info!(
                                "control config changed: {}",
                                path.display()
                            );
                        } else {
                            debug!(
                                "control config change suppressed by debounce: {}",
                                path.display()
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "failed to parse updated control config '{}': {:?}",
                        path.display(),
                        e
                    );
                }
            }
        })
        .expect("Failed to create watcher");

        watcher
            .watch(&watch_dir, RecursiveMode::NonRecursive)
            .expect("Failed to start watching file");

        watcher
    }
}

fn config_file_changed(event: &Event, target: &Path) -> bool {
    if !matches!(
        event.kind,
        notify::EventKind::Create(_)
            | notify::EventKind::Modify(_)
            | notify::EventKind::Remove(_)
    ) {
        return false;
    }

    if event.paths.is_empty() {
        return true;
    }

    event
        .paths
        .iter()
        .any(|path| path_matches_target(path, target))
}

fn path_matches_target(path: &Path, target: &Path) -> bool {
    if path == target {
        return true;
    }

    if path.file_name() == target.file_name() {
        return true;
    }

    let path_canon = path.canonicalize().ok();
    let target_canon = target.canonicalize().ok();

    match (path_canon, target_canon) {
        (Some(path_canon), Some(target_canon)) => path_canon == target_canon,
        _ => false,
    }
}

/// Produce a deterministic `u64` from a mapping name, used as the default
/// stem when the user omits `stem` from a YAML mapping. The hash is stable
/// across runs for the same name.
fn hash_stem(name: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    hasher.finish()
}

fn content_hash(content: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

fn apply_bias(value: f32, bias: f32, range: [f32; 2]) -> f32 {
    if bias == 0.0 {
        return value;
    }
    let min = range[0];
    let max = range[1];
    if min == max {
        return value;
    }
    let t = (value - min) / (max - min);
    let curved = curve(t, bias, SUGGESTED_CURVE_MAX_EXPONENT);
    min + curved * (max - min)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::motion::animation::animation_tests::{BPM, init};
    use serial_test::serial;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn create_instance(yaml: &str) -> ControlHub<FrameTiming> {
        ControlHub::new(Some(yaml), FrameTiming::new(Bpm::new(BPM)))
    }

    fn assert_close(actual: f32, expected: f32, label: &str) {
        let epsilon = 0.000_1;
        assert!(
            (actual - expected).abs() <= epsilon,
            "{}: expected {}, got {}",
            label,
            expected,
            actual
        );
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

        init(0.0);
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

        init(1.5);
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

        init(0.0);
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

        controls.ui_controls.set("a", ControlValue::Float(100.0));
        controls.midi_controls.set("b", 200.0);
        controls.osc_controls.set("c", 300.0);
        controls.take_snapshot("bar");

        init(0.0);
        controls.recall_snapshot("bar").unwrap();
        controls.update();
        assert_eq!(controls.get("a"), 100.0);
        assert_eq!(controls.get("b"), 200.0);
        assert_eq!(controls.get("c"), 300.0);

        init(0.25);
        controls.update();
        controls.recall_snapshot("foo").unwrap();
        assert_eq!(controls.get("a"), 10.0);
        assert_eq!(controls.get("b"), 20.0);
        assert_eq!(controls.get("c"), 30.0);
    }

    #[test]
    #[serial]
    fn test_snapshot_recall_interpolates_and_lands_on_saved_values() {
        let mut controls = create_instance(
            r#"
x:
  type: slider
  default: 0
y:
  type: slider
  default: 10
"#,
        );

        controls.set_transition_time(4.0);

        controls.take_snapshot("a");
        controls.ui_controls.set("x", ControlValue::Float(100.0));
        controls.ui_controls.set("y", ControlValue::Float(90.0));
        controls.take_snapshot("b");

        controls.ui_controls.set("x", ControlValue::Float(0.0));
        controls.ui_controls.set("y", ControlValue::Float(10.0));

        init(0.0);
        controls.recall_snapshot("b").unwrap();

        let transition = controls.active_transition.as_ref().unwrap();
        let (x_from, x_to) = transition.values["x"];
        let (y_from, y_to) = transition.values["y"];

        assert_close(controls.get("x"), x_from, "x at transition start");
        assert_close(controls.get("y"), y_from, "y at transition start");

        init(2.0);
        assert_close(
            controls.get("x"),
            lerp(x_from, x_to, 0.5),
            "x at transition midpoint",
        );
        assert_close(
            controls.get("y"),
            lerp(y_from, y_to, 0.5),
            "y at transition midpoint",
        );

        init(4.1);
        controls.update();
        assert_close(controls.get("x"), x_to, "x at transition end");
        assert_close(controls.get("y"), y_to, "y at transition end");
    }

    #[test]
    #[serial]
    fn test_randomize_all_transitions_and_lands_on_end_values() {
        let mut controls = create_instance(
            r#"
x:
  type: slider
  min: 0
  max: 100
  step: 1
  default: 20
y:
  type: slider
  min: 0
  max: 100
  step: 1
  default: 80
"#,
        );

        controls.set_transition_time(2.0);
        init(0.0);
        controls.randomize(vec![]);

        let transition = controls.active_transition.as_ref().unwrap();
        assert!(transition.values.contains_key("x"));
        assert!(transition.values.contains_key("y"));
        let (x_from, x_to) = transition.values["x"];
        let (y_from, y_to) = transition.values["y"];

        assert_close(controls.get("x"), x_from, "x randomize start");
        assert_close(controls.get("y"), y_from, "y randomize start");

        init(1.0);
        assert_close(
            controls.get("x"),
            lerp(x_from, x_to, 0.5),
            "x randomize midpoint",
        );
        assert_close(
            controls.get("y"),
            lerp(y_from, y_to, 0.5),
            "y randomize midpoint",
        );

        init(2.1);
        controls.update();
        assert_close(controls.get("x"), x_to, "x randomize end");
        assert_close(controls.get("y"), y_to, "y randomize end");
    }

    #[test]
    #[serial]
    fn test_randomize_single_respects_exclusions() {
        let mut controls = create_instance(
            r#"
x:
  type: slider
  min: 0
  max: 100
  step: 1
  default: 20
y:
  type: slider
  min: 0
  max: 100
  step: 1
  default: 80
"#,
        );

        controls.set_transition_time(2.0);
        let y_before = controls.get("y");

        init(0.0);
        controls.randomize(vec!["y".into()]);

        let transition = controls.active_transition.as_ref().unwrap();
        assert!(transition.values.contains_key("x"));
        assert!(!transition.values.contains_key("y"));

        init(1.0);
        assert_close(controls.get("y"), y_before, "y midpoint excluded");

        init(2.1);
        controls.update();
        assert_close(controls.get("y"), y_before, "y end excluded");
    }

    #[test]
    #[serial]
    fn test_exclusions_apply_consistently_to_snapshot_and_randomize() {
        let mut controls = create_instance(
            r#"
x:
  type: slider
  min: 0
  max: 1
  default: 0.2
y:
  type: slider
  min: 0
  max: 1
  default: 0.8
"#,
        );

        let snapshot = controls.create_snapshot(vec!["y".into()]);
        assert!(snapshot.contains_key("x"));
        assert!(!snapshot.contains_key("y"));

        controls.randomize(vec!["y".into()]);
        let transition = controls.active_transition.as_ref().unwrap();
        assert!(transition.values.contains_key("x"));
        assert!(!transition.values.contains_key("y"));
    }

    #[test]
    #[serial]
    fn test_populated_callback_emits_once_per_population() {
        let yaml = r#"
x:
  type: slider
  default: 0.5
"#;

        let mut controls = create_instance(yaml);
        let populated_count = Arc::new(AtomicUsize::new(0));
        let populated_count_clone = populated_count.clone();
        controls.register_populated_callback(move || {
            populated_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        let config = ControlHub::<FrameTiming>::parse_from_str(yaml).unwrap();
        controls.populate_controls(&config).unwrap();
        assert_eq!(populated_count.load(Ordering::SeqCst), 1);

        controls.ui_controls.set("x", ControlValue::Float(0.75));
        controls.update();
        assert_eq!(populated_count.load(Ordering::SeqCst), 1);
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

        assert!(hub.ui_controls.disabled("foo"));
    }

    #[test]
    #[serial]
    fn test_proxied_pmod_bug() {
        let mut hub = create_instance(
            r#"
foo: 
  type: slider 

foo_animation:
  type: automate 
  breakpoints:
    - position: 0
      value: $foo
      kind: step 
            "#,
        );

        hub.midi_controls.add(
            &MapMode::proxy_name("foo"),
            MidiControlConfig {
                channel: 0,
                cc: 0,
                min: 0.0,
                max: 100.0,
                value: 99.0,
            },
        );

        init(0.25);
        assert_eq!(hub.get("foo_animation"), 99.0);
    }

    fn create_snapshot_sequence_hub(
        sequence_yaml: &str,
    ) -> ControlHub<FrameTiming> {
        let yaml = format!(
            r#"
a:
  type: slider
  default: 0

{}
"#,
            sequence_yaml
        );

        let mut hub = create_instance(&yaml);
        hub.set_transition_time(0.0);

        hub.take_snapshot("1");
        hub.ui_controls.set("a", ControlValue::Float(1.0));
        hub.take_snapshot("2");
        hub.ui_controls.set("a", ControlValue::Float(-10.0));
        hub
    }

    #[test]
    #[serial]
    fn test_snapshot_sequence_loop_scheduling() {
        let mut hub = create_snapshot_sequence_hub(
            r#"
sequence:
  type: snapshot_sequence
  stages:
    - kind: stage
      snapshot: 1
      position: 0.0
    - kind: stage
      snapshot: 2
      position: 0.5
    - kind: end
      position: 1.0
"#,
        );

        init(0.0);
        hub.update();
        assert_eq!(hub.get("a"), 0.0, "stage 1 at beat 0.0");

        init(0.5);
        hub.update();
        assert_eq!(hub.get("a"), 1.0, "stage 2 at beat 0.5");

        init(1.25);
        hub.update();
        assert_eq!(hub.get("a"), 0.0, "wrapped stage 1");
    }

    #[test]
    #[serial]
    fn test_snapshot_sequence_invalid_positions() {
        let result = ControlHub::<FrameTiming>::parse_from_str(
            r#"
sequence:
  type: snapshot_sequence
  stages:
    - kind: stage
      snapshot: 1
      position: 0.5
    - kind: end
      position: 0.25
"#,
        );

        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_snapshot_sequence_forward_window_avoids_duplicate_fires() {
        let epsilon = 0.25;
        let stage = 2.0;

        assert!(ControlHub::<FrameTiming>::is_within_forward_window(
            2.0, stage, epsilon
        ));
        assert!(!ControlHub::<FrameTiming>::is_within_forward_window(
            1.75, stage, epsilon
        ));
        assert!(!ControlHub::<FrameTiming>::is_within_forward_window(
            2.25, stage, epsilon
        ));
    }

    #[test]
    #[serial]
    fn test_snapshot_sequence_stage_crossing_detects_non_wrap_and_wrap() {
        let epsilon = 0.001;

        assert!(ControlHub::<FrameTiming>::is_stage_crossed(
            1.0, 2.1, 2.0, epsilon
        ));
        assert!(!ControlHub::<FrameTiming>::is_stage_crossed(
            1.0, 1.9, 2.0, epsilon
        ));

        assert!(ControlHub::<FrameTiming>::is_stage_crossed(
            3.9, 0.2, 0.1, epsilon
        ));
        assert!(!ControlHub::<FrameTiming>::is_stage_crossed(
            3.9, 0.2, 2.0, epsilon
        ));
    }

    #[test]
    #[serial]
    fn test_update_clears_stale_transition_after_frame_reset() {
        let mut hub = create_instance(
            r#"
a:
  type: slider
  default: 0
"#,
        );

        let mut values = HashMap::default();
        values.insert("a".to_string(), (0.0, 1.0));
        hub.active_transition = Some(SnapshotTransition {
            values,
            start_beat: 10.0,
            end_beat: 12.0,
        });

        init(0.0);
        hub.update();

        assert!(hub.active_transition.is_none());
    }

    #[test]
    #[serial]
    fn test_auto_stem_deterministic_and_unique() {
        // Two different names produce different stems
        let hub = create_instance(
            r#"
a:
  type: random
  beats: 2
  range: [0, 100]

b:
  type: random
  beats: 2
  range: [0, 100]
            "#,
        );

        let stem_a = match hub.animations.get("a").unwrap().0 {
            AnimationConfig::Random(ref conf) => conf.stem,
            _ => panic!("Expected Random"),
        };
        let stem_b = match hub.animations.get("b").unwrap().0 {
            AnimationConfig::Random(ref conf) => conf.stem,
            _ => panic!("Expected Random"),
        };

        assert!(stem_a.is_some(), "stem should be resolved to Some");
        assert!(stem_b.is_some(), "stem should be resolved to Some");
        assert_ne!(
            stem_a, stem_b,
            "different names must produce different stems"
        );

        // Same name always produces the same stem (idempotent)
        let hub2 = create_instance(
            r#"
a:
  type: random
  beats: 2
  range: [0, 100]
            "#,
        );
        let stem_a2 = match hub2.animations.get("a").unwrap().0 {
            AnimationConfig::Random(ref conf) => conf.stem,
            _ => panic!("Expected Random"),
        };
        assert_eq!(stem_a, stem_a2, "same name must produce same stem");
    }

    #[test]
    #[serial]
    fn test_explicit_stem_preserved() {
        let hub = create_instance(
            r#"
a:
  type: random_slewed
  beats: 2
  range: [0, 100]
  slew: 0.5
  stem: 42

b:
  type: round_robin
  values: [0.0, 0.5, 1.0]
  beats: 2
  stem: 999
            "#,
        );

        let stem_a = match hub.animations.get("a").unwrap().0 {
            AnimationConfig::RandomSlewed(ref conf) => conf.stem,
            _ => panic!("Expected RandomSlewed"),
        };
        let stem_b = match hub.animations.get("b").unwrap().0 {
            AnimationConfig::RoundRobin(ref conf) => conf.stem,
            _ => panic!("Expected RoundRobin"),
        };

        assert_eq!(stem_a, Some(42), "explicit stem must be preserved");
        assert_eq!(stem_b, Some(999), "explicit stem must be preserved");
    }

    #[test]
    #[serial]
    fn test_snapshot_sequence_invalid_reload_keeps_current_state() {
        let hub_yaml = r#"
sequence:
  type: snapshot_sequence
  beats: 4
  snapshots: [1, 2]
"#;

        let hub = create_snapshot_sequence_hub(hub_yaml);
        let initial_length = hub.snapshot_sequence_runtime.sequence_length;
        let initial_stages = hub
            .snapshot_sequence
            .as_ref()
            .map(|sequence| sequence.stages.len());

        let invalid = ControlHub::<FrameTiming>::parse_from_str(
            r#"
sequence:
  type: snapshot_sequence
  beats: 4
  snapshots: [1, 2]
  stages:
    - kind: stage
      snapshot: 1
      position: 0.0
    - kind: end
      position: 8.0
"#,
        );

        assert!(invalid.is_err());
        assert_eq!(
            hub.snapshot_sequence_runtime.sequence_length,
            initial_length
        );
        assert_eq!(
            hub.snapshot_sequence
                .as_ref()
                .map(|sequence| sequence.stages.len()),
            initial_stages
        );
    }
}
