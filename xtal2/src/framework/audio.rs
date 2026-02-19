use std::error::Error;

use cpal::traits::{DeviceTrait, HostTrait};

pub fn list_audio_devices() -> Result<Vec<String>, Box<dyn Error>> {
    let host = cpal::default_host();
    let mut devices = Vec::new();
    for device in host.input_devices()? {
        devices.push(device.name()?);
    }
    Ok(devices)
}
