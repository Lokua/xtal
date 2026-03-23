use std::path::PathBuf;

use xtal::prelude::*;

use crate::constants::{HD_HEIGHT, HD_WIDTH};

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "blob",
    display_name: "Blob",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: HD_WIDTH,
    h: HD_HEIGHT,
    banks: 12,
};

pub struct BlobSketch {
    shader_path: PathBuf,
    control_script_path: PathBuf,
}

impl Sketch for BlobSketch {
    fn setup(&self, graph: &mut GraphBuilder) {
        let params = graph.uniforms();
        let (ping, pong) = graph.feedback();

        graph
            .render()
            .shader(self.shader_path.clone())
            .mesh(Mesh::fullscreen_quad())
            .read(params)
            .read(ping)
            .to(pong);

        graph
            .render()
            .shader(self.shader_path.clone())
            .mesh(Mesh::fullscreen_quad())
            .read(params)
            .read(pong)
            .to(ping);

        graph.present(ping);
    }

    fn control_script(&self) -> Option<PathBuf> {
        Some(self.control_script_path.clone())
    }
}

pub fn init() -> BlobSketch {
    let assets = SketchAssets::from_file(file!());

    BlobSketch {
        shader_path: assets.wgsl(),
        control_script_path: assets.yaml(),
    }
}
