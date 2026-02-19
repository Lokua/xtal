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
        let params = graph.uniforms();
        let (feedback_a, feedback_b) = graph.feedback();

        graph
            .render()
            .shader(self.shader_path.clone())
            .mesh(Mesh::fullscreen_quad())
            .read(params)
            .read(feedback_a)
            .to(feedback_b);

        graph
            .render()
            .shader(self.shader_path.clone())
            .mesh(Mesh::fullscreen_quad())
            .read(params)
            .read(feedback_b)
            .to(feedback_a);

        graph.present(feedback_a);
    }

    fn control_script(&self) -> Option<PathBuf> {
        Some(self.control_script_path.clone())
    }
}

pub fn init() -> FeedbackSketch {
    let assets = SketchAssets::from_file(file!());

    FeedbackSketch {
        shader_path: assets.wgsl(),
        control_script_path: assets.yaml(),
    }
}
