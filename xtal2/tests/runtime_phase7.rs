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
        web_view::map_event_to_runtime_command(&event),
        Some(RuntimeCommand::SwitchSketch("phase7_test".to_string()))
    );
}

#[test]
fn web_view_command_mapping_supports_perf_mode_and_window_events() {
    let perf = web_view::parse_ui_message("{\"PerfMode\":true}")
        .expect("parse perf mode message");
    assert_eq!(
        web_view::map_event_to_runtime_command(&perf),
        Some(RuntimeCommand::SetPerfMode(true))
    );

    let fullscreen = web_view::parse_ui_message("\"ToggleFullScreen\"")
        .expect("parse toggle fullscreen message");
    assert_eq!(
        web_view::map_event_to_runtime_command(&fullscreen),
        Some(RuntimeCommand::ToggleFullScreen)
    );

    let main_focus = web_view::parse_ui_message("\"ToggleMainFocus\"")
        .expect("parse toggle main focus message");
    assert_eq!(
        web_view::map_event_to_runtime_command(&main_focus),
        Some(RuntimeCommand::ToggleMainFocus)
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
