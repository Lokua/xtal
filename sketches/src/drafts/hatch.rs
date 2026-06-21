use std::path::PathBuf;

use xtal::prelude::*;

use crate::constants::{HD_HEIGHT, HD_WIDTH, WORKING_BPM};

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "hatch",
    display_name: "Hatch",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    // bpm: 134.0,
    bpm: WORKING_BPM,
    w: HD_WIDTH,
    h: HD_HEIGHT,
    banks: 12,
};

pub struct HatchSketch {
    hatch_shader_path: PathBuf,
    texture_shader_path: PathBuf,
    control_script_path: PathBuf,
}

impl Sketch for HatchSketch {
    fn setup(&self, graph: &mut GraphBuilder) {
        let params = graph.uniforms();
        let hatch_texture = graph.texture2d();

        graph
            .render()
            .shader(self.hatch_shader_path.clone())
            .mesh(Mesh::fullscreen_quad())
            .read(params)
            .to(hatch_texture);

        graph
            .render()
            .shader(self.texture_shader_path.clone())
            .mesh(Mesh::fullscreen_quad())
            .read(params)
            .read(hatch_texture)
            .to_surface();
    }

    fn control_script(&self) -> Option<PathBuf> {
        Some(self.control_script_path.clone())
    }
}

pub fn init() -> HatchSketch {
    let assets = SketchAssets::from_file(file!());

    HatchSketch {
        hatch_shader_path: assets.wgsl(),
        texture_shader_path: assets.path("hatch_texture.wgsl"),
        control_script_path: assets.yaml(),
    }
}
