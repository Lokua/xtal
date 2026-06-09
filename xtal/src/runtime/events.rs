use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use super::web_view;
use crate::control::ControlValue;
use crate::runtime::projector::ProjectorQuality;

/// Runtime command and notification contract.
///
/// Most variants are sent from the web view or keyboard shortcuts into
/// `XtalRuntime::on_runtime_event`. A smaller set is emitted by timing,
/// control hub, render, and shutdown paths so the web view bridge can stay in
/// sync with runtime state.
#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeEvent {
    /// Requests one frame while the frame clock is paused or advancing.
    /// Sent by the UI Advance command and by the main window `A` shortcut.
    AdvanceSingleFrame,
    /// Requests a PNG of the next rendered frame.
    /// Sent by the UI capture button and main window image shortcut.
    CaptureFrame,
    /// Changes the audio input device used by audio controls.
    /// Sent from settings when the selected audio device changes.
    ChangeAudioDevice(String),
    /// Changes the MIDI port used by MIDI and hybrid timing modes.
    /// Sent from settings when the MIDI clock input port changes.
    ChangeMidiClockPort(String),
    /// Changes the MIDI input port used for UI control mapping.
    /// Sent from settings when the MIDI control input changes.
    ChangeMidiControlInputPort(String),
    /// Changes the MIDI output port used by `SendMidi`.
    /// Sent from settings when the MIDI control output changes.
    ChangeMidiControlOutputPort(String),
    /// Changes the shared OSC receive port and restarts the listener.
    /// Sent from settings when the OSC port changes.
    ChangeOscPort(u16),
    /// Requests render-buffer clearing.
    /// Sent by the UI, but the runtime handler is not implemented yet.
    ClearBuffer,
    /// Commits the current MIDI learn mappings into the control hub.
    /// Sent by the controls UI after mapping edits are accepted.
    CommitMappings,
    /// Starts MIDI learn for the named control, or stops learn for empty names.
    /// Sent by the controls UI when the user chooses a control to map.
    CurrentlyMapping(String),
    /// Reports an async MIDI learn error back to the main runtime thread.
    /// Sent by the map-mode listener callback.
    MapModeError(String),
    /// Reports MIDI Continue from MIDI or hybrid timing.
    /// Sent by timing when the selected clock input receives Continue.
    MidiContinue,
    /// Reports MIDI Start from MIDI or hybrid timing.
    /// Sent by timing when the selected clock input receives Start.
    MidiStart,
    /// Reports MIDI Stop from MIDI or hybrid timing.
    /// Sent by timing when the selected clock input receives Stop.
    MidiStop,
    /// Updates the controls excluded from the next randomize/save operation.
    /// Sent by the controls UI as exclusion toggles change.
    UpdateExclusions(Vec<String>),
    /// Opens a runtime-managed directory in the OS file browser.
    /// Sent by settings for cache/config directory buttons.
    OpenOsDir(web_view::OsDir),
    /// Pauses or resumes the frame clock.
    /// Sent by the UI pause toggle and the main window pause shortcut.
    Pause(bool),
    /// Toggles recording queue state while waiting for MIDI Start/Continue.
    /// Sent by the recording UI.
    QueueRecord,
    /// Persists a user-selected images, user data, or videos directory.
    /// Sent by settings after the file-picker returns a path.
    ReceiveDir(web_view::UserDir, String),
    /// Replaces runtime mapping state with a mapping payload from the UI.
    /// Sent by the web view when it restores or edits mapping state.
    ReceiveMappings(web_view::Mappings),
    /// Removes one MIDI mapping and updates the UI mapping payload.
    /// Sent by the controls UI when a mapping is cleared.
    RemoveMapping(String),
    /// Saves current controls, mappings, and exclusions for the active sketch.
    /// Sent by the UI save command and main window save shortcut.
    Save(Vec<String>),
    /// Sends current mapping state back to the web view.
    /// Sent after mapping edits and from async mapping callbacks.
    SendMappings,
    /// Sends the current control values as MIDI messages on the output port.
    /// Sent by the UI and after snapshots/transitions end.
    SendMidi,
    /// Sets the tap-tempo BPM value while tap tempo mode is active.
    /// Sent by the UI BPM control.
    SetBpm(f32),
    /// Toggles 14-bit high-resolution MIDI CC handling.
    /// Sent by settings when HRCC mode changes.
    SetHrcc(bool),
    /// Enables or disables MIDI mapping override application.
    /// Sent by settings when mappings are globally enabled or disabled.
    SetMappingsEnabled(bool),
    /// Opens or closes the monitor preview window.
    /// Sent by settings when monitor preview is toggled.
    SetMonitorPreview(bool),
    /// Enables or disables performance mode.
    /// Sent by settings when performance mode is toggled.
    SetPerfMode(bool),
    /// Enables or disables projector output mode.
    /// Sent by settings when projector mode is toggled.
    SetProjectorMode(bool),
    /// Selects projector render quality.
    /// Sent by settings when the projector quality option changes.
    SetProjectorQuality(ProjectorQuality),
    /// Changes snapshot/randomize transition duration.
    /// Sent by the UI transition-time control.
    SetTransitionTime(f32),
    /// Starts video recording immediately.
    /// Sent by the UI or after queued recording receives MIDI Start/Continue.
    StartRecording,
    /// Stops active video recording and begins encoding.
    /// Sent by the UI or after MIDI timing receives Stop.
    StopRecording,
    /// Requests runtime shutdown.
    /// Sent by the UI quit command and main window quit shortcut.
    Quit,
    /// Starts randomized control transition with the provided exclusions.
    /// Sent by the UI and main window randomize shortcut.
    Randomize(Vec<String>),
    /// Requests that the control hub reload its control script.
    /// Reserved for runtime paths that need to force a YAML reload.
    ReloadControls,
    /// Resets runtime transport state and sketch timing.
    /// Sent by the UI and main window reset shortcut.
    Reset,
    /// Deletes the named snapshot slot.
    /// Sent by the snapshot UI.
    SnapshotDelete(String),
    /// Recalls the named snapshot slot.
    /// Sent by the snapshot UI and main window snapshot shortcut.
    SnapshotRecall(String),
    /// Stores the current control state into the named snapshot slot.
    /// Sent by the snapshot UI and main window snapshot shortcut.
    SnapshotStore(String),
    /// Switches to another registered sketch by name.
    /// Sent by the sketch picker and main window reload-sketch shortcut.
    SwitchSketch(String),
    /// Registers one tap-tempo tap.
    /// Sent by the UI and Space key while tap tempo mode is enabled.
    Tap,
    /// Enables or disables tap-tempo mode for the active sketch.
    /// Sent by the UI tap-tempo toggle.
    TapTempoEnabled(bool),
    /// Toggles fullscreen on the main render window.
    /// Sent by the UI and main window fullscreen shortcut.
    ToggleFullScreen,
    /// Makes the main render window visible and focused.
    /// Sent by the UI and main window focus shortcut.
    ToggleMainFocus,
    /// Applies one UI control value change to the active control hub.
    /// Sent by slider, checkbox, and select controls in the web view.
    UpdateUiControl((String, ControlValue)),
    /// Reports that the control hub finished populating controls.
    /// Sent by the control hub populated callback.
    HubPopulated,
    /// Reports that a snapshot or transition has finished.
    /// Sent by the control hub snapshot-ended callback.
    SnapshotEnded,
    /// Reports that the frame clock skipped a render tick.
    /// Emitted by the runtime tick loop for external observers.
    FrameSkipped,
    /// Reports that the active sketch changed.
    /// Emitted after successful sketch switching.
    SketchSwitched(String),
    /// Carries runtime-to-web-view events through the shared event channel.
    /// Emitted whenever runtime state needs to update the UI.
    WebView(Box<web_view::Event>),
    /// Reports that runtime shutdown has completed.
    /// Emitted by the shutdown path for the web view bridge.
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
