use serde::{Deserialize, Serialize};

use super::events::RuntimeEvent;
use super::registry::RuntimeRegistry;
use crate::control::{ControlHub, ControlValue, UiControlConfig};
use crate::framework::util::HashMap;
use crate::motion::TimingSource;

pub type Sender = ipc_channel::ipc::IpcSender<Event>;
pub type Receiver = ipc_channel::ipc::IpcReceiver<Event>;

pub type Bypassed = HashMap<String, f32>;
pub type Exclusions = Vec<String>;
pub type ChannelAndController = (usize, usize);
pub type Mappings = HashMap<String, ChannelAndController>;

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum UserDir {
    Images,
    UserData,
    Videos,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum OsDir {
    Cache,
    Config,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum ControlKind {
    Checkbox,
    Select,
    Separator,
    Slider,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
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
            value: String::new(),
            disabled: false,
            options: vec![],
            min: 0.0,
            max: 1.0,
            step: 0.001,
        }
    }
}

impl Control {
    #[allow(clippy::field_reassign_with_default)]
    pub fn from_config_and_hub<T: TimingSource>(
        (ui_control, hub): (&UiControlConfig, &ControlHub<T>),
    ) -> Self {
        let mut result = Self::default();
        result.disabled = ui_control.is_disabled(&hub.ui_controls);
        result.name = ui_control.name().to_string();

        match ui_control {
            UiControlConfig::Checkbox { name, .. } => {
                result.kind = ControlKind::Checkbox;
                result.value = hub.bool(name).to_string();
            }
            UiControlConfig::Select { name, options, .. } => {
                result.kind = ControlKind::Select;
                result.value = hub.string(name);
                result.options = options.clone();
            }
            UiControlConfig::Separator { .. } => {
                result.kind = ControlKind::Separator;
            }
            UiControlConfig::Slider {
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

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SketchCatalogCategory {
    pub title: String,
    pub enabled: bool,
    pub sketches: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum Event {
    Advance,
    Alert(String),
    AverageFps(f32),
    Bpm(f32),
    CaptureFrame,
    ChangeAudioDevice(String),
    ChangeDir(UserDir),
    ChangeMidiClockPort(String),
    ChangeMidiControlInputPort(String),
    ChangeMidiControlOutputPort(String),
    ChangeOscPort(u16),
    ClearBuffer,
    CommitMappings,
    CurrentlyMapping(String),
    Encoding(bool),
    Error(String),
    Exclusions(Exclusions),
    Hrcc(bool),
    HubPopulated((Vec<Control>, Bypassed)),
    SnapshotSequenceEnabled(bool),

    /// Schema expected by xtal-ui.
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
        #[serde(default)]
        sketch_catalog: Option<Vec<SketchCatalogCategory>>,
        sketch_name: String,
        transition_time: f32,
        user_data_dir: String,
        videos_dir: String,
    },

    /// Schema expected by xtal-ui.
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
        snapshot_sequence_enabled: bool,
        tap_tempo_enabled: bool,
        exclusions: Exclusions,
    },

    Mappings(Mappings),
    MappingsEnabled(bool),
    OpenOsDir(OsDir),
    Paused(bool),
    PerfMode(bool),
    QueueRecord,
    Quit,
    Randomize(Exclusions),
    Ready,
    ReceiveDir(UserDir, String),
    RemoveMapping(String),
    Reset,
    Save(Vec<String>),
    SendMidi,
    SnapshotDelete(String),
    SnapshotEnded(Vec<Control>),
    SnapshotRecall(String),
    SnapshotStore(String),
    StartRecording,
    StopRecording,
    SwitchSketch(String),
    Tap,
    TapTempoEnabled(bool),
    ToggleFullScreen,
    ToggleGuiFocus,
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
    UpdatedControls(Vec<Control>),
}

pub fn parse_ui_message(message: &str) -> Result<Event, String> {
    serde_json::from_str(message).map_err(|err| {
        format!("invalid web-view message '{}': {}", message, err)
    })
}

pub fn to_ui_message(event: &Event) -> Result<String, String> {
    serde_json::to_string(event)
        .map_err(|err| format!("failed to serialize web-view event: {}", err))
}

pub fn map_event_to_runtime_event(event: &Event) -> Option<RuntimeEvent> {
    match event {
        Event::Advance => Some(RuntimeEvent::AdvanceSingleFrame),
        Event::CaptureFrame => Some(RuntimeEvent::CaptureFrame),
        Event::ChangeAudioDevice(name) => {
            Some(RuntimeEvent::ChangeAudioDevice(name.clone()))
        }
        Event::ChangeMidiClockPort(port) => {
            Some(RuntimeEvent::ChangeMidiClockPort(port.clone()))
        }
        Event::ChangeMidiControlInputPort(port) => {
            Some(RuntimeEvent::ChangeMidiControlInputPort(port.clone()))
        }
        Event::ChangeMidiControlOutputPort(port) => {
            Some(RuntimeEvent::ChangeMidiControlOutputPort(port.clone()))
        }
        Event::ChangeOscPort(port) => Some(RuntimeEvent::ChangeOscPort(*port)),
        Event::ClearBuffer => Some(RuntimeEvent::ClearBuffer),
        Event::CommitMappings => Some(RuntimeEvent::CommitMappings),
        Event::CurrentlyMapping(name) => {
            Some(RuntimeEvent::CurrentlyMapping(name.clone()))
        }
        Event::Exclusions(exclusions) => {
            Some(RuntimeEvent::UpdateExclusions(exclusions.clone()))
        }
        Event::Hrcc(enabled) => Some(RuntimeEvent::SetHrcc(*enabled)),
        Event::Mappings(mappings) => {
            Some(RuntimeEvent::ReceiveMappings(mappings.clone()))
        }
        Event::MappingsEnabled(enabled) => {
            Some(RuntimeEvent::SetMappingsEnabled(*enabled))
        }
        Event::OpenOsDir(kind) => Some(RuntimeEvent::OpenOsDir(kind.clone())),
        Event::Paused(paused) => Some(RuntimeEvent::Pause(*paused)),
        Event::PerfMode(enabled) => Some(RuntimeEvent::SetPerfMode(*enabled)),
        Event::QueueRecord => Some(RuntimeEvent::QueueRecord),
        Event::Randomize(exclusions) => {
            Some(RuntimeEvent::Randomize(exclusions.clone()))
        }
        Event::ReceiveDir(kind, dir) => {
            Some(RuntimeEvent::ReceiveDir(kind.clone(), dir.clone()))
        }
        Event::RemoveMapping(name) => {
            Some(RuntimeEvent::RemoveMapping(name.clone()))
        }
        Event::Reset => Some(RuntimeEvent::Reset),
        Event::Save(exclusions) => Some(RuntimeEvent::Save(exclusions.clone())),
        Event::SendMidi => Some(RuntimeEvent::SendMidi),
        Event::Quit => Some(RuntimeEvent::Quit),
        Event::SnapshotDelete(id) => {
            Some(RuntimeEvent::SnapshotDelete(id.clone()))
        }
        Event::SnapshotRecall(id) => {
            Some(RuntimeEvent::SnapshotRecall(id.clone()))
        }
        Event::SnapshotStore(id) => {
            Some(RuntimeEvent::SnapshotStore(id.clone()))
        }
        Event::StartRecording => Some(RuntimeEvent::StartRecording),
        Event::StopRecording => Some(RuntimeEvent::StopRecording),
        Event::SwitchSketch(name) => {
            Some(RuntimeEvent::SwitchSketch(name.clone()))
        }
        Event::Tap => Some(RuntimeEvent::Tap),
        Event::TapTempoEnabled(enabled) => {
            Some(RuntimeEvent::TapTempoEnabled(*enabled))
        }
        Event::TransitionTime(time) => {
            Some(RuntimeEvent::SetTransitionTime(*time))
        }
        Event::ToggleFullScreen => Some(RuntimeEvent::ToggleFullScreen),
        Event::ToggleMainFocus => Some(RuntimeEvent::ToggleMainFocus),
        Event::UpdateControlBool { name, value } => {
            Some(RuntimeEvent::UpdateUiControl((
                name.clone(),
                ControlValue::from(*value),
            )))
        }
        Event::UpdateControlFloat { name, value } => {
            Some(RuntimeEvent::UpdateUiControl((
                name.clone(),
                ControlValue::from(*value),
            )))
        }
        Event::UpdateControlString { name, value } => {
            Some(RuntimeEvent::UpdateUiControl((
                name.clone(),
                ControlValue::from(value.clone()),
            )))
        }
        _ => None,
    }
}

pub fn map_event_to_runtime_command(event: &Event) -> Option<RuntimeEvent> {
    map_event_to_runtime_event(event)
}

pub fn controls_from_hub<T: TimingSource>(hub: &ControlHub<T>) -> Vec<Control> {
    hub.ui_controls
        .config_refs()
        .values()
        .map(|config| Control::from_config_and_hub((config, hub)))
        .collect()
}

pub fn sketch_catalog_from_registry(
    registry: &RuntimeRegistry,
) -> Vec<SketchCatalogCategory> {
    registry
        .categories()
        .iter()
        .map(|category| SketchCatalogCategory {
            title: category.title.clone(),
            enabled: category.enabled,
            sketches: category.sketches.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unit_and_payload_messages() {
        let ready = parse_ui_message("\"Ready\"").unwrap();
        assert_eq!(ready, Event::Ready);

        let switch = parse_ui_message("{\"SwitchSketch\":\"demo\"}").unwrap();
        assert_eq!(switch, Event::SwitchSketch("demo".to_string()));
    }

    #[test]
    fn maps_switch_to_runtime_command() {
        let event = Event::SwitchSketch("image".to_string());
        let command = map_event_to_runtime_event(&event);
        assert_eq!(command, Some(RuntimeEvent::SwitchSketch("image".into())));
    }

    #[test]
    fn maps_perf_mode_to_runtime_command() {
        let event = Event::PerfMode(true);
        let command = map_event_to_runtime_event(&event);
        assert_eq!(command, Some(RuntimeEvent::SetPerfMode(true)));

        let hrcc = map_event_to_runtime_event(&Event::Hrcc(true));
        assert_eq!(hrcc, Some(RuntimeEvent::SetHrcc(true)));

        let mappings_enabled =
            map_event_to_runtime_event(&Event::MappingsEnabled(false));
        assert_eq!(
            mappings_enabled,
            Some(RuntimeEvent::SetMappingsEnabled(false))
        );
    }

    #[test]
    fn maps_window_focus_commands() {
        let fullscreen = map_event_to_runtime_event(&Event::ToggleFullScreen);
        assert_eq!(fullscreen, Some(RuntimeEvent::ToggleFullScreen));

        let main_focus = map_event_to_runtime_event(&Event::ToggleMainFocus);
        assert_eq!(main_focus, Some(RuntimeEvent::ToggleMainFocus));
    }

    #[test]
    fn maps_snapshot_commands() {
        let store =
            map_event_to_runtime_event(&Event::SnapshotStore("1".into()));
        assert_eq!(store, Some(RuntimeEvent::SnapshotStore("1".into())));

        let recall =
            map_event_to_runtime_event(&Event::SnapshotRecall("2".into()));
        assert_eq!(recall, Some(RuntimeEvent::SnapshotRecall("2".into())));

        let delete =
            map_event_to_runtime_event(&Event::SnapshotDelete("3".into()));
        assert_eq!(delete, Some(RuntimeEvent::SnapshotDelete("3".into())));
    }

    #[test]
    fn maps_randomize_reset_and_transition_time_commands() {
        let randomize = map_event_to_runtime_event(&Event::Randomize(vec![
            "foo".into(),
            "bar".into(),
        ]));
        assert_eq!(
            randomize,
            Some(RuntimeEvent::Randomize(vec!["foo".into(), "bar".into()]))
        );

        let reset = map_event_to_runtime_event(&Event::Reset);
        assert_eq!(reset, Some(RuntimeEvent::Reset));

        let transition =
            map_event_to_runtime_event(&Event::TransitionTime(2.5));
        assert_eq!(transition, Some(RuntimeEvent::SetTransitionTime(2.5)));

        let tap = map_event_to_runtime_event(&Event::Tap);
        assert_eq!(tap, Some(RuntimeEvent::Tap));

        let tap_tempo =
            map_event_to_runtime_event(&Event::TapTempoEnabled(true));
        assert_eq!(tap_tempo, Some(RuntimeEvent::TapTempoEnabled(true)));
    }

    #[test]
    fn maps_remaining_ui_bridge_commands() {
        assert_eq!(
            map_event_to_runtime_event(&Event::Save(vec!["foo".into()])),
            Some(RuntimeEvent::Save(vec!["foo".into()]))
        );
        assert_eq!(
            map_event_to_runtime_event(&Event::CommitMappings),
            Some(RuntimeEvent::CommitMappings)
        );
        assert_eq!(
            map_event_to_runtime_event(&Event::CurrentlyMapping("ax".into())),
            Some(RuntimeEvent::CurrentlyMapping("ax".into()))
        );
        assert_eq!(
            map_event_to_runtime_event(&Event::Exclusions(vec![
                "foo".into(),
                "bar".into()
            ])),
            Some(RuntimeEvent::UpdateExclusions(vec![
                "foo".into(),
                "bar".into()
            ]))
        );
        assert_eq!(
            map_event_to_runtime_event(&Event::RemoveMapping("ax".into())),
            Some(RuntimeEvent::RemoveMapping("ax".into()))
        );
        assert_eq!(
            map_event_to_runtime_event(&Event::SendMidi),
            Some(RuntimeEvent::SendMidi)
        );
        assert_eq!(
            map_event_to_runtime_event(&Event::ChangeAudioDevice(
                "Built-in".into()
            )),
            Some(RuntimeEvent::ChangeAudioDevice("Built-in".into()))
        );
        assert_eq!(
            map_event_to_runtime_event(&Event::ChangeMidiClockPort(
                "clock".into()
            )),
            Some(RuntimeEvent::ChangeMidiClockPort("clock".into()))
        );
        assert_eq!(
            map_event_to_runtime_event(&Event::ChangeMidiControlInputPort(
                "in".into()
            )),
            Some(RuntimeEvent::ChangeMidiControlInputPort("in".into()))
        );
        assert_eq!(
            map_event_to_runtime_event(&Event::ChangeMidiControlOutputPort(
                "out".into()
            )),
            Some(RuntimeEvent::ChangeMidiControlOutputPort("out".into()))
        );
        assert_eq!(
            map_event_to_runtime_event(&Event::ChangeOscPort(9000)),
            Some(RuntimeEvent::ChangeOscPort(9000))
        );
        assert_eq!(
            map_event_to_runtime_event(&Event::ReceiveDir(
                UserDir::Images,
                "/tmp/images".into()
            )),
            Some(RuntimeEvent::ReceiveDir(
                UserDir::Images,
                "/tmp/images".into()
            ))
        );
        assert_eq!(
            map_event_to_runtime_event(&Event::OpenOsDir(OsDir::Cache)),
            Some(RuntimeEvent::OpenOsDir(OsDir::Cache))
        );
        assert_eq!(
            map_event_to_runtime_event(&Event::CaptureFrame),
            Some(RuntimeEvent::CaptureFrame)
        );
        assert_eq!(
            map_event_to_runtime_event(&Event::QueueRecord),
            Some(RuntimeEvent::QueueRecord)
        );
        assert_eq!(
            map_event_to_runtime_event(&Event::StartRecording),
            Some(RuntimeEvent::StartRecording)
        );
        assert_eq!(
            map_event_to_runtime_event(&Event::StopRecording),
            Some(RuntimeEvent::StopRecording)
        );
        assert_eq!(
            map_event_to_runtime_event(&Event::ClearBuffer),
            Some(RuntimeEvent::ClearBuffer)
        );
    }

    #[test]
    fn maps_control_updates_to_single_runtime_variant() {
        assert_eq!(
            map_event_to_runtime_event(&Event::UpdateControlBool {
                name: "enabled".into(),
                value: true,
            }),
            Some(RuntimeEvent::UpdateUiControl((
                "enabled".into(),
                ControlValue::Bool(true),
            )))
        );

        assert_eq!(
            map_event_to_runtime_event(&Event::UpdateControlFloat {
                name: "amount".into(),
                value: 0.75,
            }),
            Some(RuntimeEvent::UpdateUiControl((
                "amount".into(),
                ControlValue::Float(0.75),
            )))
        );

        assert_eq!(
            map_event_to_runtime_event(&Event::UpdateControlString {
                name: "mode".into(),
                value: "fast".into(),
            }),
            Some(RuntimeEvent::UpdateUiControl((
                "mode".into(),
                ControlValue::String("fast".into()),
            )))
        );
    }

    #[test]
    fn outbound_events_do_not_map_back_into_runtime() {
        assert_eq!(
            map_event_to_runtime_event(&Event::HubPopulated((
                vec![],
                HashMap::default(),
            ))),
            None
        );
        assert_eq!(
            map_event_to_runtime_event(&Event::UpdatedControls(vec![])),
            None
        );
        assert_eq!(
            map_event_to_runtime_event(&Event::SnapshotEnded(vec![])),
            None
        );
    }
}
