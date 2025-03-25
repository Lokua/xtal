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

/// Used to send/receive data from our web view using ipc-channel. Most events
/// should be assumed to be one-way from child to parent unless otherwise
/// documented.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Event {
    Advance,
    Alert(String),
    AverageFps(f32),

    /// Sent from parent after receiving Tap event
    Bpm(f32),
    CaptureFrame,
    ClearBuffer,
    Error(String),
    HubPopulated(Vec<SerializableControl>),

    /// Sent from parent after child sends [`Event::Ready`]
    #[serde(rename_all = "camelCase")]
    Init {
        is_light_theme: bool,
        midi_input_port: String,
        midi_output_port: String,
        midi_input_ports: Vec<(usize, String)>,
        midi_output_ports: Vec<(usize, String)>,
        sketch_names: Vec<String>,
        sketch_name: String,
    },

    /// Sent after the child emits [`Event::SwitchSketch`]
    #[serde(rename_all = "camelCase")]
    LoadSketch {
        bpm: f32,
        controls: Vec<SerializableControl>,
        display_name: String,
        fps: f32,
        paused: bool,
        mappings: Vec<(String, ChannelAndControl)>,
        sketch_name: String,
        tap_tempo_enabled: bool,
    },

    // Sent whenever the user physically moves a MIDI control when in map mode
    Mappings(Vec<(String, ChannelAndControl)>),
    QueueRecord,
    Ready,

    /// A two-way message. Can be sent manually from UI, or set from backend
    /// when receiving a MIDI Start when QueueRecording is enabled
    Record,
    RemoveMapping(String),
    Reset,
    Save,
    SendMidi,
    // TODO: "set" is bit ugly - just use the var name
    SetCurrentlyMapping(String),
    SetHrcc(bool),
    SetIsEncoding(bool),
    SetPaused(bool),
    SetPerfMode(bool),
    SetTapTempoEnabled(bool),
    SetTransitionTime(f32),
    SnapshotEnded(Vec<SerializableControl>),
    SnapshotRecall(String),
    SnapshotStore(String),

    /// A two-way message. Can be sent manually from UI, or set from backend
    /// when receiving a MIDI Stop when QueueRecording is enabled
    StopRecording,

    SwitchSketch(String),
    Tap,
    ToggleFullScreen,
    ToggleGuiFocus,
    ToggleMainFocus,
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

#[derive(Clone)]
pub struct EventSender {
    tx: Sender,
}

impl EventSender {
    pub fn new(tx: Sender) -> Self {
        Self { tx }
    }

    pub fn emit(&self, event: Event) {
        self.tx.send(event).expect("Failed to send event");
    }
}

type Bootstrap = (Sender, Receiver);

/// Launches the tao/wry web_view code as a child process and sets up IPC
/// channels. This is necessary because both tao and nannou need to run on a
/// main thread and control the event loop, which we can't have in a single
/// process.
pub fn launch(
    app_event_tx: &AppEventSender,
    sketch_name: &str,
) -> Result<EventSender, Box<dyn std::error::Error>> {
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

            match message {
                Event::Advance => {
                    app_event_tx.emit(AppEvent::AdvanceSingleFrame);
                }
                Event::Alert(_) => {}
                Event::AverageFps(_) => {}
                Event::Bpm(_) => {}
                Event::CaptureFrame => {
                    app_event_tx.emit(AppEvent::CaptureFrame);
                }
                Event::ClearBuffer => {
                    app_event_tx.emit(AppEvent::ClearNextFrame);
                }
                Event::Error(e) => error!("Received error from child: {}", e),
                Event::HubPopulated(_) => {}
                Event::Init { .. } => {}
                Event::LoadSketch { .. } => {}
                Event::Mappings(mappings) => {
                    app_event_tx.emit(AppEvent::ReceiveMappings(mappings));
                }
                Event::QueueRecord => {
                    app_event_tx.emit(AppEvent::QueueRecord);
                }
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
                        midi_input_ports: midi::list_ports(Inputs).unwrap(),
                        midi_output_ports: midi::list_ports(Outputs).unwrap(),
                    };

                    init_sender.send(data).unwrap();
                    app_event_tx.emit(AppEvent::WebViewReady);
                }
                Event::Record => {
                    app_event_tx.emit(AppEvent::StartRecording);
                }
                Event::RemoveMapping(name) => {
                    app_event_tx.emit(AppEvent::RemoveMapping(name));
                }
                Event::Reset => {
                    app_event_tx.emit(AppEvent::Reset);
                }
                Event::Save => {
                    app_event_tx.emit(AppEvent::SaveProgramState);
                }
                Event::SendMidi => {
                    app_event_tx.emit(AppEvent::SendMidi);
                }
                Event::SetCurrentlyMapping(name) => {
                    app_event_tx
                        .emit(AppEvent::SetCurrentlyMapping(name.clone()));
                }
                Event::SetHrcc(hrcc) => {
                    app_event_tx.emit(AppEvent::SetHrcc(hrcc));
                }
                Event::SetIsEncoding(_) => {}
                Event::SetPaused(paused) => {
                    app_event_tx.emit(AppEvent::SetPaused(paused));
                }
                Event::SetPerfMode(perf_mode) => {
                    app_event_tx.emit(AppEvent::SetPerfMode(perf_mode));
                }
                Event::SetTapTempoEnabled(enabled) => {
                    app_event_tx.emit(AppEvent::SetTapTempoEnabled(enabled));
                }
                Event::SetTransitionTime(time) => {
                    app_event_tx.emit(AppEvent::SetTransitionTime(time));
                }
                Event::SnapshotEnded(_) => {}
                Event::SnapshotRecall(id) => {
                    app_event_tx.emit(AppEvent::SnapshotRecall(id.clone()));
                }
                Event::SnapshotStore(id) => {
                    app_event_tx.emit(AppEvent::SnapshotStore(id.clone()));
                }
                Event::StopRecording => {
                    app_event_tx.emit(AppEvent::StopRecording);
                }
                Event::SwitchSketch(sketch_name) => {
                    app_event_tx
                        .emit(AppEvent::SwitchSketch(sketch_name.clone()));
                }
                Event::Tap => {
                    app_event_tx.emit(AppEvent::Tap);
                }
                Event::ToggleFullScreen => {
                    app_event_tx.emit(AppEvent::ToggleFullScreen);
                }
                Event::ToggleGuiFocus => {
                    app_event_tx.emit(AppEvent::ToggleGuiFocus);
                }
                Event::ToggleMainFocus => {
                    app_event_tx.emit(AppEvent::ToggleMainFocus);
                }
                Event::UpdateControlBool { name, value } => {
                    app_event_tx.emit(AppEvent::UpdateUiControl((
                        name.clone(),
                        ControlValue::from(value),
                    )))
                }
                Event::UpdateControlFloat { name, value } => {
                    app_event_tx.emit(AppEvent::UpdateUiControl((
                        name.clone(),
                        ControlValue::from(value),
                    )))
                }
                Event::UpdateControlString { name, value } => app_event_tx
                    .emit(AppEvent::UpdateUiControl((
                        name.clone(),
                        ControlValue::from(value.clone()),
                    ))),
            };
        }
    });

    Ok(EventSender::new(sender))
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

impl From<(&Control, &ControlHub<Timing>)> for SerializableControl {
    fn from((control, hub): (&Control, &ControlHub<Timing>)) -> Self {
        match control {
            Control::Checkbox { name, .. } => SerializableControl::Checkbox {
                name: name.clone(),
                value: hub.bool(name),
                disabled: control.is_disabled(&hub.ui_controls),
            },
            Control::Select { name, options, .. } => {
                SerializableControl::Select {
                    name: name.clone(),
                    value: hub.string(name),
                    options: options.clone(),
                    disabled: control.is_disabled(&hub.ui_controls),
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
                value: hub.get(name),
                min: *min,
                max: *max,
                step: *step,
                disabled: control.is_disabled(&hub.ui_controls),
            },
        }
    }
}
