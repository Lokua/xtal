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
pub struct SierpinskiTriangle {
    #[allow(dead_code)]
    animation: Animation<Timing>,
    controls: Controls,
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

pub fn init(app: &App, ctx: LatticeContext) -> SierpinskiTriangle {
    let wr = ctx.window_rect();
    let animation = Animation::new(Timing::new(ctx.bpm()));

    let controls = Controls::with_previous(vec![
        Control::slider("primary_iterations", 1.0, (0.0, 16.0), 1.0),
        Control::slider("second_iterations", 1.0, (0.0, 16.0), 1.0),
        Control::slider("third_iterations", 1.0, (0.0, 16.0), 1.0),
        Control::slider("fourth_iterations", 1.0, (0.0, 16.0), 1.0),
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
        wr.resolution_u32(),
        to_absolute_path(file!(), "./sierpinski_triangle.wgsl"),
        &params,
        true,
    );

    SierpinskiTriangle {
        animation,
        controls,
        gpu,
    }
}

impl Sketch for SierpinskiTriangle {
    fn update(&mut self, app: &App, _update: Update, ctx: &LatticeContext) {
        let wr = ctx.window_rect();

        let params = ShaderParams {
            resolution: [wr.w(), wr.h(), 0.0, 0.0],
            a: [
                self.controls.float("primary_iterations"),
                self.controls.float("second_iterations"),
                self.controls.float("third_iterations"),
                self.controls.float("fourth_iterations"),
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
