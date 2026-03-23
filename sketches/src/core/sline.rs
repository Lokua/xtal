use std::path::PathBuf;

use xtal::prelude::*;

use crate::constants::{HD_HEIGHT, HD_WIDTH};

const PROCEDURAL_VERTICES: usize = 300_000;

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "sline",
    display_name: "Spiral | Lines Version",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 120.0,
    w: HD_WIDTH,
    h: HD_HEIGHT,
    banks: 12,
};

pub struct SlineSketch {
    shader_path: PathBuf,
    control_script_path: PathBuf,
}

impl Sketch for SlineSketch {
    fn setup(&self, graph: &mut GraphBuilder) {
        let params = graph.uniforms();
        let mesh = Mesh::positions2d(vec![[0.0, 0.0]; PROCEDURAL_VERTICES]);

        graph
            .render()
            .shader(self.shader_path.clone())
            .mesh(mesh)
            .read(params)
            .to_surface();
    }

    fn control_script(&self) -> Option<PathBuf> {
        Some(self.control_script_path.clone())
    }
}

pub fn init() -> SlineSketch {
    let assets = SketchAssets::from_file(file!());

    SlineSketch {
        shader_path: assets.wgsl(),
        control_script_path: assets.yaml(),
    }
}
