use ipc_channel::ipc::{IpcOneShotServer, IpcReceiver, IpcSender};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;

use super::app::AppEventSender;
use crate::config::{MIDI_CONTROL_IN_PORT, MIDI_CONTROL_OUT_PORT};
// use crate::framework::control::config::ControlType;
use crate::framework::prelude::*;
use crate::runtime::app::AppEvent;
use crate::runtime::registry::REGISTRY;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum Data {
    #[default]
    Empty,
    Tap,
    Ready,
    Reset,
    String(String),
    Json(serde_json::Value),
    #[serde(rename = "init", rename_all = "camelCase")]
    Init {
        is_light_theme: bool,
        sketch_names: Vec<String>,
        sketch_name: String,
        midi_input_port: String,
        midi_output_port: String,
        midi_input_ports: Vec<(usize, String)>,
        midi_output_ports: Vec<(usize, String)>,
        // controls: Vec<ControlType>,
    },
    Test,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Event {
    pub event: String,
    pub data: Option<Data>,
}

impl Event {
    pub fn new(event: &str) -> Self {
        Self {
            event: event.to_string(),
            data: None,
        }
    }

    pub fn with_data(event: &str, data: Data) -> Self {
        Self {
            event: event.to_string(),
            data: Some(data),
        }
    }
}

pub type Sender = IpcSender<Event>;
pub type Receiver = IpcReceiver<Event>;

type Bootstrap = (Sender, Receiver);

pub fn launch(
    app_event_tx: &AppEventSender,
    sketch_name: &str,
) -> Result<Sender, Box<dyn std::error::Error>> {
    let (server, server_name) = IpcOneShotServer::<Bootstrap>::new()?;

    let mut child = Command::new("cargo")
        .args(["run", "--release", "--bin", "web_view_poc", &server_name])
        .env("RUST_LOG", "lattice=info,dx_poc=debug")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    trace!("Child process spawned");

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        thread::spawn(move || {
            for line in reader.lines().map_while(Result::ok) {
                println!("{}", line);
            }
        });
    }

    thread::sleep(std::time::Duration::from_millis(100));

    let (_receiver, (sender, receiver)): (IpcReceiver<Bootstrap>, Bootstrap) =
        server.accept()?;

    let event_tx_clone = app_event_tx.clone();
    let init_sender = sender.clone();
    let sketch_name = sketch_name.to_owned();

    thread::spawn(move || {
        while let Ok(message) = receiver.recv() {
            trace!("Received message from child: {:?}", message);

            match message.event.as_str() {
                "reset" => event_tx_clone.send(AppEvent::Reset),
                "tap" => event_tx_clone.send(AppEvent::Tap),
                "ready" => {
                    let registry = REGISTRY.read().unwrap();

                    let data = Data::Init {
                        is_light_theme: matches!(
                            dark_light::detect(),
                            dark_light::Mode::Light
                        ),
                        sketch_names: registry.names().clone(),
                        sketch_name: sketch_name.to_string(),
                        midi_input_port: MIDI_CONTROL_IN_PORT.to_string(),
                        midi_output_port: MIDI_CONTROL_OUT_PORT.to_string(),
                        midi_input_ports: midi::list_ports(
                            midi::InputsOrOutputs::Inputs,
                        )
                        .unwrap(),
                        midi_output_ports: midi::list_ports(
                            midi::InputsOrOutputs::Outputs,
                        )
                        .unwrap(),
                    };

                    init_sender.send(Event::with_data("init", data)).unwrap()
                }
                _ => warn!("Unknown event: {}", message.event),
            }
        }
    });

    Ok(sender)
}
