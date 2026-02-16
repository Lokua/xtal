use std::path::PathBuf;

use xtal2::prelude::*;

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "compute",
    display_name: "Compute",
    fps: 60.0,
    w: 900,
    h: 600,
    banks: 4,
};

pub struct ComputeSketch {
    compute_shader: PathBuf,
    present_shader: PathBuf,
}

impl Sketch for ComputeSketch {
    fn setup(&self, graph: &mut GraphBuilder) {
        graph.uniforms("params");
        graph.texture2d("field");

        graph
            .compute("field_compute")
            .shader(self.compute_shader.clone())
            .read_write("field")
            .add();

        graph
            .render("present")
            .shader(self.present_shader.clone())
            .read("params")
            .read("field")
            .write("surface")
            .add();

        graph.present("surface");
    }
}

pub fn init() -> ComputeSketch {
    let assets = SketchAssets::from_file(file!());

    ComputeSketch {
        compute_shader: assets.wgsl(),
        present_shader: assets.path("compute_present.wgsl"),
    }
}
