use notify::{Event, RecursiveMode, Watcher};
use std::{
    collections::HashMap,
    error::Error,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use toml;

use super::prelude::*;

type Config = HashMap<String, Vec<(String, f32)>>;

#[derive(Debug)]
struct ParsedKeyframe {
    beats: f32,
    value: f32,
}

struct UpdateState {
    _watcher: notify::RecommendedWatcher,
    state: Arc<Mutex<Option<(Config, HashMap<String, Vec<Keyframe>>)>>>,
}

pub struct AnimationScript<T: TimingSource> {
    pub animation: Animation<T>,
    #[allow(dead_code)]
    path: PathBuf,
    config: Config,
    keyframe_sequences: HashMap<String, Vec<Keyframe>>,
    update_state: UpdateState,
}

impl<T: TimingSource> AnimationScript<T> {
    pub fn new(path: PathBuf, animation: Animation<T>) -> Self {
        let state = Arc::new(Mutex::new(None));
        let state_clone = state.clone();

        let config =
            Self::import_script(&path).expect("Unable to import script");

        let mut script = Self {
            animation,
            config: config.clone(),
            path: path.clone(),
            keyframe_sequences: HashMap::new(),
            update_state: UpdateState {
                state: state.clone(),
                _watcher: Self::setup_watcher(path, state_clone),
            },
        };

        script.precompute_keyframes();

        *script.update_state.state.lock().unwrap() =
            Some((config, script.keyframe_sequences.clone()));

        script
    }

    fn import_script(path: &PathBuf) -> Result<Config, Box<dyn Error>> {
        let file_content = fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&file_content)?;
        trace!("{:?}", config);
        Ok(config)
    }

    fn precompute_keyframes(&mut self) {
        self.keyframe_sequences.clear();

        for (param, time_values) in &self.config {
            let mut parsed_keyframes = Vec::new();

            for (time_str, value) in time_values {
                if let Ok(beats) = parse_bar_beat_16th(time_str) {
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

            self.keyframe_sequences.insert(param.clone(), keyframes);
        }

        trace!("keyframe_sequences: {:?}", self.keyframe_sequences);
    }

    pub fn get(&self, param: &str) -> f32 {
        if let Some(keyframes) = self.keyframe_sequences.get(param) {
            self.animation.lerp(keyframes.clone(), 0.0)
        } else {
            warn!("No param named {}. Falling back to 0.0", param);
            0.0
        }
    }

    fn setup_watcher(
        path: PathBuf,
        state: Arc<Mutex<Option<(Config, HashMap<String, Vec<Keyframe>>)>>>,
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

            info!(
                "{:?} changed. Attempting to rebuild keyframe sequences.",
                path
            );

            let new_config = match Self::import_script(&path) {
                Ok(config) => config,
                Err(_) => return,
            };

            let mut keyframe_sequences = HashMap::new();
            for (param, time_values) in &new_config {
                let mut keyframes = Vec::new();
                let parsed_keyframes: Vec<_> = time_values
                    .iter()
                    .filter_map(|(time_str, value)| {
                        parse_bar_beat_16th(time_str).ok().map(|beats| {
                            ParsedKeyframe {
                                beats,
                                value: *value,
                            }
                        })
                    })
                    .collect();

                for (i, current) in parsed_keyframes.iter().enumerate() {
                    let duration = if i < parsed_keyframes.len() - 1 {
                        parsed_keyframes[i + 1].beats - current.beats
                    } else {
                        0.0
                    };
                    keyframes.push(Keyframe::new(current.value, duration));
                }

                keyframe_sequences.insert(param.clone(), keyframes);
            }

            if let Ok(mut guard) = state.lock() {
                info!(
                    "Created new keyframe sequences; call `update` to refresh."
                );
                debug!("keyframes: {:?}", keyframe_sequences);
                *guard = Some((new_config, keyframe_sequences));
            }
        })
        .expect("Failed to create watcher");

        watcher
            .watch(&path_to_watch, RecursiveMode::NonRecursive)
            .expect("Failed to start watching file");

        watcher
    }

    pub fn update(&mut self) {
        if let Ok(mut guard) = self.update_state.state.lock() {
            // take() sets to None
            if let Some((new_config, new_keyframes)) = guard.take() {
                info!("Swapping in new keyframe sequences.");
                self.config = new_config;
                self.keyframe_sequences = new_keyframes;
            }
        }
    }
}

fn parse_bar_beat_16th(time_str: &str) -> Result<f32, Box<dyn Error>> {
    let parts: Vec<f32> = time_str
        .split('.')
        .map(|s| s.parse::<f32>())
        .collect::<Result<Vec<f32>, _>>()?;

    if parts.len() != 3 {
        return Err("Time string must be in format bar.beat.16th".into());
    }

    let [bars, beats, sixteenths] = [parts[0], parts[1], parts[2]];
    let total_beats = (bars * 4.0) + beats + (sixteenths * 0.25);

    Ok(total_beats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framework::animation::tests::init;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_animation_script_interpolation() {
        init(0);
        let animation = Animation::new(FrameTiming::new(360.0));
        let script = AnimationScript::new(
            to_absolute_path(file!(), "./animation_script.toml"),
            animation,
        );

        let radius = script.get("radius");
        assert_eq!(radius, 0.0, "Should start at initial radius");

        init(32);
        let radius = script.get("radius");
        assert!(
            (radius - 250.0).abs() < 0.01,
            "Radius should be approximately 250.0 at halfway point"
        );

        let hue = script.get("hue");
        assert!(
            (hue - 0.5).abs() < 0.01,
            "Hue should be approximately 0.5 at halfway point"
        );
    }

    #[test]
    #[serial]
    fn test_bar_beat_16th_conversion() {
        let test_cases = vec![
            // bars.beats.16ths, beats
            ("0.0.0", 0.0),
            ("1.0.0", 4.0),
            ("0.1.0", 1.0),
            ("0.0.2", 0.5),
            ("2.2.2", 10.5),
        ];

        for (input, expected) in test_cases {
            let result = parse_bar_beat_16th(input)
                .expect("Failed to parse time string");
            assert!(
                (result - expected).abs() < 0.001,
                "Converting {} should yield {} but got {}",
                input,
                expected,
                result
            );
        }
    }
}
