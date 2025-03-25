use ipc_channel::ipc::{IpcOneShotServer, IpcReceiver, IpcSender};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;

use super::app::AppEventSender;
use crate::config::{MIDI_CONTROL_IN_PORT, MIDI_CONTROL_OUT_PORT};
use crate::framework::midi::InputsOrOutputs::{Inputs, Outputs};
use crate::framework::prelude::*;
use crate::runtime::app::AppEvent;
use crate::runtime::registry::REGISTRY;

/// Event enum used to send/receive data from our web view using ipc-channel
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Event {
    Advance,
    Alert(String),
    CaptureFrame,
    ClearBuffer,
    Error(String),
    #[serde(rename_all = "camelCase")]
    Init {
        is_light_theme: bool,
        sketch_names: Vec<String>,
        sketch_name: String,
        midi_input_port: String,
        midi_output_port: String,
        midi_input_ports: Vec<(usize, String)>,
        midi_output_ports: Vec<(usize, String)>,
    },
    #[serde(rename_all = "camelCase")]
    LoadSketch {
        sketch_name: String,
        display_name: String,
        tap_tempo_enabled: bool,
        fps: f32,
        bpm: f32,
        controls: Vec<SerializableControl>,
        paused: bool,
    },
    QueueRecord,
    Ready,
    Record,
    Reset,
    SetPaused(bool),
    SetPerfMode(bool),
    SetTapTempoEnabled(bool),
    SetTransitionTime(f32),
    SwitchSketch(String),
    Tap,
    UpdateControlBool {
        name: String,
        value: bool,
    },
    UpdateControlFloat {
        name: String,
        value: f32,
    },
    UpdateControlString {
        name: String,
        value: String,
    },
}

pub type Sender = IpcSender<Event>;
pub type Receiver = IpcReceiver<Event>;

type Bootstrap = (Sender, Receiver);

/// Launches the tao/wry web_view code as a child process. This is necessary
/// because both tao and nannou need to run on a main thread and control the
/// event loop, which we can't have in a single process.
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

    let init_sender = sender.clone();
    let sketch_name = sketch_name.to_owned();
    let app_event_tx = app_event_tx.clone();

    thread::spawn(move || {
        while let Ok(message) = receiver.recv() {
            debug!("Received message from child: {:?}", message);

            match &message {
                Event::Advance => {
                    app_event_tx.send(AppEvent::AdvanceSingleFrame)
                }
                // Only sent down
                Event::Alert(_) => {}
                Event::CaptureFrame => {
                    app_event_tx.send(AppEvent::CaptureFrame)
                }
                Event::ClearBuffer => {
                    app_event_tx.send(AppEvent::ClearNextFrame)
                }
                Event::Error(e) => error!("Received error from child: {}", e),
                // Only sent down
                Event::Init { .. } => {}
                // Only sent down
                Event::LoadSketch { .. } => {}
                Event::QueueRecord => app_event_tx.send(AppEvent::QueueRecord),
                Event::Ready => {
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
                        // TODO: replace with list_input_ports etc this is silly
                        midi_input_ports: midi::list_ports(Inputs).unwrap(),
                        midi_output_ports: midi::list_ports(Outputs).unwrap(),
                    };

                    init_sender.send(data).unwrap();
                    app_event_tx.send(AppEvent::WebViewReady);
                }
                Event::Record => app_event_tx.send(AppEvent::Record),
                Event::Reset => app_event_tx.send(AppEvent::Reset),
                Event::SetPaused(paused) => {
                    app_event_tx.send(AppEvent::SetPaused(*paused))
                }
                Event::SetPerfMode(perf_mode) => {
                    app_event_tx.send(AppEvent::SetPerfMode(*perf_mode))
                }
                Event::SetTapTempoEnabled(enabled) => {
                    app_event_tx.send(AppEvent::SetTapTempoEnabled(*enabled))
                }
                Event::SetTransitionTime(time) => {
                    app_event_tx.send(AppEvent::SetTransitionTime(*time))
                }
                Event::SwitchSketch(sketch_name) => app_event_tx
                    .send(AppEvent::SwitchSketch(sketch_name.clone())),
                Event::Tap => app_event_tx.send(AppEvent::Tap),
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
            };
        }
    });

    Ok(sender)
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
        }
    }
}
