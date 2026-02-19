use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use super::web_view;
use crate::control::ControlValue;

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeEvent {
    AdvanceSingleFrame,
    CaptureFrame,
    ChangeAudioDevice(String),
    ChangeMidiClockPort(String),
    ChangeMidiControlInputPort(String),
    ChangeMidiControlOutputPort(String),
    ChangeOscPort(u16),
    ClearBuffer,
    CommitMappings,
    CurrentlyMapping(String),
    UpdateExclusions(Vec<String>),
    OpenOsDir(web_view::OsDir),
    Pause(bool),
    QueueRecord,
    ReceiveDir(web_view::UserDir, String),
    ReceiveMappings(web_view::Mappings),
    RemoveMapping(String),
    Save(Vec<String>),
    SendMidi,
    SetHrcc(bool),
    SetMappingsEnabled(bool),
    SetPerfMode(bool),
    SetTransitionTime(f32),
    StartRecording,
    StopRecording,
    Quit,
    Randomize(Vec<String>),
    ReloadControls,
    Reset,
    SnapshotDelete(String),
    SnapshotRecall(String),
    SnapshotStore(String),
    SwitchSketch(String),
    Tap,
    TapTempoEnabled(bool),
    ToggleFullScreen,
    ToggleMainFocus,
    UpdateUiControl((String, ControlValue)),
    HubPopulated,
    SnapshotEnded,
    FrameSkipped,
    SketchSwitched(String),
    WebView(web_view::Event),
    Stopped,
}

pub type RuntimeCommand = RuntimeEvent;
pub type RuntimeCommandSender = Sender<RuntimeCommand>;
pub type RuntimeCommandReceiver = Receiver<RuntimeCommand>;
pub type RuntimeEventSender = Sender<RuntimeEvent>;
pub type RuntimeEventReceiver = Receiver<RuntimeEvent>;

pub fn command_channel() -> (RuntimeCommandSender, RuntimeCommandReceiver) {
    mpsc::channel()
}

pub fn event_channel() -> (RuntimeEventSender, RuntimeEventReceiver) {
    mpsc::channel()
}
