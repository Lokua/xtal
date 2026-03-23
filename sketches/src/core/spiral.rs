use std::path::PathBuf;

use xtal::prelude::*;

use crate::constants::{HD_HEIGHT, HD_WIDTH};

// auto_spiral used a very dense procedural draw count at runtime.
// Keep this high so the rendered structure reads as a stable spiral
// instead of sparse rotating points.
const PROCEDURAL_VERTICES: usize = 32_000_000;

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "spiral",
    display_name: "Spiral",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: HD_WIDTH,
    h: HD_HEIGHT,
    banks: 12,
};

pub struct SpiralSketch {
    shader_path: PathBuf,
    control_script_path: PathBuf,
}

impl Sketch for SpiralSketch {
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

pub fn init() -> SpiralSketch {
    let assets = SketchAssets::from_file(file!());

    SpiralSketch {
        shader_path: assets.wgsl(),
        control_script_path: assets.yaml(),
    }
}
