use xtal::prelude::*;

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "projector_stress",
    display_name: "Projector Stress",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 120.0,
    w: 640,
    h: 360,
    banks: 4,
};

pub fn init() -> FullscreenShaderSketch {
    let assets = SketchAssets::from_file(file!());
    FullscreenShaderSketch::new(assets.wgsl())
        .with_control_script(assets.yaml())
}
