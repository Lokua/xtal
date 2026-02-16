use xtal2::prelude::*;

const CONFIG: SketchConfig = SketchConfig {
    name: "multipass",
    display_name: "Multipass",
    fps: 60.0,
    w: 900,
    h: 600,
    banks: 4,
};

struct MultiPassSketch {
    pass_a: &'static str,
    pass_b: &'static str,
}

impl Sketch for MultiPassSketch {
    fn setup(&self, graph: &mut GraphBuilder) {
        graph.uniforms("params");
        graph.texture2d("rt0");

        graph
            .render("pass_a")
            .shader(self.pass_a)
            .read("params")
            .write("rt0")
            .add();

        graph
            .render("pass_b")
            .shader(self.pass_b)
            .read("params")
            .read("rt0")
            .write("surface")
            .add();

        graph.present("surface");
    }
}

fn main() {
    let pass_a =
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/multipass_a.wgsl");
    let pass_b =
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/multipass_b.wgsl");
    let control_script =
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/multipass.yaml");

    let sketch = MultiPassSketch { pass_a, pass_b };

    if let Err(err) = run_with_control_script(&CONFIG, sketch, control_script) {
        eprintln!("xtal2 multipass failed: {}", err);
        std::process::exit(1);
    }
}
