use std::path::PathBuf;

use xtal::prelude::*;

use crate::constants::{HD_HEIGHT, HD_WIDTH};

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "gyroid",
    display_name: "Gyroid",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: HD_WIDTH,
    h: HD_HEIGHT,
    banks: 12,
};

pub struct GyroidSketch {
    pass_a: PathBuf,
    pass_b: PathBuf,
    control_script_path: PathBuf,
}

impl Sketch for GyroidSketch {
    fn setup(&self, graph: &mut GraphBuilder) {
        let params = graph.uniforms();
        let rt0 = graph.texture2d();

        graph
            .render()
            .shader(self.pass_a.clone())
            .mesh(Mesh::fullscreen_quad())
            .read(params)
            .to(rt0);

        graph
            .render()
            .shader(self.pass_b.clone())
            .mesh(Mesh::fullscreen_quad())
            .read(params)
            .read(rt0)
            .to_surface();
    }

    fn control_script(&self) -> Option<PathBuf> {
        Some(self.control_script_path.clone())
    }
}

pub fn init() -> GyroidSketch {
    let assets = SketchAssets::from_file(file!());

    GyroidSketch {
        pass_a: assets.wgsl(),
        pass_b: assets.path("gyroid_post.wgsl"),
        control_script_path: assets.yaml(),
    }
}
