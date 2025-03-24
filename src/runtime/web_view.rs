use ipc_channel::ipc::{IpcOneShotServer, IpcReceiver, IpcSender};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;

use super::app::AppEventSender;
use crate::config::{MIDI_CONTROL_IN_PORT, MIDI_CONTROL_OUT_PORT};
use crate::framework::prelude::*;
use crate::runtime::app::AppEvent;
use crate::runtime::registry::REGISTRY;

pub trait ToMsg {
    fn to_msg(&self) -> String;
}

pub trait FromMsg {
    fn from_msg(s: &str) -> Self;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SerializableControl {
    Slider {
        name: String,
        value: f32,
        min: f32,
        max: f32,
        step: f32,
        disabled: bool,
    },
    Checkbox {
        name: String,
        value: bool,
        disabled: bool,
    },
    Select {
        name: String,
        value: String,
        options: Vec<String>,
        disabled: bool,
    },
    Separator {},
    DynamicSeparator {
        name: String,
    },
}

impl From<(&Control, &UiControls)> for SerializableControl {
    fn from((control, ui_controls): (&Control, &UiControls)) -> Self {
        match control {
            Control::Slider {
                name,
                min,
                max,
                step,
                ..
            } => SerializableControl::Slider {
                name: name.clone(),
                value: ui_controls.float(name),
                min: *min,
                max: *max,
                step: *step,
                disabled: control.is_disabled(ui_controls),
            },
            Control::Checkbox { name, .. } => SerializableControl::Checkbox {
                name: name.clone(),
                value: ui_controls.bool(name),
                disabled: control.is_disabled(ui_controls),
            },
            Control::Select { name, options, .. } => {
                SerializableControl::Select {
                    name: name.clone(),
                    value: ui_controls.string(name),
                    options: options.clone(),
                    disabled: control.is_disabled(ui_controls),
                }
            }
            Control::Separator {} => SerializableControl::Separator {},
            Control::DynamicSeparator { name } => {
                SerializableControl::DynamicSeparator { name: name.clone() }
            }
        }
    }
}

impl ToMsg for UiControls {
    fn to_msg(&self) -> String {
        self.configs()
            .iter()
            .map(|config| match config {
                Control::Checkbox { name, .. } => {
                    let value = self.bool(name);
                    format!(
                        "type=checkbox;name={};value={};disabled=false",
                        name, value
                    )
                }
                Control::Slider { name, min, max, step, .. } => {
                    let value = self.float(name);
                    format!(
                        "type=slider;name={};value={};min={};max={};step={};disabled=false",
                        name, value, min, max, step
                    )
                }
                _ => "".to_string(),
            })
            .collect::<Vec<String>>().join("\n")
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum Event {
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
    },
    #[serde(rename = "loadSketch", rename_all = "camelCase")]
    LoadSketch {
        sketch_name: String,
        display_name: String,
        controls: Vec<SerializableControl>,
    },
    #[serde(rename = "updateControlBool")]
    UpdateControlBool {
        name: String,
        value: bool,
    },
    #[serde(rename = "updateControlFloat")]
    UpdateControlFloat {
        name: String,
        value: f32,
    },
    #[serde(rename = "updateControlString")]
    UpdateControlString {
        name: String,
        value: String,
    },
    Test,
    #[serde(rename = "error")]
    Error(String),
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
        .env("RUST_LOG", "lattice=info,web_view_poc=debug")
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

    let (_receiver, (sender, receiver)): (IpcReceiver<Bootstrap>, Bootstrap) =
        server.accept()?;

    let event_tx_clone = app_event_tx.clone();
    let init_sender = sender.clone();
    let sketch_name = sketch_name.to_owned();
    let app_event_tx = app_event_tx.clone();

    thread::spawn(move || {
        while let Ok(message) = receiver.recv() {
            trace!("Received message from child: {:?}", message);

            match &message {
                Event::UpdateControlBool { name, value } => {
                    app_event_tx.send(AppEvent::UpdateUiControl((
                        name.clone(),
                        ControlValue::from(*value),
                    )))
                }
                Event::UpdateControlFloat { name, value } => {
                    app_event_tx.send(AppEvent::UpdateUiControl((
                        name.clone(),
                        ControlValue::from(*value),
                    )))
                }
                Event::UpdateControlString { name, value } => app_event_tx
                    .send(AppEvent::UpdateUiControl((
                        name.clone(),
                        ControlValue::from(value.clone()),
                    ))),
                Event::Reset => event_tx_clone.send(AppEvent::Reset),
                Event::Tap => event_tx_clone.send(AppEvent::Tap),
                Event::Ready => {
                    debug!("wv received ready event");
                    let registry = REGISTRY.read().unwrap();

                    let data = Event::Init {
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

                    init_sender.send(data).unwrap();
                    app_event_tx.send(AppEvent::WebViewReady);
                }
                _ => trace!("No handler for event: {:?}", message),
            };
        }
    });

    Ok(sender)
}
