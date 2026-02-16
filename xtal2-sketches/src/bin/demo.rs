use xtal2::prelude::*;

const CONFIG: SketchConfig = SketchConfig {
    name: "demo",
    display_name: "Demo",
    fps: 60.0,
    w: 1920,
    h: 1080,
    banks: 4,
};

fn main() {
    let shader_path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/demo.wgsl");
    let control_script_path =
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/demo.yaml");

    if let Err(err) =
        run_fullscreen_shader(&CONFIG, shader_path, control_script_path)
    {
        eprintln!("xtal2 demo failed: {}", err);
        std::process::exit(1);
    }
}
