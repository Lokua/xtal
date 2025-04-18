use std::error::Error;
use std::path::PathBuf;
use std::{fs, str};

use serde::{Deserialize, Serialize};

use super::map_mode::Mappings;
use super::serialization::{
    GlobalSettings, SerializableSketchState, TransitorySketchState,
};
use crate::framework::prelude::*;
use crate::runtime::global;

/// The appropriate OS config dir, currently used to store serialized
/// [`GlobalSettings`]
pub fn config_dir() -> Option<PathBuf> {
    directories_next::BaseDirs::new().map(|base| base.config_dir().join("Xtal"))
}

/// The appropriate OS cache dir, currently used to store individual frame
/// captures during video recording
pub fn cache_dir() -> Option<PathBuf> {
    directories_next::BaseDirs::new().map(|base| base.cache_dir().join("Xtal"))
}

fn global_state_storage_path() -> PathBuf {
    config_dir()
        .unwrap_or_default()
        .join("global_settings.json")
}

pub fn save_global_state(state: GlobalSettings) -> Result<(), Box<dyn Error>> {
    let json = serde_json::to_string_pretty(&state)?;
    let path = global_state_storage_path();
    if let Some(parent_dir) = path.parent() {
        fs::create_dir_all(parent_dir)?;
    }
    fs::write(&path, json)?;
    Ok(())
}

pub fn load_global_state() -> Result<GlobalSettings, Box<dyn Error>> {
    let path = global_state_storage_path();
    let bytes = fs::read(path)?;
    let json = str::from_utf8(&bytes).ok().map(|s| s.to_owned()).unwrap();
    let settings = serde_json::from_str::<GlobalSettings>(&json)?;
    Ok(settings)
}

fn sketch_state_storage_path(sketch_name: &str) -> PathBuf {
    PathBuf::from(global::user_data_dir())
        .join("Controls")
        .join(format!("{}_controls.json", sketch_name))
}

pub fn save_sketch_state<T: TimingSource + std::fmt::Debug + 'static>(
    sketch_name: &str,
    hub: &ControlHub<T>,
    mappings: Mappings,
    exclusions: Vec<String>,
) -> Result<PathBuf, Box<dyn Error>> {
    let state = TransitorySketchState {
        ui_controls: hub.ui_controls.clone(),
        midi_controls: hub.midi_controls.clone(),
        osc_controls: hub.osc_controls.clone(),
        snapshots: hub.snapshots.clone(),
        mappings,
        exclusions,
    };

    let serializable_controls = SerializableSketchState::from(&state);
    let json = serde_json::to_string_pretty(&serializable_controls)?;
    let path = sketch_state_storage_path(sketch_name);
    if let Some(parent_dir) = path.parent() {
        fs::create_dir_all(parent_dir)?;
    }
    fs::write(&path, json)?;
    Ok(path)
}

/// Takes in external sketch state and merges with deserialized state.
/// This ensures that the external state can be the source of truth for ui,
/// midi, and osc keys rather than possibly loading invalid or outdated data
/// from file.
pub fn load_sketch_state<'a>(
    sketch_name: &str,
    state: &'a mut TransitorySketchState,
) -> Result<&'a mut TransitorySketchState, Box<dyn Error>> {
    let path = sketch_state_storage_path(sketch_name);
    let bytes = fs::read(path)?;
    let json = str::from_utf8(&bytes).ok().map(|s| s.to_owned()).unwrap();

    let serialized = serde_json::from_str::<SerializableSketchState>(&json)?;
    state.merge(serialized);

    Ok(state)
}

// -----------------------------------------------------------------------------
// Image Index
// -----------------------------------------------------------------------------

/// The image index is used because OSs and online services are really bad at
/// maintaining the date_created field and this is important to me
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
    PathBuf::from(global::user_data_dir()).join("images_metadata.json")
}

pub fn image_metadata_exists() -> bool {
    image_index_path().try_exists().unwrap_or(false)
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
