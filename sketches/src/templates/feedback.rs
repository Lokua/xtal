use std::path::PathBuf;

use xtal::prelude::*;

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "feedback",
    display_name: "Feedback",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 120.0,
    w: 960,
    h: 540,
    banks: 4,
};

pub struct FeedbackSketch {
    shader_path: PathBuf,
    control_script_path: PathBuf,
}

impl Sketch for FeedbackSketch {
    fn setup(&self, graph: &mut GraphBuilder) {
        graph.uniforms("params");
        graph.texture2d("feedback_a");
        graph.texture2d("feedback_b");

        graph
            .render("feedback_step_a")
            .shader(self.shader_path.clone())
            .read("params")
            .read("feedback_a")
            .write("feedback_b")
            .add();

        graph
            .render("feedback_step_b")
            .shader(self.shader_path.clone())
            .read("params")
            .read("feedback_b")
            .write("feedback_a")
            .add();

        graph.present("feedback_a");
    }

    fn control_script(&self) -> Option<PathBuf> {
        Some(self.control_script_path.clone())
    }
}

pub fn init() -> FeedbackSketch {
    let assets = SketchAssets::from_manifest_file(env!("CARGO_MANIFEST_DIR"), file!());

    FeedbackSketch {
        shader_path: assets.wgsl(),
        control_script_path: assets.yaml(),
    }
}
