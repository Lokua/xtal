use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "sierpinski_triangle_anim",
    display_name: "Sierpinski Triangle Animation",
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
    midi: MidiControls,
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

    let midi = MidiControlBuilder::new()
        .control_mapped("primary_iterations", (0, 1), (0.0, 5.0), 0.0)
        .control_mapped("second_iterations", (0, 2), (0.0, 3.0), 0.0)
        .control_mapped("third_iterations", (0, 3), (0.0, 3.0), 0.0)
        .control_mapped("fourth_iterations", (0, 4), (0.0, 3.0), 0.0)
        .build();

    Model {
        animation,
        controls,
        wr,
        gpu,
        midi,
    }
}

pub fn update(app: &App, m: &mut Model, _update: Update) {
    let params = ShaderParams {
        resolution: [m.wr.w(), m.wr.h(), 0.0, 0.0],
        a: [
            m.midi.get("primary_iterations").floor(),
            m.midi.get("second_iterations").floor(),
            m.midi.get("third_iterations").floor(),
            m.midi.get("fourth_iterations").floor(),
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
