//! Launches and sets up communication channels with a child process responsible
//! for rendering our frontend.
//!
//! # Event Flow
//! ```md
//! Frontend Interaction ->
//! WebView Child Process IPC Sender (web_view::Event) ->
//! Parent Process (main.rs) IPC Receiver (web_view::Event) ->
//! app::AppEvent
//! ```
//! (and vis a versa)
use ipc_channel::ipc::{IpcOneShotServer, IpcReceiver, IpcSender};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::thread;

use super::app::AppEventSender;
use super::map_mode::Mappings;
use crate::framework::control::ui_controls;
use crate::framework::prelude::*;
use crate::runtime::app::AppEvent;

type Bypassed = HashMap<String, f32>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum UserDir {
    Images,
    UserData,
    Videos,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum OsDir {
    Cache,
    Config,
}

/// Used to send/receive data from our app into a web view using ipc-channel.
/// Most events should be assumed to be one-way from child to parent unless
/// otherwise documented.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Event {
    Advance,

    /// Sent from parent to alert frontend of various success/failures
    Alert(String),

    /// Sent from parent every ~1sec
    AverageFps(f32),

    /// Sent from parent after receiving Tap event
    Bpm(f32),
    CaptureFrame,
    ChangeAudioDevice(String),

    /// Event intercepted from frontend -> web_view_process to open a File
    /// Dialog. See [`Event::ReceiveDir`] for making use of the new dir
    ChangeDir(UserDir),
    ChangeMidiClockPort(String),
    ChangeMidiControlInputPort(String),
    ChangeMidiControlOutputPort(String),
    ChangeOscPort(u16),

    ClearBuffer,
    CommitMappings,
    CurrentlyMapping(String),

    /// Sent from parent after a recording has been stopped and video encoding
    /// has began
    Encoding(bool),

    /// TODO: are we even using this?
    Error(String),
    Hrcc(bool),

    /// Sent from parent whenever a control script has changed and controls have
    /// been reloaded
    HubPopulated((Vec<Control>, Bypassed)),

    /// Sent from parent after child sends [`Event::Ready`]
    #[serde(rename_all = "camelCase")]
    Init {
        audio_device: String,
        audio_devices: Vec<String>,
        hrcc: bool,
        images_dir: String,
        is_light_theme: bool,
        mappings_enabled: bool,
        midi_clock_port: String,
        midi_input_port: String,
        midi_output_port: String,
        midi_input_ports: Vec<(usize, String)>,
        midi_output_ports: Vec<(usize, String)>,
        osc_port: u16,
        sketch_names: Vec<String>,
        sketch_name: String,
        transition_time: f32,
        user_data_dir: String,
        videos_dir: String,
    },

    /// Sent after the child emits [`Event::SwitchSketch`]
    #[serde(rename_all = "camelCase")]
    LoadSketch {
        bpm: f32,
        bypassed: Bypassed,
        controls: Vec<Control>,
        display_name: String,
        fps: f32,
        mappings: Mappings,
        paused: bool,
        perf_mode: bool,
        sketch_name: String,
        sketch_width: i32,
        sketch_height: i32,
        snapshot_slots: Vec<String>,
        tap_tempo_enabled: bool,
        exclusions: Exclusions,
    },

    /// Sent whenever the user physically moves a MIDI control when in map mode
    Mappings(Mappings),
    MappingsEnabled(bool),
    OpenOsDir(OsDir),
    Paused(bool),
    PerfMode(bool),
    QueueRecord,
    Quit,
    Ready,

    /// A two-way message:
    /// 1. Sent from web_view_process to here after user has chosen dir
    /// 2. Sent to main app to save dir to global state
    /// 3. Sent from here back to frontend to show the updated dir
    ReceiveDir(UserDir, String),
    Randomize(Exclusions),
    RemoveMapping(String),
    Reset,
    Save(Vec<String>),
    SendMidi,

    /// Sent from parent after a snapshot has completed so we can keep controls
    /// in sync
    SnapshotEnded(Vec<Control>),
    SnapshotDelete(String),
    SnapshotRecall(String),
    SnapshotStore(String),

    /// A two-way message. Can be sent manually from UI, or set from backend
    /// when receiving a MIDI Start when QueueRecording is enabled
    StartRecording,

    /// A two-way message. Can be sent manually from UI, or set from backend
    /// when receiving a MIDI Stop when QueueRecording is enabled
    StopRecording,

    SwitchSketch(String),
    Tap,
    TapTempoEnabled(bool),
    ToggleFullScreen,

    /// Two message depending on which window receives the key event
    ToggleGuiFocus,

    /// Two message depending on which window receives the key event
    ToggleMainFocus,
    TransitionTime(f32),
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

    /// Sent from parent
    UpdatedControls(Vec<Control>),
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
/// process (at least not on all OSs)
pub fn launch(
    app_tx: &AppEventSender,
) -> Result<(EventSender, Child), Box<dyn std::error::Error>> {
    let (server, server_name) = IpcOneShotServer::<Bootstrap>::new()?;

    let mut child = Command::new("cargo")
        .args(["run", "--release", "-p", "lattice", "--", &server_name])
        .env("RUST_LOG", "lattice=debug")
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

    let app_tx = app_tx.clone();
    let wv_tx = sender.clone();

    thread::spawn(move || {
        while let Ok(message) = receiver.recv() {
            trace!("Received message from child: {:?}", message);

            // Events with empty implementations are direct Parent->Child
            match message {
                Event::Advance => {
                    app_tx.emit(AppEvent::AdvanceSingleFrame);
                }
                Event::Alert(_) => {}
                Event::AverageFps(_) => {}
                Event::Bpm(_) => {}
                Event::CaptureFrame => {
                    app_tx.emit(AppEvent::CaptureFrame);
                }
                Event::ChangeAudioDevice(name) => {
                    app_tx.emit(AppEvent::ChangeAudioDevice(name));
                }
                Event::ChangeDir(_) => {}
                Event::ChangeMidiClockPort(port) => {
                    app_tx.emit(AppEvent::ChangeMidiClockPort(port));
                }
                Event::ChangeMidiControlInputPort(port) => {
                    app_tx.emit(AppEvent::ChangeMidiControlInputPort(port));
                }
                Event::ChangeMidiControlOutputPort(port) => {
                    app_tx.emit(AppEvent::ChangeMidiControlOutputPort(port));
                }
                Event::ChangeOscPort(port) => {
                    app_tx.emit(AppEvent::ChangeOscPort(port));
                }
                Event::ClearBuffer => {
                    app_tx.emit(AppEvent::ClearNextFrame);
                }
                Event::CommitMappings => {
                    app_tx.emit(AppEvent::CommitMappings);
                }
                Event::CurrentlyMapping(name) => {
                    app_tx.emit(AppEvent::CurrentlyMapping(name.clone()));
                }
                Event::Encoding(_) => {}
                Event::Error(e) => error!("Received error from child: {}", e),
                Event::Hrcc(hrcc) => {
                    app_tx.emit(AppEvent::Hrcc(hrcc));
                }
                Event::HubPopulated(_) => {}
                Event::Init { .. } => {}
                Event::LoadSketch { .. } => {}
                Event::Mappings(mappings) => {
                    app_tx.emit(AppEvent::ReceiveMappings(mappings));
                }
                Event::MappingsEnabled(enabled) => {
                    app_tx.emit(AppEvent::MappingsEnabled(enabled));
                }
                Event::OpenOsDir(os_dir) => {
                    app_tx.emit(AppEvent::OpenOsDir(os_dir));
                }
                Event::Paused(paused) => {
                    app_tx.emit(AppEvent::Paused(paused));
                }
                Event::PerfMode(perf_mode) => {
                    app_tx.emit(AppEvent::PerfMode(perf_mode));
                }
                Event::QueueRecord => {
                    app_tx.emit(AppEvent::QueueRecord);
                }
                Event::Quit => {
                    app_tx.emit(AppEvent::Quit);
                }
                Event::Randomize(exclusions) => {
                    app_tx.emit(AppEvent::Randomize(exclusions));
                }
                Event::Ready => {
                    app_tx.emit(AppEvent::WebViewReady);
                }
                Event::ReceiveDir(kind, dir) => {
                    app_tx
                        .emit(AppEvent::ReceiveDir(kind.clone(), dir.clone()));
                    wv_tx.send(Event::ReceiveDir(kind, dir)).unwrap();
                }
                Event::RemoveMapping(name) => {
                    app_tx.emit(AppEvent::RemoveMapping(name));
                }
                Event::Reset => {
                    app_tx.emit(AppEvent::Reset);
                }
                Event::StartRecording => {
                    app_tx.emit(AppEvent::StartRecording);
                }
                Event::Save(exclusions) => {
                    app_tx.emit(AppEvent::Save(exclusions));
                }
                Event::SendMidi => {
                    app_tx.emit(AppEvent::SendMidi);
                }
                Event::SnapshotEnded(_) => {}
                Event::SnapshotRecall(id) => {
                    app_tx.emit(AppEvent::SnapshotRecall(id.clone()));
                }
                Event::SnapshotDelete(id) => {
                    app_tx.emit(AppEvent::SnapshotDelete(id.clone()));
                }
                Event::SnapshotStore(id) => {
                    app_tx.emit(AppEvent::SnapshotStore(id.clone()));
                }
                Event::StopRecording => {
                    app_tx.emit(AppEvent::StopRecording);
                }
                Event::SwitchSketch(sketch_name) => {
                    app_tx.emit(AppEvent::SwitchSketch(sketch_name.clone()));
                }
                Event::Tap => {
                    app_tx.emit(AppEvent::Tap);
                }
                Event::TapTempoEnabled(enabled) => {
                    app_tx.emit(AppEvent::TapTempoEnabled(enabled));
                }
                Event::ToggleFullScreen => {
                    app_tx.emit(AppEvent::ToggleFullScreen);
                }
                Event::ToggleGuiFocus => {}
                Event::ToggleMainFocus => {
                    app_tx.emit(AppEvent::ToggleMainFocus);
                }
                Event::TransitionTime(time) => {
                    app_tx.emit(AppEvent::TransitionTime(time));
                }
                Event::UpdateControlBool { name, value } => {
                    app_tx.emit(AppEvent::UpdateUiControl((
                        name.clone(),
                        ControlValue::from(value),
                    )))
                }
                Event::UpdateControlFloat { name, value } => {
                    app_tx.emit(AppEvent::UpdateUiControl((
                        name.clone(),
                        ControlValue::from(value),
                    )))
                }
                Event::UpdateControlString { name, value } => {
                    app_tx.emit(AppEvent::UpdateUiControl((
                        name.clone(),
                        ControlValue::from(value.clone()),
                    )))
                }
                Event::UpdatedControls(_) => {}
            }
        }
    });

    Ok((EventSender::new(sender), child))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ControlKind {
    Checkbox,
    DynamicSeparator,
    Select,
    Separator,
    Slider,
}

/// Provides a uniform type for all [`ui_controls::UiControlConfig`] variants.
/// This is a work around for sending data over [`ipc_channel`] which uses
/// `bincode` for serialization and can't support serde's untagged enum types
/// which leads to really gnarly code on the frontend, for example a list of
/// these:
///
/// ```rust,ignore
/// #[derive(Clone, Debug, Deserialize, Serialize)]
/// #[serde(rename_all = "camelCase")]
/// pub enum SerializableControl {
///     Checkbox {
///         name: String,
///         value: bool,
///         disabled: bool,
///     },
///     // other control impls
/// }
/// ```
///
/// Results in:
///
/// ```tsx
/// [{ checkbox: {...} }, { slider: {...} }]
/// ```
///
/// Which doesn't seem all that bad until you want to start typing with
/// Typescript, filtering, mapping - every regular single thing becomes twice as
/// complicated due to that single key:
///
/// ```tsx
/// import type { Control, Checkbox} from './types.ts'
///
/// const type = Object.keys(control)[0] as keyof Control
/// const control = control[type] as Checkbox['checkbox']
/// ```
///
/// Which just looks stupid and gets worse when you have to do anything real in
/// a generic way, so here we are picking the lesser of two not-so-great
/// solutions by just over-packing the data type
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Control {
    pub kind: ControlKind,
    pub name: String,
    pub value: String,
    pub disabled: bool,
    pub options: Vec<String>,
    pub min: f32,
    pub max: f32,
    pub step: f32,
}

impl Default for Control {
    fn default() -> Self {
        Self {
            kind: ControlKind::Separator,
            name: "<default_name>".to_string(),
            value: "".to_string(),
            disabled: false,
            options: vec![],
            min: 0.0,
            max: 1.0,
            step: 0.001,
        }
    }
}

impl From<(&ui_controls::UiControlConfig, &ControlHub<Timing>)> for Control {
    #[allow(clippy::field_reassign_with_default)]
    fn from(
        (ui_control, hub): (&ui_controls::UiControlConfig, &ControlHub<Timing>),
    ) -> Self {
        let mut result = Control::default();
        result.disabled = ui_control.is_disabled(&hub.ui_controls);
        result.name = ui_control.name().to_string();

        match ui_control {
            ui_controls::UiControlConfig::Checkbox { name, .. } => {
                result.kind = ControlKind::Checkbox;
                result.value = hub.bool(name).to_string();
            }
            ui_controls::UiControlConfig::Select { name, options, .. } => {
                result.kind = ControlKind::Select;
                result.value = hub.string(name);
                result.options = options.clone();
            }
            ui_controls::UiControlConfig::Separator { .. } => {
                result.kind = ControlKind::Separator;
            }
            ui_controls::UiControlConfig::Slider {
                name,
                min,
                max,
                step,
                ..
            } => {
                result.kind = ControlKind::Slider;
                result.value = hub.get(name).to_string();
                result.min = *min;
                result.max = *max;
                result.step = *step;
            }
        }

        result
    }
}
