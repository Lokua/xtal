use directories_next::{BaseDirs, UserDirs};
use std::error::Error;
use std::sync::{LazyLock, Mutex};

use crate::framework::prelude::*;

const DEFAULT_OSC_PORT: u16 = 2346;

/// Stores global state that is not easily shared via call chains
pub static GLOBAL: LazyLock<Mutex<Global>> =
    LazyLock::new(|| Mutex::new(Global::default()));

pub fn audio_device_name() -> Option<String> {
    let global = GLOBAL.lock().unwrap();
    global.audio_device_name.clone()
}

pub fn set_audio_device_name(name: &str) {
    let mut global = GLOBAL.lock().unwrap();
    global.audio_device_name = set_device_or_fallback(
        "Audio device",
        name,
        list_audio_devices,
        |name| name,
    );
}

pub fn images_dir() -> String {
    let global = GLOBAL.lock().unwrap();
    global.images_dir.clone()
}

pub fn set_images_dir(dir: &str) {
    let mut global = GLOBAL.lock().unwrap();
    global.images_dir = dir.to_string();
}

pub fn midi_clock_port() -> Option<String> {
    let global = GLOBAL.lock().unwrap();
    global.midi_clock_port.clone()
}

pub fn set_midi_clock_port(port: &str) {
    let mut global = GLOBAL.lock().unwrap();
    global.midi_clock_port = set_device_or_fallback(
        "MIDI clock port",
        port,
        midi::list_input_ports,
        |(_, name)| name,
    );
}

pub fn midi_control_in_port() -> Option<String> {
    let global = GLOBAL.lock().unwrap();
    global.midi_control_in_port.clone()
}

pub fn set_midi_control_in_port(port: &str) {
    let mut global = GLOBAL.lock().unwrap();
    global.midi_clock_port = set_device_or_fallback(
        "MIDI control in port",
        port,
        midi::list_input_ports,
        |(_, name)| name,
    );
}

pub fn midi_control_out_port() -> Option<String> {
    let global = GLOBAL.lock().unwrap();
    global.midi_control_out_port.clone()
}

pub fn set_midi_control_out_port(port: &str) {
    let mut global = GLOBAL.lock().unwrap();
    global.midi_clock_port = set_device_or_fallback(
        "MIDI control out port",
        port,
        midi::list_input_ports,
        |(_, name)| name,
    );
}

pub fn osc_port() -> u16 {
    let global = GLOBAL.lock().unwrap();
    global.osc_port
}

pub fn set_osc_port(port: u16) {
    let mut global = GLOBAL.lock().unwrap();
    global.osc_port = port;
}

pub fn user_data_dir() -> String {
    let global = GLOBAL.lock().unwrap();
    global.user_data_dir.clone()
}

pub fn set_user_data_dir(dir: &str) {
    let mut global = GLOBAL.lock().unwrap();
    global.user_data_dir = dir.to_string();
}

pub fn videos_dir() -> String {
    let global = GLOBAL.lock().unwrap();
    global.videos_dir.clone()
}

pub fn set_videos_dir(dir: &str) {
    let mut global = GLOBAL.lock().unwrap();
    global.videos_dir = dir.to_string();
}

pub struct Global {
    audio_device_name: Option<String>,
    images_dir: String,
    midi_clock_port: Option<String>,
    midi_control_in_port: Option<String>,
    midi_control_out_port: Option<String>,
    osc_port: u16,
    user_data_dir: String,
    videos_dir: String,
}

impl Default for Global {
    fn default() -> Self {
        let audio_device_name = list_audio_devices()
            .ok()
            .and_then(|devices| devices.first().cloned());

        let midi_input_port = midi::list_input_ports()
            .ok()
            .and_then(|ports| ports.first().map(|(_, name)| name.clone()));

        let midi_output_port = midi::list_output_ports()
            .ok()
            .and_then(|ports| ports.first().map(|(_, name)| name.clone()));

        Self {
            audio_device_name,
            images_dir: user_dir(|ud| ud.picture_dir(), "Images"),
            midi_clock_port: midi_input_port.clone(),
            midi_control_in_port: midi_input_port,
            midi_control_out_port: midi_output_port,
            osc_port: DEFAULT_OSC_PORT,
            user_data_dir: user_dir(|ud| ud.document_dir(), "SketchData"),
            videos_dir: user_dir(|ud| ud.video_dir(), "Videos"),
        }
    }
}

/// Helper function to determine application directories with specific fallback
/// rules:
/// 1. Try to use the specified user directory (Pictures, Movies) + "/Xtal"
/// 2. If unavailable, fall back to homedir + "/Xtal/[Images or Videos]"
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

fn set_device_or_fallback<T>(
    label: &str,
    requested: &str,
    list_fn: impl Fn() -> Result<Vec<T>, Box<dyn Error>>,
    extract_name: impl Fn(&T) -> &str,
) -> Option<String> {
    match list_fn() {
        Ok(devices) => {
            if devices.iter().any(|d| extract_name(d) == requested) {
                Some(requested.to_string())
            } else if let Some(fallback) = devices.first() {
                let fallback_name = extract_name(fallback);
                warn!(
                    "No {label} named '{requested}'; \
                    falling back to '{fallback_name}'"
                );
                Some(fallback_name.to_string())
            } else {
                warn!("No available {label}s");
                None
            }
        }
        Err(err) => {
            warn!("Failed to list {label}s: {err}");
            None
        }
    }
}
