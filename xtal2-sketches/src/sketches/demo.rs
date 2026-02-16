use xtal2::prelude::*;

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "demo",
    display_name: "Demo",
    fps: 60.0,
    bpm: 120.0,
    w: 1920 / 2,
    h: 1080 / 2,
    banks: 4,
};

pub fn init() -> FullscreenShaderSketch {
    let assets = SketchAssets::from_file(file!());
    FullscreenShaderSketch::new(assets.wgsl())
        .with_control_script(assets.yaml())
}
