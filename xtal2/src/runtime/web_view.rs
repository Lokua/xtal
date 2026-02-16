use serde::{Deserialize, Serialize};

use super::events::RuntimeCommand;
use super::registry::RuntimeRegistry;
use crate::control::{ControlHub, UiControlConfig};
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

pub fn map_event_to_runtime_command(event: &Event) -> Option<RuntimeCommand> {
    match event {
        Event::Advance => Some(RuntimeCommand::AdvanceSingleFrame),
        Event::Paused(paused) => Some(RuntimeCommand::Pause(*paused)),
        Event::Quit => Some(RuntimeCommand::Quit),
        Event::SwitchSketch(name) => {
            Some(RuntimeCommand::SwitchSketch(name.clone()))
        }
        Event::UpdateControlBool { name, value } => {
            Some(RuntimeCommand::UpdateControlBool {
                name: name.clone(),
                value: *value,
            })
        }
        Event::UpdateControlFloat { name, value } => {
            Some(RuntimeCommand::UpdateControlFloat {
                name: name.clone(),
                value: *value,
            })
        }
        Event::UpdateControlString { name, value } => {
            Some(RuntimeCommand::UpdateControlString {
                name: name.clone(),
                value: value.clone(),
            })
        }
        _ => None,
    }
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
        let command = map_event_to_runtime_command(&event);
        assert_eq!(command, Some(RuntimeCommand::SwitchSketch("image".into())));
    }
}
