use std::sync::Mutex;

use once_cell::sync::Lazy;

use crate::config::{
    MIDI_CLOCK_PORT, MIDI_CONTROL_IN_PORT, MIDI_CONTROL_OUT_PORT,
    MULTICHANNEL_AUDIO_DEVICE_NAME,
};

pub static GLOBAL: Lazy<Mutex<Global>> =
    Lazy::new(|| Mutex::new(Global::default()));

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

pub fn audio_device_name() -> String {
    let global = GLOBAL.lock().unwrap();
    global.audio_device_name.clone()
}

pub fn set_audio_device_name(name: &str) {
    let mut global = GLOBAL.lock().unwrap();
    global.audio_device_name = name.to_string();
}

pub struct Global {
    midi_clock_port: String,
    midi_control_in_port: String,
    midi_control_out_port: String,
    audio_device_name: String,
}

impl Default for Global {
    fn default() -> Self {
        Self {
            midi_clock_port: MIDI_CLOCK_PORT.to_string(),
            midi_control_in_port: MIDI_CONTROL_IN_PORT.to_string(),
            midi_control_out_port: MIDI_CONTROL_OUT_PORT.to_string(),
            audio_device_name: MULTICHANNEL_AUDIO_DEVICE_NAME.to_string(),
        }
    }
}
