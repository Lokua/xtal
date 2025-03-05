use nannou::prelude::*;

use crate::framework::prelude::*;

// Live/2025/2025.01.14 Lattice - Sierpinski Triangle Project

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "g25_13_triangle",
    display_name: "Genuary 13: Triangles and nothing else",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(360),
};

#[derive(SketchComponents)]
pub struct G25_13Triangle {
    #[allow(dead_code)]
    animation: Animation<FrameTiming>,
    controls: Controls,
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

pub fn init(app: &App, ctx: &LatticeContext) -> G25_13Triangle {
    let animation = Animation::new(FrameTiming::new(ctx.bpm()));

    let controls = Controls::with_previous(vec![
        Control::slider("scale", 1.0, (0.0001, 2.0), 0.0001),
        Control::slide("y_offset", 0.3),
    ]);

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
    };

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        ctx.window_rect().resolution_u32(),
        to_absolute_path(file!(), "g25_13_triangle.wgsl"),
        &params,
        true,
    );

    let midi = MidiControlBuilder::new()
        .control_mapped("primary_iterations", (0, 1), (0.0, 5.0), 0.0)
        .control_mapped("second_iterations", (0, 2), (0.0, 3.0), 0.0)
        .control_mapped("third_iterations", (0, 3), (0.0, 3.0), 0.0)
        .control_mapped("fourth_iterations", (0, 4), (0.0, 3.0), 0.0)
        .build();

    G25_13Triangle {
        animation,
        controls,
        gpu,
        midi,
    }
}

impl Sketch for G25_13Triangle {
    fn update(&mut self, app: &App, _update: Update, ctx: &LatticeContext) {
        let wr = ctx.window_rect();

        let params = ShaderParams {
            resolution: [wr.w(), wr.h(), 0.0, 0.0],
            a: [
                self.midi.get("primary_iterations").floor(),
                self.midi.get("second_iterations").floor(),
                self.midi.get("third_iterations").floor(),
                self.midi.get("fourth_iterations").floor(),
            ],
            b: [
                self.controls.float("scale"),
                self.controls.float("y_offset"),
                0.0,
                0.0,
            ],
        };

        self.gpu.update_params(
            app,
            ctx.window_rect().resolution_u32(),
            &params,
        );
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &LatticeContext) {
        frame.clear(BLACK);
        self.gpu.render(&frame);
    }
}
