use std::path::PathBuf;

use xtal::prelude::*;

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "multipass",
    display_name: "Multipass",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 120.0,
    w: 900,
    h: 600,
    banks: 4,
};

pub struct MultiPassSketch {
    pass_a: PathBuf,
    pass_b: PathBuf,
    control_script_path: PathBuf,
}

impl Sketch for MultiPassSketch {
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

pub fn init() -> MultiPassSketch {
    let assets = SketchAssets::from_file(file!());

    MultiPassSketch {
        pass_a: assets.path("multipass_a.wgsl"),
        pass_b: assets.path("multipass_b.wgsl"),
        control_script_path: assets.yaml(),
    }
}
