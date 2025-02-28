use midir::Ignore;
use midir::MidiInput;
use midir::MidiOutput;
use std::error::Error;
use std::thread;

use super::prelude::*;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct MidiControlConfig {
    pub channel: u8,
    pub cc: u8,
    pub min: f32,
    pub max: f32,
    pub default: f32,
}

impl MidiControlConfig {
    pub fn new(midi: (u8, u8), range: (f32, f32), default: f32) -> Self {
        let (channel, cc) = midi;
        let (min, max) = range;
        Self {
            channel,
            cc,
            min,
            max,
            default,
        }
    }
}

#[derive(Default)]
struct MidiState {
    values: HashMap<String, f32>,
}

impl MidiState {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn get(&self, name: &str) -> f32 {
        *self.values.get(name).unwrap_or(&0.0)
    }

    pub fn set(&mut self, name: &str, value: f32) {
        self.values.insert(name.to_string(), value);
    }

    pub fn has(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }

    pub fn values(&self) -> HashMap<String, f32> {
        return self.values.clone();
    }
}

pub struct MidiControls {
    configs: HashMap<String, MidiControlConfig>,
    state: Arc<Mutex<MidiState>>,
    is_active: bool,
}

impl MidiControls {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            state: Arc::new(Mutex::new(MidiState::new())),
            is_active: false,
        }
    }

    pub fn add(&mut self, name: &str, config: MidiControlConfig) {
        self.state.lock().unwrap().set(name, config.default);
        self.configs.insert(name.to_string(), config);
    }

    pub fn get(&self, name: &str) -> f32 {
        self.state.lock().unwrap().get(name)
    }

    pub fn set(&self, name: &str, value: f32) {
        self.state.lock().unwrap().set(name, value)
    }

    pub fn has(&self, name: &str) -> bool {
        self.state.lock().unwrap().has(name)
    }

    pub fn values(&self) -> HashMap<String, f32> {
        return self.state.lock().unwrap().values();
    }

    fn start(&mut self) {
        let state = self.state.clone();
        let configs = self.configs.clone();

        match on_message(
            move |message| {
                if message.len() < 3 {
                    return;
                }

                let status = message[0];
                let channel = status & 0x0F;
                let cc = message[1];
                let value = message[2] as f32 / 127.0;

                for (name, config) in configs.iter() {
                    if config.channel == channel && config.cc == cc {
                        let mapped_value =
                            value * (config.max - config.min) + config.min;

                        trace!(
                            "Message - \
                                channel: {}, \
                                cc: {}, \
                                value: {}, \
                                mapped: {}",
                            channel,
                            cc,
                            value,
                            mapped_value
                        );

                        state.lock().unwrap().set(name, mapped_value);
                    }
                }
            },
            "[MidiControls]",
        ) {
            Ok(_) => {
                self.is_active = true;
                info!("MidiControls initialized successfully");
            }
            Err(e) => {
                warn!("Failed to initialize MidiControls: {}. Using default values.", e);
                self.is_active = false;
            }
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }
}

pub struct MidiControlBuilder {
    controls: MidiControls,
}

impl MidiControlBuilder {
    pub fn new() -> Self {
        Self {
            controls: MidiControls::new(),
        }
    }

    pub fn control_mapped(
        mut self,
        name: &str,
        midi: (u8, u8),
        range: (f32, f32),
        default: f32,
    ) -> Self {
        self.controls
            .add(name, MidiControlConfig::new(midi, range, default));
        self
    }

    pub fn control(mut self, name: &str, midi: (u8, u8), default: f32) -> Self {
        self.controls
            .add(name, MidiControlConfig::new(midi, (0.0, 1.0), default));
        self
    }

    pub fn build(mut self) -> MidiControls {
        self.controls.start();
        self.controls
    }
}

pub fn on_message<F>(
    callback: F,
    connection_purpose: &str,
) -> Result<(), Box<dyn Error>>
where
    F: Fn(&[u8]) + Send + Sync + 'static,
{
    let midi_in = MidiInput::new("Lattice Shared Input")?;

    let in_ports = midi_in.ports();
    let in_port = in_ports
        .iter()
        .find(|p| {
            midi_in.port_name(p).unwrap_or_default()
                == crate::config::MIDI_CLOCK_PORT
        })
        .expect("Unable to find input port")
        .clone();

    let connection_purpose = connection_purpose.to_string();
    thread::spawn(move || {
        // _conn_in needs to be a named parameter,
        // because it needs to be kept alive until the end of the scope
        let _conn_in = midi_in
            .connect(
                &in_port,
                "Lattice Shared Input Read",
                move |_stamp, message, _| {
                    trace!("MIDI message: {:?}", message);
                    callback(message);
                },
                (),
            )
            .expect("Unable to connect");

        info!(
            "Connected to {}, connection_purpose: {}",
            crate::config::MIDI_CLOCK_PORT,
            connection_purpose
        );

        thread::park();
    });

    Ok(())
}

pub fn print_ports() -> Result<(), Box<dyn Error>> {
    let mut midi_in = MidiInput::new("midir test input")?;
    midi_in.ignore(Ignore::None);
    let midi_out = MidiOutput::new("midir test output")?;

    println!("\nAvailable input ports:");
    for (i, p) in midi_in.ports().iter().enumerate() {
        println!("    {}: {}", i, midi_in.port_name(p)?);
    }

    println!("\nAvailable output ports:");
    for (i, p) in midi_out.ports().iter().enumerate() {
        println!("    {}: {}", i, midi_out.port_name(p)?);
    }

    println!("");

    Ok(())
}
