use directories_next::{BaseDirs, UserDirs};
use std::sync::{LazyLock, Mutex};

use crate::framework::prelude::*;

const DEFAULT_OSC_PORT: u16 = 2346;

/// Stores global state that is not easily shared via call chains
pub static GLOBAL: LazyLock<Mutex<Global>> =
    LazyLock::new(|| Mutex::new(Global::default()));

pub fn audio_device_name() -> String {
    let global = GLOBAL.lock().unwrap();
    global.audio_device_name.clone()
}

pub fn set_audio_device_name(name: &str) {
    let mut global = GLOBAL.lock().unwrap();
    global.audio_device_name = name.to_string();
}

pub fn images_dir() -> String {
    let global = GLOBAL.lock().unwrap();
    global.images_dir.clone()
}

pub fn set_images_dir(dir: String) {
    let mut global = GLOBAL.lock().unwrap();
    global.images_dir = dir;
}

pub fn midi_clock_port() -> String {
    let global = GLOBAL.lock().unwrap();
    global.midi_clock_port.clone()
}

pub fn set_midi_clock_port(port: String) {
    let mut global = GLOBAL.lock().unwrap();
    global.midi_clock_port = port;
}

pub fn midi_control_in_port() -> String {
    let global = GLOBAL.lock().unwrap();
    global.midi_control_in_port.clone()
}

pub fn set_midi_control_in_port(port: String) {
    let mut global = GLOBAL.lock().unwrap();
    global.midi_control_in_port = port;
}

pub fn midi_control_out_port() -> String {
    let global = GLOBAL.lock().unwrap();
    global.midi_control_out_port.clone()
}

pub fn set_midi_control_out_port(port: String) {
    let mut global = GLOBAL.lock().unwrap();
    global.midi_control_out_port = port;
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

pub fn set_user_data_dir(dir: String) {
    let mut global = GLOBAL.lock().unwrap();
    global.user_data_dir = dir;
}

pub fn videos_dir() -> String {
    let global = GLOBAL.lock().unwrap();
    global.videos_dir.clone()
}

pub fn set_videos_dir(dir: String) {
    let mut global = GLOBAL.lock().unwrap();
    global.videos_dir = dir;
}

pub struct Global {
    audio_device_name: String,
    images_dir: String,
    midi_clock_port: String,
    midi_control_in_port: String,
    midi_control_out_port: String,
    osc_port: u16,
    user_data_dir: String,
    videos_dir: String,
}

impl Default for Global {
    fn default() -> Self {
        let midi_input_port = midi::list_input_ports().map_or_else(
            |_| String::new(),
            |ports| {
                ports
                    .first()
                    .map(|(_, port)| {
                        trace!("Default MIDI input port: {}", port);
                        port.clone()
                    })
                    .unwrap_or_default()
            },
        );

        let midi_output_port = midi::list_output_ports().map_or_else(
            |_| String::new(),
            |ports| {
                ports
                    .first()
                    .map(|(_, port)| {
                        trace!("Default MIDI output port: {}", port);
                        port.clone()
                    })
                    .unwrap_or_default()
            },
        );

        Self {
            audio_device_name: list_audio_devices().map_or_else(
                |_| String::new(),
                |devices| {
                    devices
                        .first()
                        .map(|device| {
                            trace!("Default audio device: {}", device);
                            device.clone()
                        })
                        .unwrap_or_default()
                },
            ),
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
