use xtal2::graph::GraphBuilder;
use xtal2::prelude::*;

struct TestSketch;

impl Sketch for TestSketch {
    fn setup(&self, _graph: &mut GraphBuilder) {}
}

static TEST_CONFIG: SketchConfig = SketchConfig {
    name: "phase1_test",
    display_name: "Phase 1 Test",
    fps: 60.0,
    bpm: 120.0,
    w: 640,
    h: 480,
    banks: 4,
};

#[test]
fn registry_supports_categories_with_enabled_flag() {
    let mut registry = RuntimeRegistry::new();
    registry
        .register(&TEST_CONFIG, || Box::new(TestSketch))
        .expect("register sketch");

    registry
        .define_category("Main", true, vec![TEST_CONFIG.name.to_string()])
        .expect("define category");

    let categories = registry.categories();
    assert_eq!(categories.len(), 1);
    assert_eq!(categories[0].title, "Main");
    assert!(categories[0].enabled);
    assert_eq!(categories[0].sketches, vec!["phase1_test"]);
}

#[test]
fn registry_rejects_duplicate_sketch_names() {
    let mut registry = RuntimeRegistry::new();
    registry
        .register(&TEST_CONFIG, || Box::new(TestSketch))
        .expect("first register");

    let err = registry
        .register(&TEST_CONFIG, || Box::new(TestSketch))
        .expect_err("duplicate sketch must fail");

    assert!(err.contains("duplicate sketch"));
}

#[test]
fn runtime_command_and_event_channels_round_trip() {
    let (command_tx, command_rx) = command_channel();
    let (event_tx, event_rx) = event_channel();

    command_tx
        .send(RuntimeEvent::SwitchSketch("phase1_test".to_string()))
        .expect("send command");

    event_tx
        .send(RuntimeEvent::SketchSwitched("phase1_test".to_string()))
        .expect("send event");
    event_tx
        .send(RuntimeEvent::WebView(web_view::Event::AverageFps(58.5)))
        .expect("send average fps event");

    assert_eq!(
        command_rx.recv().expect("recv command"),
        RuntimeEvent::SwitchSketch("phase1_test".to_string())
    );

    assert_eq!(
        event_rx.recv().expect("recv event"),
        RuntimeEvent::SketchSwitched("phase1_test".to_string())
    );
    assert_eq!(
        event_rx.recv().expect("recv average fps event"),
        RuntimeEvent::WebView(web_view::Event::AverageFps(58.5))
    );
}
