use xtal2::prelude::*;

const CONFIG: SketchConfig = SketchConfig {
    name: "compute",
    display_name: "Compute",
    fps: 60.0,
    w: 900,
    h: 600,
    banks: 4,
};

struct ComputeSketch {
    compute_shader: &'static str,
    present_shader: &'static str,
}

impl Sketch for ComputeSketch {
    fn setup(&self, graph: &mut GraphBuilder) {
        graph.uniforms("params");
        graph.texture2d("field");

        graph
            .compute("field_compute")
            .shader(self.compute_shader)
            .read_write("field")
            .add();

        graph
            .render("present")
            .shader(self.present_shader)
            .read("params")
            .read("field")
            .write("surface")
            .add();

        graph.present("surface");
    }
}

fn main() {
    let compute_shader =
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/compute.wgsl");
    let present_shader =
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/compute_present.wgsl");
    let control_script =
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/compute.yaml");

    let sketch = ComputeSketch {
        compute_shader,
        present_shader,
    };

    if let Err(err) = run_with_control_script(&CONFIG, sketch, control_script) {
        eprintln!("xtal2 compute failed: {}", err);
        std::process::exit(1);
    }
}
