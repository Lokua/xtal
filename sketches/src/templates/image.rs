use std::path::PathBuf;

use xtal::prelude::*;

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "image",
    display_name: "Image",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 120.0,
    w: 700,
    h: 700,
    banks: 4,
};

pub struct ImageSketch {
    shader_path: PathBuf,
    image_path: PathBuf,
    control_script_path: PathBuf,
}

impl Sketch for ImageSketch {
    fn setup(&self, graph: &mut GraphBuilder) {
        graph.uniforms("params");
        graph.image("img0", self.image_path.clone());

        graph
            .render("image_pass")
            .shader(self.shader_path.clone())
            .read("params")
            .read("img0")
            .write("surface")
            .add();

        graph.present("surface");
    }

    fn control_script(&self) -> Option<PathBuf> {
        Some(self.control_script_path.clone())
    }
}

pub fn init() -> ImageSketch {
    let assets = SketchAssets::from_manifest_file(env!("CARGO_MANIFEST_DIR"), file!());

    ImageSketch {
        shader_path: assets.wgsl(),
        image_path: assets.path("../../../assets/vor.png"),
        control_script_path: assets.yaml(),
    }
}
