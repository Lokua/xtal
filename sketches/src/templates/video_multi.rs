use std::path::PathBuf;

use xtal::prelude::*;

use crate::constants::{HD_HEIGHT, HD_WIDTH};

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "video_multi",
    display_name: "Video Multi",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 120.0,
    w: HD_WIDTH,
    h: HD_HEIGHT,
    banks: 24,
};

pub struct VideoMultiSketch {
    shader_path: PathBuf,
    video_a_path: PathBuf,
    video_b_path: PathBuf,
    control_script_path: PathBuf,
}

impl Sketch for VideoMultiSketch {
    fn setup(&self, graph: &mut GraphBuilder) {
        let params = graph.uniforms();
        let video_a = graph.video(self.video_a_path.clone());
        let video_b = graph.video(self.video_b_path.clone());

        graph
            .render()
            .shader(self.shader_path.clone())
            .mesh(Mesh::fullscreen_quad())
            .read(params)
            .read(video_a)
            .read(video_b)
            .to_surface();
    }

    fn control_script(&self) -> Option<PathBuf> {
        Some(self.control_script_path.clone())
    }
}

pub fn init() -> VideoMultiSketch {
    let assets = SketchAssets::from_file(file!());

    VideoMultiSketch {
        shader_path: assets.wgsl(),
        video_a_path: PathBuf::from("clips/Viaduct - Columbia.mp4"),
        video_b_path: PathBuf::from("clips/Viaduct - Columbia Edge.mp4"),
        control_script_path: assets.yaml(),
    }
}
