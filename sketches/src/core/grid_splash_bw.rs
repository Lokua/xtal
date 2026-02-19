use std::path::PathBuf;

use xtal::prelude::*;

use crate::constants::{HD_HEIGHT, HD_WIDTH};

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "grid_splash_bw",
    display_name: "Grid Splash B&W",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: HD_WIDTH,
    h: HD_HEIGHT,
    banks: 10,
};

pub struct GridSplashBwSketch {
    shader_path: PathBuf,
    control_script_path: PathBuf,
}

impl Sketch for GridSplashBwSketch {
    fn setup(&self, graph: &mut GraphBuilder) {
        let params = graph.uniforms();

        // Feedback ping-pong textures.
        let (ping, pong) = graph.feedback();

        graph
            .render()
            .shader(self.shader_path.clone())
            .mesh(Mesh::fullscreen_quad())
            .read(params)
            .read(ping)
            .to(pong);

        // Second feedback step writes back into `ping` so the next frame
        // samples an updated history texture.
        graph
            .render()
            .shader(self.shader_path.clone())
            .mesh(Mesh::fullscreen_quad())
            .read(params)
            .read(pong)
            .to(ping);

        graph.present(ping);
    }

    fn control_script(&self) -> Option<PathBuf> {
        Some(self.control_script_path.clone())
    }
}

pub fn init() -> GridSplashBwSketch {
    let assets = SketchAssets::from_file(file!());

    GridSplashBwSketch {
        shader_path: assets.wgsl(),
        control_script_path: assets.yaml(),
    }
}
