use xtal::prelude::*;

use crate::constants::{HD_HEIGHT, HD_WIDTH};

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "watercolor",
    display_name: "Watercolor",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: HD_WIDTH,
    h: HD_HEIGHT,
    banks: 7,
};

pub fn init() -> FullscreenShaderSketch {
    let assets =
        SketchAssets::from_manifest_file(env!("CARGO_MANIFEST_DIR"), file!());
    FullscreenShaderSketch::new(assets.wgsl())
        .with_control_script(assets.yaml())
}
