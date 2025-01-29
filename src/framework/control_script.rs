use serde::Deserialize;
use std::{collections::HashMap, error::Error, fs, path::PathBuf};

use super::prelude::*;

#[derive(Deserialize, Debug)]
enum ControlType {
    #[serde(rename = "slider")]
    Slider,
}

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

type ConfigFile = HashMap<String, MaybeControlConfig>;

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

pub struct ControlScript {
    control_configs: Option<ConfigFile>,
    pub controls: Controls,
    // _watcher: notify::RecommendedWatcher,
}

impl ControlScript {
    pub fn new(path: PathBuf) -> Self {
        let mut script = Self {
            control_configs: None,
            controls: Controls::with_previous(vec![]),
        };
        script
            .import_script(&path)
            .expect("Unable to import script");
        script
    }

    fn import_script(&mut self, path: &PathBuf) -> Result<(), Box<dyn Error>> {
        let file_content = fs::read_to_string(&path)?;
        let control_configs: ConfigFile = serde_yml::from_str(&file_content)
            .expect("Unable to parse yaml file");
        trace!("control_configs: {:?}", control_configs);
        self.control_configs = Some(control_configs);
        self.populate_controls()
            .expect("Unable to populate controls");
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
                    let slider_config: SliderConfig =
                        serde_yml::from_value(config.config.clone())?;
                    let slider = Control::slider(
                        id.as_str(),
                        slider_config.default,
                        (slider_config.range[0], slider_config.range[1]),
                        slider_config.step,
                    );
                    self.controls.add(slider);
                }
            }
        }

        Ok(())
    }

    pub fn get(&self, name: &str) -> f32 {
        self.controls.float(name)
    }

    pub fn update(&self) {
        // debug!("update called");
    }
}
