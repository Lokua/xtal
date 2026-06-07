use std::path::PathBuf;

use xtal::prelude::*;

use crate::constants::{HD_HEIGHT, HD_WIDTH};

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "video",
    display_name: "Video",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 90.0,
    w: HD_WIDTH,
    h: HD_HEIGHT,
    banks: 8,
};

pub struct VideoSketch {
    shader_path: PathBuf,
    video_path: PathBuf,
    control_script_path: PathBuf,
}

impl Sketch for VideoSketch {
    fn setup(&self, graph: &mut GraphBuilder) {
        let params = graph.uniforms();
        let video = graph.video("a", self.video_path.clone());

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

pub fn init() -> VideoSketch {
    let assets = SketchAssets::from_file(file!());

    VideoSketch {
        shader_path: assets.wgsl(),
        video_path: PathBuf::from("clips/Viaduct - Columbia Sky.mp4"),
        control_script_path: assets.yaml(),
    }
}
