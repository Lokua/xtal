use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use super::web_view;

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeCommand {
    AdvanceSingleFrame,
    Pause(bool),
    SetPerfMode(bool),
    Quit,
    ReloadControls,
    SwitchSketch(String),
    ToggleFullScreen,
    ToggleMainFocus,
    UpdateControlBool { name: String, value: bool },
    UpdateControlFloat { name: String, value: f32 },
    UpdateControlString { name: String, value: String },
}

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeEvent {
    FrameAdvanced(u64),
    FrameSkipped,
    SketchSwitched(String),
    WebView(web_view::Event),
    Stopped,
}

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
