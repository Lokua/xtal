use xtal::prelude::*;

use crate::constants::{HD_HEIGHT, HD_WIDTH};

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "basic",
    display_name: "Basic",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 120.0,
    w: HD_WIDTH,
    h: HD_HEIGHT,
    banks: 4,
};

pub fn init() -> FullscreenShaderSketch {
    let assets = SketchAssets::from_file(file!());
    FullscreenShaderSketch::new(assets.wgsl())
        .with_control_script(assets.yaml())
}
