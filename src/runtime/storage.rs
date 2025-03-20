use std::error::Error;
use std::path::PathBuf;
use std::{fs, str};

use serde::{Deserialize, Serialize};

use super::serialization::{SaveableProgramState, SerializableProgramState};
use super::shared::{lattice_config_dir, lattice_project_root};
use crate::framework::prelude::*;

/// When false will use the appropriate OS config dir; when true will store
/// within the Lattice project's controls_cache folder for easy source control.
const STORE_CONTROLS_CACHE_IN_PROJECT: bool = true;

fn controls_storage_path(sketch_name: &str) -> Option<PathBuf> {
    if STORE_CONTROLS_CACHE_IN_PROJECT {
        return Some(
            lattice_project_root()
                .join("storage")
                .join(format!("{}_controls.json", sketch_name)),
        );
    }

    lattice_config_dir().map(|config_dir| {
        config_dir
            .join("Controls")
            .join(format!("{}_controls.json", sketch_name))
    })
}

pub fn save_program_state<T: TimingSource + std::fmt::Debug + 'static>(
    sketch_name: &str,
    hrcc: bool,
    hub: &ControlHub<T>,
) -> Result<PathBuf, Box<dyn Error>> {
    let concrete_controls = SaveableProgramState {
        hrcc,
        ui_controls: hub.ui_controls.clone(),
        midi_controls: hub.midi_controls.clone(),
        osc_controls: hub.osc_controls.clone(),
        snapshots: hub.snapshots.clone(),
    };

    let serializable_controls =
        SerializableProgramState::from(&concrete_controls);
    let json = serde_json::to_string_pretty(&serializable_controls)?;
    let path = controls_storage_path(sketch_name)
        .ok_or("Could not determine the configuration directory")?;
    if let Some(parent_dir) = path.parent() {
        fs::create_dir_all(parent_dir)?;
    }
    fs::write(&path, json)?;
    Ok(path)
}

pub fn load_program_state<T: TimingSource + std::fmt::Debug + 'static>(
    sketch_name: &str,
    // hrcc: &mut bool,
    hub: &mut ControlHub<T>,
) -> Result<(), Box<dyn Error>> {
    let path = controls_storage_path(sketch_name)
        .ok_or("Could not determine controls cache directory")?;
    let bytes = fs::read(path)?;
    let json = str::from_utf8(&bytes).ok().map(|s| s.to_owned()).unwrap();

    let serialized = serde_json::from_str::<SerializableProgramState>(&json)?;

    let mut state = SaveableProgramState {
        // Temporary value...
        hrcc: false,
        ui_controls: hub.ui_controls.clone(),
        midi_controls: hub.midi_controls.clone(),
        osc_controls: hub.osc_controls.clone(),
        snapshots: hub.snapshots.clone(),
    };

    state.merge(serialized);

    // *hrcc = state.hrcc;

    state.ui_controls.values().iter().for_each(|(name, value)| {
        hub.ui_controls.update_value(name, value.clone());
    });

    state
        .midi_controls
        .values()
        .iter()
        .for_each(|(name, value)| {
            hub.midi_controls.update_value(name, *value);
        });

    state
        .osc_controls
        .values()
        .iter()
        .for_each(|(name, value)| {
            hub.osc_controls.update_value(name, *value);
        });

    for (name, snapshot) in state.snapshots {
        hub.snapshots.insert(name, snapshot);
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// Image Index
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageIndex {
    pub items: Vec<ImageIndexItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageIndexItem {
    pub filename: String,
    pub created_at: String,
}

fn image_index_path() -> PathBuf {
    lattice_project_root().join("images").join("_index.json")
}

pub fn load_image_index() -> Result<ImageIndex, Box<dyn Error>> {
    let bytes = fs::read(image_index_path())?;
    let json = str::from_utf8(&bytes).ok().map(|s| s.to_owned()).unwrap();
    let image_index_file: ImageIndex = serde_json::from_str(&json)?;
    Ok(image_index_file)
}

pub fn save_image_index(
    image_index: &ImageIndex,
) -> Result<(), Box<dyn Error>> {
    let json = serde_json::to_string_pretty(image_index)?;
    fs::write(image_index_path(), json)?;
    Ok(())
}
