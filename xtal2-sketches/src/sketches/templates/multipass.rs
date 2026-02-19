use std::path::PathBuf;

use xtal2::prelude::*;

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
        graph.uniforms("params");
        graph.texture2d("rt0");

        graph
            .render("pass_a")
            .shader(self.pass_a.clone())
            .read("params")
            .write("rt0")
            .add();

        graph
            .render("pass_b")
            .shader(self.pass_b.clone())
            .read("params")
            .read("rt0")
            .write("surface")
            .add();

        graph.present("surface");
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
