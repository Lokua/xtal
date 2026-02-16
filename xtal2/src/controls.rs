use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde_yml::Value;
use yaml_merge_keys::merge_keys_serde_yml;

#[derive(Clone, Debug)]
pub struct ControlDefaults {
    values: Vec<ControlValue>,
}

#[derive(Clone, Debug)]
pub struct ControlValue {
    pub id: String,
    pub value: f32,
}

pub struct ControlScriptWatcher {
    changed: Arc<AtomicBool>,
    _watcher: RecommendedWatcher,
}

impl ControlDefaults {
    pub fn load(path: &Path) -> Result<Self, String> {
        let source = fs::read_to_string(path).map_err(|err| {
            format!(
                "failed to read control script '{}': {}",
                path.display(),
                err
            )
        })?;

        let raw: Value = serde_yml::from_str(&source).map_err(|err| {
            format!(
                "failed to parse control script '{}': {}",
                path.display(),
                err
            )
        })?;

        let merged = merge_keys_serde_yml(raw).map_err(|err| {
            format!(
                "failed to process YAML merge keys in '{}': {}",
                path.display(),
                err
            )
        })?;

        Self::from_value(merged).map_err(|err| {
            format!(
                "failed to decode control defaults in '{}': {}",
                path.display(),
                err
            )
        })
    }

    pub fn values(&self) -> &[ControlValue] {
        &self.values
    }

    fn from_value(value: Value) -> Result<Self, String> {
        let mapping = value
            .as_mapping()
            .ok_or_else(|| "top-level YAML must be a mapping".to_string())?;

        let mut values = Vec::new();

        for (key, control_value) in mapping {
            let Some(name) = key.as_str() else {
                continue;
            };

            let Some(control_map) = control_value.as_mapping() else {
                continue;
            };

            let Some(control_type) =
                get_string(control_map, "type").map(str::to_lowercase)
            else {
                continue;
            };

            let id = get_string(control_map, "var")
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| name.to_string());

            let default = extract_default(control_map, &control_type)?;
            let Some(value) = default else {
                continue;
            };

            values.push(ControlValue { id, value });
        }

        Ok(Self { values })
    }
}

impl ControlScriptWatcher {
    pub fn start(path: PathBuf) -> Result<Self, notify::Error> {
        let changed = Arc::new(AtomicBool::new(false));
        let changed_flag = changed.clone();
        let watched_path = path.clone();

        let mut watcher = notify::recommended_watcher(move |result| {
            let Ok(event) = result else {
                return;
            };

            if control_script_changed(&event, &watched_path) {
                changed_flag.store(true, Ordering::SeqCst);
            }
        })?;

        watcher.watch(&path, RecursiveMode::NonRecursive)?;

        Ok(Self {
            changed,
            _watcher: watcher,
        })
    }

    pub fn take_changed(&self) -> bool {
        self.changed.swap(false, Ordering::SeqCst)
    }
}

pub fn resolve_control_script_path(
    path: impl Into<PathBuf>,
) -> Result<PathBuf, String> {
    let path = path.into();

    if path.is_absolute() {
        return Ok(path);
    }

    let cwd = std::env::current_dir()
        .map_err(|err| format!("failed to get current directory: {}", err))?;

    Ok(cwd.join(path))
}

fn extract_default(
    control_map: &serde_yml::Mapping,
    control_type: &str,
) -> Result<Option<f32>, String> {
    let bypass = get_f32(control_map, "bypass");

    if let Some(value) = bypass {
        return Ok(Some(value));
    }

    match control_type {
        "slider" => Ok(Some(get_f32(control_map, "default").unwrap_or(0.0))),
        "checkbox" => {
            Ok(Some(if get_bool(control_map, "default").unwrap_or(false) {
                1.0
            } else {
                0.0
            }))
        }
        "select" => select_default_index(control_map).map(Some),
        "midi" => Ok(Some(get_f32(control_map, "default").unwrap_or(0.0))),
        "osc" => Ok(Some(get_f32(control_map, "default").unwrap_or(0.0))),
        "separator" => Ok(None),
        _ => Ok(None),
    }
}

fn select_default_index(
    control_map: &serde_yml::Mapping,
) -> Result<f32, String> {
    let options = control_map
        .get(Value::String("options".to_string()))
        .and_then(Value::as_sequence)
        .ok_or_else(|| {
            "select control missing 'options' sequence".to_string()
        })?;

    let default = get_string(control_map, "default")
        .ok_or_else(|| "select control missing 'default' string".to_string())?;

    for (index, option) in options.iter().enumerate() {
        if option.as_str() == Some(default) {
            return Ok(index as f32);
        }
    }

    Err(format!("select default '{}' not found in options", default))
}

fn get_string<'a>(
    mapping: &'a serde_yml::Mapping,
    key: &str,
) -> Option<&'a str> {
    mapping
        .get(Value::String(key.to_string()))
        .and_then(Value::as_str)
}

fn get_bool(mapping: &serde_yml::Mapping, key: &str) -> Option<bool> {
    mapping
        .get(Value::String(key.to_string()))
        .and_then(Value::as_bool)
}

fn get_f32(mapping: &serde_yml::Mapping, key: &str) -> Option<f32> {
    match mapping.get(Value::String(key.to_string())) {
        Some(Value::Number(number)) => number.as_f64().map(|v| v as f32),
        Some(Value::String(text)) => text.parse::<f32>().ok(),
        _ => None,
    }
}

fn control_script_changed(event: &Event, watched_path: &PathBuf) -> bool {
    if !matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    ) {
        return false;
    }

    if event.paths.is_empty() {
        return true;
    }

    event
        .paths
        .iter()
        .any(|path| path_matches_target(path, watched_path))
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
