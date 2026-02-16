use xtal2::prelude::*;

const CONFIG: SketchConfig = SketchConfig {
    name: "image",
    display_name: "Image",
    fps: 60.0,
    w: 900,
    h: 600,
    banks: 4,
};

struct ImageSketch {
    shader_path: &'static str,
    image_path: &'static str,
}

impl Sketch for ImageSketch {
    fn setup(&self, graph: &mut GraphBuilder) {
        graph.uniforms("params");
        graph.image("img0", self.image_path);

        graph
            .render("image_pass")
            .shader(self.shader_path)
            .read("params")
            .read("img0")
            .write("surface")
            .add();

        graph.present("surface");
    }
}

fn main() {
    let shader_path =
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/image.wgsl");
    let image_path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vor.png");
    let control_script =
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/image.yaml");

    let sketch = ImageSketch {
        shader_path,
        image_path,
    };

    if let Err(err) = run_with_control_script(&CONFIG, sketch, control_script) {
        eprintln!("xtal2 image demo failed: {}", err);
        std::process::exit(1);
    }
}
