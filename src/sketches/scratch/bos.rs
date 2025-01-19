use gpu::GpuState;
use nannou::prelude::*;

use crate::framework::prelude::*;

// Scratch sketch to follow along with https://thebookofshaders.com

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "bos_07",
    display_name: "BOS 07",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(360),
};

#[derive(SketchComponents)]
pub struct Model {
    animation: Animation<FrameTiming>,
    controls: Controls,
    wr: WindowRect,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderParams {
    resolution: [f32; 2],
    a: f32,
    b: f32,
    t: f32,
    _pad: f32,
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let animation = Animation::new(FrameTiming::new(SKETCH_CONFIG.bpm));

    let controls = Controls::with_previous(vec![
        Control::slider_norm("a", 0.5),
        Control::slider_norm("b", 0.5),
    ]);

    let params = ShaderParams {
        resolution: wr.resolution(),
        a: 0.0,
        b: 0.0,
        t: 0.0,
        _pad: 0.0,
    };

    let gpu = gpu::GpuState::new_full_screen(
        app,
        to_absolute_path(file!(), "./bos.wgsl"),
        &params,
        true,
    );

    Model {
        animation,
        controls,
        wr,
        gpu,
    }
}

pub fn update(app: &App, m: &mut Model, _update: Update) {
    let params = ShaderParams {
        resolution: m.wr.resolution(),
        a: m.controls.float("a"),
        b: m.controls.float("b"),
        t: m.animation.ping_pong(4.0),
        _pad: 0.0,
    };

    m.gpu.update_params(app, &params);
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    frame.clear(BLACK);
    m.gpu.render(&frame);
}
