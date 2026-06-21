use std::path::PathBuf;

use xtal::prelude::*;

use crate::constants::WORKING_BPM;

use crate::constants::{IG_HEIGHT, IG_WIDTH};

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "snd",
    display_name: "SND",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    // bpm: 134.0,
    bpm: WORKING_BPM,
    w: IG_WIDTH,
    h: IG_HEIGHT,
    banks: 12,
};

pub struct SndSketch {
    shader_path: PathBuf,
    video_path: PathBuf,
    control_script_path: PathBuf,
}

impl Sketch for SndSketch {
    fn setup(&self, graph: &mut GraphBuilder) {
        let params = graph.uniforms();
        let video = graph.video("source", self.video_path.clone());

        graph
            .render()
            .shader(self.shader_path.clone())
            .mesh(Mesh::fullscreen_quad())
            .read(params)
            .read(video)
            .to_surface();
    }

    fn control_script(&self) -> Option<PathBuf> {
        Some(self.control_script_path.clone())
    }
}

pub fn init() -> SndSketch {
    let assets = SketchAssets::from_file(file!());

    SndSketch {
        shader_path: assets.wgsl(),
        video_path: PathBuf::from("clips/Rogers Park Trees (trimmed).mp4"),
        control_script_path: assets.yaml(),
    }
}
