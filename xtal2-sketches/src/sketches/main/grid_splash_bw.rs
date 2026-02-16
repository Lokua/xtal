use std::path::PathBuf;

use xtal2::prelude::*;

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "grid_splash_bw",
    display_name: "Grid Splash B&W",
    fps: 60.0,
    bpm: 134.0,
    w: 1920 / 2,
    h: 1080 / 2,
    banks: 10,
};

pub struct GridSplashBwSketch {
    shader_path: PathBuf,
    control_script_path: PathBuf,
}

impl Sketch for GridSplashBwSketch {
    fn setup(&self, graph: &mut GraphBuilder) {
        graph.uniforms("params");

        // Provide a sampled texture binding for shaders that reference
        // @group(1) feedback textures.
        graph.texture2d("feedback");

        graph
            .render("main")
            .shader(self.shader_path.clone())
            .read("params")
            .read("feedback")
            .write("surface")
            .add();

        graph.present("surface");
    }

    fn control_script(&self) -> Option<PathBuf> {
        Some(self.control_script_path.clone())
    }
}

pub fn init() -> GridSplashBwSketch {
    let assets = SketchAssets::from_file(file!());

    GridSplashBwSketch {
        shader_path: assets.wgsl(),
        control_script_path: assets.yaml(),
    }
}
