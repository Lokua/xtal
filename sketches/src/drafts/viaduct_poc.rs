use std::path::PathBuf;

use xtal::prelude::*;

use crate::constants::{HD_HEIGHT, HD_WIDTH, WORKING_BPM};

const VIDEO_FILES: [&str; 5] = [
    "clips/Viaduct - Columbia Edge.mp4",
    "clips/Viaduct - Columbia Sky.mp4",
    "clips/Viaduct - Columbia.mp4",
    "clips/Viaduct - Pratt & Glenwood.mp4",
    "clips/Viaduct - South Evanston.mp4",
];

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "viaduct_poc",
    display_name: "Viaduct POC",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    // bpm: 134.0,
    bpm: WORKING_BPM,
    w: HD_WIDTH,
    h: HD_HEIGHT,
    banks: 12,
};

pub struct ViaductPocSketch {
    shader_path: PathBuf,
    video_paths: Vec<PathBuf>,
    control_script_path: PathBuf,
}

impl Sketch for ViaductPocSketch {
    fn setup(&self, graph: &mut GraphBuilder) {
        let params = graph.uniforms();
        let videos = self
            .video_paths
            .iter()
            .enumerate()
            .map(|(index, path)| graph.video(index.to_string(), path.clone()))
            .collect::<Vec<_>>();

        let mut render = graph
            .render()
            .shader(self.shader_path.clone())
            .mesh(Mesh::fullscreen_quad())
            .read(params);

        for video in videos {
            render = render.read(video);
        }

        render.to_surface();
    }

    fn control_script(&self) -> Option<PathBuf> {
        Some(self.control_script_path.clone())
    }
}

pub fn init() -> ViaductPocSketch {
    let assets = SketchAssets::from_file(file!());

    ViaductPocSketch {
        shader_path: assets.wgsl(),
        video_paths: VIDEO_FILES.iter().map(PathBuf::from).collect(),
        control_script_path: assets.yaml(),
    }
}
