use std::path::PathBuf;

use xtal2::control::{ControlCollection, ControlHub, ControlValue};
use xtal2::framework::frame_controller;
use xtal2::motion::{Bpm, Timing};

fn hub_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../xtal2-sketches/src/sketches/main/grid_splash_bw.yaml")
}

#[test]
fn debug_snapshot_grid_transition_progression() {
    let path = hub_path();
    assert!(path.exists(), "missing test yaml at {}", path.display());

    frame_controller::set_fps(60.0);
    frame_controller::set_paused(false);
    frame_controller::set_frame_count(0);

    let timing = Timing::frame(Bpm::new(134.0));
    let mut hub = ControlHub::from_path(path, timing);
    hub.set_transition_time(4.0);

    // Snapshot A defaults.
    hub.take_snapshot("a");

    // Snapshot B with obvious deltas.
    hub.ui_controls.set("ab_mix", ControlValue::Float(1.0));
    hub.ui_controls.set("a_freq", ControlValue::Float(1.0));
    hub.ui_controls.set("feedback", ControlValue::Float(1.0));
    hub.take_snapshot("b");

    // Back to A values, then recall B.
    hub.ui_controls.set("ab_mix", ControlValue::Float(0.0));
    hub.ui_controls.set("a_freq", ControlValue::Float(0.0));
    hub.ui_controls.set("feedback", ControlValue::Float(0.0));

    hub.recall_snapshot("b").unwrap();

    let sample = |hub: &ControlHub<Timing>, frame: u32| -> (f32, f32, f32) {
        frame_controller::set_frame_count(frame);
        (hub.get("ab_mix"), hub.get("a_freq"), hub.get("feedback"))
    };

    let f0 = sample(&hub, 0);
    let f10 = sample(&hub, 10);
    let f30 = sample(&hub, 30);
    let f60 = sample(&hub, 60);
    let f119 = sample(&hub, 119);

    // End transition and apply terminal values.
    frame_controller::set_frame_count(120);
    hub.update();
    let fend = sample(&hub, 120);

    eprintln!(
        "f0={:?} f10={:?} f30={:?} f60={:?} f119={:?} fend={:?}",
        f0, f10, f30, f60, f119, fend
    );

    // A few sanity checks: should move toward 1.0 and end at/near 1.0.
    assert!(f10.0 >= f0.0);
    assert!(f30.0 >= f10.0);
    assert!(fend.0 > 0.9);
}
