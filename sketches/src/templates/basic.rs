use xtal::prelude::*;

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "basic",
    display_name: "Basic",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 120.0,
    w: 1920 / 2,
    h: 1080 / 2,
    banks: 4,
};

pub fn init() -> FullscreenShaderSketch {
    let assets = SketchAssets::from_manifest_file(env!("CARGO_MANIFEST_DIR"), file!());
    FullscreenShaderSketch::new(assets.wgsl())
        .with_control_script(assets.yaml())
}
