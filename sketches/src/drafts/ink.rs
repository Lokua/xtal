use std::path::PathBuf;

use xtal::prelude::*;

use crate::constants::{HD_HEIGHT, HD_WIDTH};

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "ink",
    display_name: "Ink",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: HD_WIDTH,
    h: HD_HEIGHT,
    banks: 8,
};

pub struct InkSketch {
    pass_a: PathBuf,
    pass_b: PathBuf,
    control_script_path: PathBuf,
}

impl Sketch for InkSketch {
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

pub fn init() -> InkSketch {
    let assets = SketchAssets::from_file(file!());

    InkSketch {
        pass_a: assets.wgsl(),
        pass_b: assets.path("ink_post.wgsl"),
        control_script_path: assets.yaml(),
    }
}
