use xtal::prelude::*;

use crate::constants::WORKING_BPM;

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "marcher",
    display_name: "Marcher",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    // bpm: 134.0,
    bpm: WORKING_BPM,
    w: 800,
    h: 800,
    banks: 8,
};

pub fn init() -> FullscreenShaderSketch {
    let assets = SketchAssets::from_file(file!());
    FullscreenShaderSketch::new(assets.wgsl())
        .with_control_script(assets.yaml())
}
