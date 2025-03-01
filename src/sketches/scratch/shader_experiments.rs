use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "shader_experiments",
    display_name: "Shader Experiments",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(500),
};

#[derive(LegacySketchComponents)]
pub struct Model {
    #[allow(dead_code)]
    animation: Animation<FrameTiming>,
    controls: Controls,
    wr: WindowRect,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderParams {
    // w, h, ..unused
    resolution: [f32; 4],
    a: [f32; 4],
    b: [f32; 4],
    c: [f32; 4],
    d: [f32; 4],
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let animation = Animation::new(FrameTiming::new(SKETCH_CONFIG.bpm));

    let controls = Controls::with_previous(vec![
        Control::slide("a1", 0.5),
        Control::slide("a2", 0.5),
        Control::slide("a3", 0.5),
        Control::slide("a4", 0.5),
        Control::Separator {}, // -------------------
        Control::slide("b1", 0.5),
        Control::slide("b2", 0.5),
        Control::slide("b3", 0.5),
        Control::slide("b4", 0.5),
        Control::Separator {}, // -------------------
        Control::slide("c1", 0.5),
        Control::slide("c2", 0.5),
        Control::slide("c3", 0.5),
        Control::slide("c4", 0.5),
        Control::Separator {}, // -------------------
        Control::slide("d1", 0.5),
        Control::slide("d2", 0.5),
        Control::slide("d3", 0.5),
        Control::slide("d4", 0.5),
    ]);

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
        c: [0.0; 4],
        d: [0.0; 4],
    };

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "./shader_experiments.wgsl"),
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
        resolution: [m.wr.w(), m.wr.h(), 0.0, 0.0],
        a: [
            m.controls.float("a1"),
            m.controls.float("a2"),
            m.controls.float("a3"),
            m.controls.float("a4"),
        ],
        b: [
            m.controls.float("b1"),
            m.controls.float("b2"),
            m.controls.float("b3"),
            m.controls.float("b4"),
        ],
        c: [
            m.controls.float("c1"),
            m.controls.float("c2"),
            m.controls.float("c3"),
            m.controls.float("c4"),
        ],
        d: [
            m.controls.float("d1"),
            m.controls.float("d2"),
            m.controls.float("d3"),
            m.controls.float("d4"),
        ],
    };

    m.gpu.update_params(app, m.wr.resolution_u32(), &params);
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    frame.clear(BLACK);
    m.gpu.render(&frame);
}
