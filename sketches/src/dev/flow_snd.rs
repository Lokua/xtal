use xtal::prelude::*;

use crate::constants::{IG_HEIGHT, IG_WIDTH};

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "flow_snd",
    display_name: "Flow Field Snd",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: IG_WIDTH,
    h: IG_HEIGHT,
    banks: 12,
};

pub fn init() -> FullscreenShaderSketch {
    let assets = SketchAssets::from_file(file!());
    FullscreenShaderSketch::new(assets.wgsl())
        .with_control_script(assets.yaml())
}
