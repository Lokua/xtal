use bytemuck::{Pod, Zeroable};
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "blob",
    display_name: "Blob",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 1244,
    gui_w: None,
    gui_h: Some(360),
};

#[derive(SketchComponents)]
pub struct Model {
    controls: ControlScript<Timing>,
    wr: WindowRect,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct ShaderParams {
    // w, h, ..unused
    resolution: [f32; 4],

    // t1, t2, t3, t4
    a: [f32; 4],

    // b1, b2, ..unused
    b: [f32; 4],
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let controls = ControlScript::new(
        to_absolute_path(file!(), "blob.yaml"),
        Timing::new(SKETCH_CONFIG.bpm),
    );

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
    };

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "./blob.wgsl"),
        &params,
        true,
    );

    Model { controls, wr, gpu }
}

pub fn update(app: &App, m: &mut Model, _update: Update) {
    let params = ShaderParams {
        resolution: [m.wr.w(), m.wr.h(), 0.0, 0.0],
        a: [
            m.controls.get("a1"),
            m.controls.get("a2"),
            m.controls.get("a3"),
            m.controls.get("a4"),
        ],
        b: [
            m.controls.get("b1"),
            m.controls.get("b2"),
            m.controls.get("b3"),
            m.controls.get("b4"),
        ],
    };

    m.gpu.update_params(app, m.wr.resolution_u32(), &params);
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    frame.clear(BLACK);
    m.gpu.render(&frame);
}
