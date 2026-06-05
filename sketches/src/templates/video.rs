use std::path::PathBuf;

use xtal::prelude::*;

use crate::constants::{IG_HEIGHT, IG_WIDTH};

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "video",
    display_name: "Video",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 120.0,
    w: IG_WIDTH,
    h: IG_HEIGHT,
    banks: 4,
};

pub struct VideoSketch {
    shader_path: PathBuf,
    video_path: PathBuf,
}

impl Sketch for VideoSketch {
    fn setup(&self, graph: &mut GraphBuilder) {
        let params = graph.uniforms();
        let video = graph.video(self.video_path.clone());

        graph
            .render()
            .shader(self.shader_path.clone())
            .mesh(Mesh::fullscreen_quad())
            .read(params)
            .read(video)
            .to_surface();
    }
}

pub fn init() -> VideoSketch {
    let assets = SketchAssets::from_file(file!());

    VideoSketch {
        shader_path: assets.wgsl(),
        video_path: PathBuf::from("clips/Rogers Park Trees.mp4"),
    }
}
