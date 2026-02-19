use xtal2::graph::GraphBuilder;
use xtal2::prelude::*;

struct TestSketch;

impl Sketch for TestSketch {
    fn setup(&self, _graph: &mut GraphBuilder) {}
}

static TEST_CONFIG: SketchConfig = SketchConfig {
    name: "phase7_test",
    display_name: "Phase 7 Test",
    fps: 60.0,
    bpm: 120.0,
    w: 640,
    h: 480,
    banks: 4,
};

#[test]
fn web_view_json_parsing_and_command_mapping_support_switch_sketch() {
    let event =
        web_view::parse_ui_message("{\"SwitchSketch\":\"phase7_test\"}")
            .expect("parse switch sketch message");

    assert_eq!(
        web_view::map_event_to_runtime_event(&event),
        Some(RuntimeEvent::SwitchSketch("phase7_test".to_string()))
    );
}

#[test]
fn web_view_command_mapping_supports_perf_mode_and_window_events() {
    let perf = web_view::parse_ui_message("{\"PerfMode\":true}")
        .expect("parse perf mode message");
    assert_eq!(
        web_view::map_event_to_runtime_event(&perf),
        Some(RuntimeEvent::SetPerfMode(true))
    );

    let fullscreen = web_view::parse_ui_message("\"ToggleFullScreen\"")
        .expect("parse toggle fullscreen message");
    assert_eq!(
        web_view::map_event_to_runtime_event(&fullscreen),
        Some(RuntimeEvent::ToggleFullScreen)
    );

    let main_focus = web_view::parse_ui_message("\"ToggleMainFocus\"")
        .expect("parse toggle main focus message");
    assert_eq!(
        web_view::map_event_to_runtime_event(&main_focus),
        Some(RuntimeEvent::ToggleMainFocus)
    );
}

#[test]
fn web_view_command_mapping_supports_phase1_actions() {
    let randomize =
        web_view::parse_ui_message("{\"Randomize\":[\"foo\",\"bar\"]}")
            .expect("parse randomize message");
    assert_eq!(
        web_view::map_event_to_runtime_event(&randomize),
        Some(RuntimeEvent::Randomize(vec!["foo".into(), "bar".into()]))
    );

    let reset =
        web_view::parse_ui_message("\"Reset\"").expect("parse reset message");
    assert_eq!(
        web_view::map_event_to_runtime_event(&reset),
        Some(RuntimeEvent::Reset)
    );

    let transition = web_view::parse_ui_message("{\"TransitionTime\":3.0}")
        .expect("parse transition time message");
    assert_eq!(
        web_view::map_event_to_runtime_event(&transition),
        Some(RuntimeEvent::SetTransitionTime(3.0))
    );

    let store = web_view::parse_ui_message("{\"SnapshotStore\":\"1\"}")
        .expect("parse snapshot store message");
    assert_eq!(
        web_view::map_event_to_runtime_event(&store),
        Some(RuntimeEvent::SnapshotStore("1".into()))
    );

    let tap = web_view::parse_ui_message("\"Tap\"").expect("parse tap message");
    assert_eq!(
        web_view::map_event_to_runtime_event(&tap),
        Some(RuntimeEvent::Tap)
    );

    let tap_tempo = web_view::parse_ui_message("{\"TapTempoEnabled\":true}")
        .expect("parse tap tempo enabled message");
    assert_eq!(
        web_view::map_event_to_runtime_event(&tap_tempo),
        Some(RuntimeEvent::TapTempoEnabled(true))
    );

    let hrcc =
        web_view::parse_ui_message("{\"Hrcc\":true}").expect("parse hrcc");
    assert_eq!(
        web_view::map_event_to_runtime_event(&hrcc),
        Some(RuntimeEvent::SetHrcc(true))
    );

    let mappings_enabled =
        web_view::parse_ui_message("{\"MappingsEnabled\":false}")
            .expect("parse mappings enabled");
    assert_eq!(
        web_view::map_event_to_runtime_event(&mappings_enabled),
        Some(RuntimeEvent::SetMappingsEnabled(false))
    );
}

#[test]
fn web_view_command_mapping_supports_remaining_phase1_actions() {
    let save =
        web_view::parse_ui_message("{\"Save\":[\"foo\"]}").expect("parse save");
    assert_eq!(
        web_view::map_event_to_runtime_event(&save),
        Some(RuntimeEvent::Save(vec!["foo".into()]))
    );

    let receive_dir = web_view::parse_ui_message(
        "{\"ReceiveDir\":[\"Images\",\"/tmp/images\"]}",
    )
    .expect("parse receive dir");
    assert_eq!(
        web_view::map_event_to_runtime_event(&receive_dir),
        Some(RuntimeEvent::ReceiveDir(
            web_view::UserDir::Images,
            "/tmp/images".into()
        ))
    );

    let currently_mapping =
        web_view::parse_ui_message("{\"CurrentlyMapping\":\"ax\"}")
            .expect("parse currently mapping");
    assert_eq!(
        web_view::map_event_to_runtime_event(&currently_mapping),
        Some(RuntimeEvent::CurrentlyMapping("ax".into()))
    );

    let exclusions =
        web_view::parse_ui_message("{\"Exclusions\":[\"foo\",\"bar\"]}")
            .expect("parse exclusions");
    assert_eq!(
        web_view::map_event_to_runtime_event(&exclusions),
        Some(RuntimeEvent::UpdateExclusions(vec![
            "foo".into(),
            "bar".into()
        ]))
    );

    let commit =
        web_view::parse_ui_message("\"CommitMappings\"").expect("parse commit");
    assert_eq!(
        web_view::map_event_to_runtime_event(&commit),
        Some(RuntimeEvent::CommitMappings)
    );

    let remove_mapping =
        web_view::parse_ui_message("{\"RemoveMapping\":\"ax\"}")
            .expect("parse remove mapping");
    assert_eq!(
        web_view::map_event_to_runtime_event(&remove_mapping),
        Some(RuntimeEvent::RemoveMapping("ax".into()))
    );

    let send_midi =
        web_view::parse_ui_message("\"SendMidi\"").expect("parse send midi");
    assert_eq!(
        web_view::map_event_to_runtime_event(&send_midi),
        Some(RuntimeEvent::SendMidi)
    );

    let change_audio =
        web_view::parse_ui_message("{\"ChangeAudioDevice\":\"Built-in\"}")
            .expect("parse change audio");
    assert_eq!(
        web_view::map_event_to_runtime_event(&change_audio),
        Some(RuntimeEvent::ChangeAudioDevice("Built-in".into()))
    );

    let change_osc = web_view::parse_ui_message("{\"ChangeOscPort\":9000}")
        .expect("parse change osc");
    assert_eq!(
        web_view::map_event_to_runtime_event(&change_osc),
        Some(RuntimeEvent::ChangeOscPort(9000))
    );

    let open_os_dir = web_view::parse_ui_message("{\"OpenOsDir\":\"Cache\"}")
        .expect("parse open os dir");
    assert_eq!(
        web_view::map_event_to_runtime_event(&open_os_dir),
        Some(RuntimeEvent::OpenOsDir(web_view::OsDir::Cache))
    );

    let capture =
        web_view::parse_ui_message("\"CaptureFrame\"").expect("parse capture");
    assert_eq!(
        web_view::map_event_to_runtime_event(&capture),
        Some(RuntimeEvent::CaptureFrame)
    );

    let queue =
        web_view::parse_ui_message("\"QueueRecord\"").expect("parse queue");
    assert_eq!(
        web_view::map_event_to_runtime_event(&queue),
        Some(RuntimeEvent::QueueRecord)
    );

    let start = web_view::parse_ui_message("\"StartRecording\"")
        .expect("parse start recording");
    assert_eq!(
        web_view::map_event_to_runtime_event(&start),
        Some(RuntimeEvent::StartRecording)
    );

    let stop = web_view::parse_ui_message("\"StopRecording\"")
        .expect("parse stop recording");
    assert_eq!(
        web_view::map_event_to_runtime_event(&stop),
        Some(RuntimeEvent::StopRecording)
    );

    let clear = web_view::parse_ui_message("\"ClearBuffer\"")
        .expect("parse clear buffer");
    assert_eq!(
        web_view::map_event_to_runtime_event(&clear),
        Some(RuntimeEvent::ClearBuffer)
    );
}

#[test]
fn web_view_init_serializes_optional_sketch_catalog_in_camel_case() {
    let event = web_view::Event::Init {
        audio_device: String::new(),
        audio_devices: vec![],
        hrcc: false,
        images_dir: String::new(),
        is_light_theme: true,
        mappings_enabled: false,
        midi_clock_port: String::new(),
        midi_input_port: String::new(),
        midi_output_port: String::new(),
        midi_input_ports: vec![],
        midi_output_ports: vec![],
        osc_port: 0,
        sketch_names: vec!["demo".to_string()],
        sketch_catalog: Some(vec![web_view::SketchCatalogCategory {
            title: "Main".to_string(),
            enabled: true,
            sketches: vec!["demo".to_string()],
        }]),
        sketch_name: "demo".to_string(),
        transition_time: 4.0,
        user_data_dir: String::new(),
        videos_dir: String::new(),
    };

    let json = web_view::to_ui_message(&event).expect("serialize init event");
    assert!(json.contains("\"sketchCatalog\""));
    assert!(json.contains("\"title\":\"Main\""));
}

#[test]
fn web_view_catalog_helper_uses_registry_categories() {
    let mut registry = RuntimeRegistry::new();
    registry
        .register(&TEST_CONFIG, || Box::new(TestSketch))
        .expect("register sketch");
    registry
        .define_category("Main", true, vec![TEST_CONFIG.name.to_string()])
        .expect("define category");

    let catalog = web_view::sketch_catalog_from_registry(&registry);
    assert_eq!(catalog.len(), 1);
    assert_eq!(catalog[0].title, "Main");
    assert!(catalog[0].enabled);
    assert_eq!(catalog[0].sketches, vec!["phase7_test"]);
}
