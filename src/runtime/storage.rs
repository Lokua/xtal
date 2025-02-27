use std::error::Error;
use std::path::PathBuf;
use std::{fs, str};

use super::prelude::*;
use crate::framework::prelude::*;

const STORE_CONTROLS_CACHE_IN_PROJECT: bool = true;

pub fn stored_controls(sketch_name: &str) -> Option<ControlValues> {
    let path = controls_storage_path(sketch_name)?;
    let bytes = fs::read(path).ok()?;
    let string = str::from_utf8(&bytes).ok()?;
    let serialized = serde_json::from_str::<SerializedControls>(string).ok()?;
    Some(serialized.values)
}

pub fn persist_controls(
    sketch_name: &str,
    controls: &Controls,
) -> Result<PathBuf, Box<dyn Error>> {
    let path = controls_storage_path(sketch_name)
        .ok_or("Could not determine the configuration directory")?;
    if let Some(parent_dir) = path.parent() {
        fs::create_dir_all(parent_dir)?;
    }
    let serialized = controls.to_serialized();
    let json = serde_json::to_string_pretty(&serialized)?;
    fs::write(&path, json)?;
    Ok(path)
}

pub fn delete_stored_controls(sketch_name: &str) -> Result<(), Box<dyn Error>> {
    let path = controls_storage_path(sketch_name)
        .ok_or("Could not determine the configuration directory")?;
    if path.exists() {
        fs::remove_file(path)?;
        info!("Deleted controls for sketch: {}", sketch_name);
    } else {
        warn!("No stored controls found for sketch: {}", sketch_name);
    }
    Ok(())
}

fn controls_storage_path(sketch_name: &str) -> Option<PathBuf> {
    if STORE_CONTROLS_CACHE_IN_PROJECT {
        return Some(
            lattice_project_root()
                .join("control-cache")
                .join(format!("{}_controls.json", sketch_name)),
        );
    }

    lattice_config_dir().map(|config_dir| {
        config_dir
            .join("Controls")
            .join(format!("{}_controls.json", sketch_name))
    })
}
