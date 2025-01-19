use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "sierpinski_triangle",
    display_name: "Sierpinski Triangle",
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
    // primary_iterations, second_iterations, third_iterations, fourth_iterations
    a: [f32; 4],
    // scale, y_offset, unused, unused
    b: [f32; 4],
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let animation = Animation::new(FrameTiming::new(SKETCH_CONFIG.bpm));

    let controls = Controls::with_previous(vec![
        Control::slider("primary_iterations", 1.0, (0.0, 16.0), 1.0),
        Control::slider("second_iterations", 1.0, (0.0, 16.0), 1.0),
        Control::slider("third_iterations", 1.0, (0.0, 16.0), 1.0),
        Control::slider("fourth_iterations", 1.0, (0.0, 16.0), 1.0),
        Control::slider("scale", 1.0, (0.0001, 2.0), 0.0001),
        Control::slider_norm("y_offset", 0.3),
    ]);

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
    };

    let shader = wgpu::include_wgsl!("./sierpinski_triangle.wgsl");
    let gpu = gpu::GpuState::new_full_screen(app, shader, &params);

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
            m.controls.float("primary_iterations"),
            m.controls.float("second_iterations"),
            m.controls.float("third_iterations"),
            m.controls.float("fourth_iterations"),
        ],
        b: [
            m.controls.float("scale"),
            m.controls.float("y_offset"),
            0.0,
            0.0,
        ],
    };

    m.gpu.update_params(app, &params);
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    frame.clear(BLACK);
    m.gpu.render(&frame);
}
