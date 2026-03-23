use std::error::Error;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::str;

use directories_next::{BaseDirs, UserDirs};
use serde::{Deserialize, Serialize};

use super::serialization::{
    GlobalSettings, SerializableSketchState, TransitorySketchState,
};
use super::web_view::Mappings;
use crate::control::ControlHub;
use crate::control::Exclusions;
use crate::motion::TimingSource;

pub fn config_dir() -> Option<PathBuf> {
    BaseDirs::new().map(|base| base.config_dir().join("Xtal"))
}

pub fn cache_dir() -> Option<PathBuf> {
    BaseDirs::new().map(|base| base.cache_dir().join("Xtal"))
}

pub fn default_images_dir() -> String {
    user_dir(|ud| ud.picture_dir(), "Images")
}

pub fn default_user_data_dir() -> String {
    user_dir(|ud| ud.document_dir(), "SketchData")
}

pub fn default_videos_dir() -> String {
    user_dir(|ud| ud.video_dir(), "Videos")
}

fn user_dir(
    dir_fn: impl FnOnce(&UserDirs) -> Option<&std::path::Path>,
    subfolder: &str,
) -> String {
    let primary_path = UserDirs::new()
        .and_then(|ud| dir_fn(&ud).map(|p| p.to_path_buf().join("Xtal")));

    let fallback_path = BaseDirs::new()
        .map(|bd| bd.home_dir().to_path_buf().join("Xtal").join(subfolder));

    primary_path
        .or(fallback_path)
        .unwrap_or_else(|| panic!("Could not determine directory path"))
        .to_string_lossy()
        .into_owned()
}

fn global_state_storage_path(storage_dir: &str) -> PathBuf {
    PathBuf::from(storage_dir).join("global_settings.json")
}

pub fn save_global_state(
    storage_dir: &str,
    state: GlobalSettings,
) -> Result<(), Box<dyn Error>> {
    let json = serde_json::to_string_pretty(&state)?;
    let path = global_state_storage_path(storage_dir);
    if let Some(parent_dir) = path.parent() {
        fs::create_dir_all(parent_dir)?;
    }
    fs::write(&path, json)?;
    Ok(())
}

pub fn load_global_state(
    storage_dir: &str,
) -> Result<GlobalSettings, Box<dyn Error>> {
    let path = global_state_storage_path(storage_dir);
    let bytes = fs::read(path)?;
    let json = str::from_utf8(&bytes).ok().map(|s| s.to_owned()).unwrap();
    let settings = serde_json::from_str::<GlobalSettings>(&json)?;
    Ok(settings)
}

pub fn load_global_state_if_exists(
    storage_dir: &str,
) -> Result<Option<GlobalSettings>, Box<dyn Error>> {
    match load_global_state(storage_dir) {
        Ok(settings) => Ok(Some(settings)),
        Err(err) => {
            if err
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == ErrorKind::NotFound)
            {
                Ok(None)
            } else {
                Err(err)
            }
        }
    }
}

fn sketch_state_storage_path(
    user_data_dir: &str,
    sketch_name: &str,
) -> PathBuf {
    PathBuf::from(user_data_dir)
        .join("Controls")
        .join(format!("{}_controls.json", sketch_name))
}

pub fn save_sketch_state<T: TimingSource + std::fmt::Debug + 'static>(
    user_data_dir: &str,
    sketch_name: &str,
    hub: &ControlHub<T>,
    mappings: Mappings,
    exclusions: Exclusions,
) -> Result<PathBuf, Box<dyn Error>> {
    let state = TransitorySketchState::from_hub(hub, mappings, exclusions);
    let serializable_controls = SerializableSketchState::from(&state);

    let json = serde_json::to_string_pretty(&serializable_controls)?;
    let path = sketch_state_storage_path(user_data_dir, sketch_name);
    if let Some(parent_dir) = path.parent() {
        fs::create_dir_all(parent_dir)?;
    }
    fs::write(&path, json)?;
    Ok(path)
}

pub fn load_sketch_state<'a>(
    user_data_dir: &str,
    sketch_name: &str,
    state: &'a mut TransitorySketchState,
) -> Result<&'a mut TransitorySketchState, Box<dyn Error>> {
    let path = sketch_state_storage_path(user_data_dir, sketch_name);
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

fn image_index_path(user_data_dir: &str) -> PathBuf {
    PathBuf::from(user_data_dir).join("images_metadata.json")
}

pub fn image_metadata_exists(user_data_dir: &str) -> bool {
    image_index_path(user_data_dir)
        .try_exists()
        .unwrap_or(false)
}

pub fn load_image_index(user_data_dir: &str) -> Result<ImageIndex, Box<dyn Error>> {
    let bytes = fs::read(image_index_path(user_data_dir))?;
    let json = str::from_utf8(&bytes).ok().map(|s| s.to_owned()).unwrap();
    let image_index_file: ImageIndex = serde_json::from_str(&json)?;
    Ok(image_index_file)
}

pub fn save_image_index(
    user_data_dir: &str,
    image_index: &ImageIndex,
) -> Result<(), Box<dyn Error>> {
    let json = serde_json::to_string_pretty(image_index)?;
    fs::write(image_index_path(user_data_dir), json)?;
    Ok(())
}
