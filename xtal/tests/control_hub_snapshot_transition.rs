use std::path::PathBuf;

use xtal::control::{ControlCollection, ControlHub, ControlValue};
use xtal::motion::{Bpm, Timing};
use xtal::time::frame_clock;

fn hub_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../sketches/src/drafts/grid_splash_bw.yaml")
}

#[test]
fn debug_snapshot_grid_transition_progression() {
    const FPS: f32 = 60.0;
    const BPM: f32 = 134.0;
    const TRANSITION_BEATS: f32 = 4.0;

    let path = hub_path();
    assert!(path.exists(), "missing test yaml at {}", path.display());

    frame_clock::set_fps(FPS);
    frame_clock::set_paused(true);
    frame_clock::set_frame_count(0);
    frame_clock::set_elapsed_seconds(0.0);

    let timing = Timing::frame(Bpm::new(BPM));
    let mut hub = ControlHub::from_path(path, timing);
    hub.set_transition_time(TRANSITION_BEATS);

    // Snapshot A defaults.
    hub.take_snapshot("a");

    // Snapshot B with obvious deltas.
    hub.ui_controls.set("colorize", ControlValue::Float(1.0));
    hub.ui_controls
        .set("norm_color_disp", ControlValue::Float(1.0));
    hub.ui_controls.set("feedback", ControlValue::Float(1.0));
    hub.take_snapshot("b");

    // Back to A values, then recall B.
    hub.ui_controls.set("colorize", ControlValue::Float(0.0));
    hub.ui_controls
        .set("norm_color_disp", ControlValue::Float(0.0));
    hub.ui_controls.set("feedback", ControlValue::Float(0.0));

    hub.recall_snapshot("b").unwrap();

    let sample = |hub: &ControlHub<Timing>, frame: u32| -> (f32, f32, f32) {
        frame_clock::set_frame_count(frame);
        frame_clock::set_elapsed_seconds(frame as f32 / FPS);
        (
            hub.get("colorize"),
            hub.get("norm_color_disp"),
            hub.get("feedback"),
        )
    };

    let end_frame =
        ((TRANSITION_BEATS * FPS * 60.0 / BPM).ceil() as u32).max(1);
    let f0 = sample(&hub, 0);
    let f10 = sample(&hub, 10);
    let f30 = sample(&hub, 30);
    let f60 = sample(&hub, 60);
    let f_before_end = sample(&hub, end_frame - 1);

    // End transition and apply terminal values.
    frame_clock::set_frame_count(end_frame);
    frame_clock::set_elapsed_seconds(end_frame as f32 / FPS);
    hub.update();
    let fend = sample(&hub, end_frame);

    eprintln!(
        "f0={:?} f10={:?} f30={:?} f60={:?} f_before_end={:?} fend={:?}",
        f0, f10, f30, f60, f_before_end, fend
    );

    // A few sanity checks: should move toward 1.0 and end at/near 1.0.
    assert!(f10.0 >= f0.0);
    assert!(f30.0 >= f10.0);
    assert!(f60.0 >= f30.0);
    assert!(f_before_end.0 >= f60.0);
    assert!((fend.0 - 1.0).abs() < 0.001);
    assert!((fend.1 - 1.0).abs() < 0.001);
    assert!((fend.2 - 1.0).abs() < 0.001);
}
