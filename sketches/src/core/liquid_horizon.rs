use xtal::prelude::*;

use crate::constants::{HD_HEIGHT, HD_WIDTH, WORKING_BPM};

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "liquid_horizon",
    display_name: "Liquid Horizon",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: WORKING_BPM,
    w: HD_WIDTH,
    h: HD_HEIGHT,
    banks: 12,
};

pub fn init() -> FullscreenShaderSketch {
    let assets = SketchAssets::from_file(file!());
    FullscreenShaderSketch::new(assets.wgsl())
        .with_control_script(assets.yaml())
}
