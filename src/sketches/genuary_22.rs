use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "genuary_22",
    display_name: "Genuary 22: Gradients Only",
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
    #[allow(dead_code)]
    animation: Animation<FrameTiming>,
    animation_script: AnimationScript<FrameTiming>,
    controls: Controls,
    wr: WindowRect,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderParams {
    // w, h, ..unused
    resolution: [f32; 4],

    // t1, t2, t3, t4
    a: [f32; 4],

    // b1, b2, ..unused
    b: [f32; 4],
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let animation = Animation::new(FrameTiming::new(SKETCH_CONFIG.bpm));

    let animation_script = AnimationScript::new(
        to_absolute_path(file!(), "./genuary_22.toml"),
        animation.clone(),
    );

    let controls = Controls::new(vec![
        Control::slider_norm("b1", 0.5),
        Control::slider_norm("b2", 0.5),
    ]);

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
    };

    let gpu = gpu::GpuState::new_full_screen(
        app,
        to_absolute_path(file!(), "./genuary_22.wgsl"),
        &params,
        true,
    );

    Model {
        animation,
        animation_script,
        controls,
        wr,
        gpu,
    }
}

pub fn update(app: &App, m: &mut Model, _update: Update) {
    m.animation_script.update();

    let params = ShaderParams {
        resolution: [m.wr.w(), m.wr.h(), 0.0, 0.0],
        a: [
            m.animation.ping_pong(1.5),
            m.animation.ping_pong(2.0),
            m.animation.ping_pong(3.0),
            m.animation.ping_pong(4.0),
        ],
        b: [m.controls.float("b1"), m.controls.float("b2"), 0.0, 0.0],
    };

    m.gpu.update_params(app, &params);
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    frame.clear(BLACK);
    m.gpu.render(&frame);
}
