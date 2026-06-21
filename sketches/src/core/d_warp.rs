use xtal::prelude::*;

use crate::constants::WORKING_BPM;

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "d_warp",
    display_name: "Domain Warping",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    // bpm: 134.0,
    bpm: WORKING_BPM,
    w: 700,
    h: 700,
    banks: 7,
};

pub fn init() -> FullscreenShaderSketch {
    let assets = SketchAssets::from_file(file!());
    FullscreenShaderSketch::new(assets.wgsl())
        .with_control_script(assets.yaml())
}
