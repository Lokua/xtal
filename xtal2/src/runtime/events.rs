use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeCommand {
    AdvanceSingleFrame,
    Pause(bool),
    Quit,
    ReloadControls,
    SwitchSketch(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeEvent {
    FrameAdvanced(u64),
    FrameSkipped,
    SketchSwitched(String),
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
