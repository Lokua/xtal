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

#[derive(SketchComponents)]
pub struct ShaderExperiments {
    #[allow(dead_code)]
    animation: Animation<FrameTiming>,
    controls: Controls,
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

pub fn init(app: &App, ctx: &LatticeContext) -> ShaderExperiments {
    let animation =
        Animation::new(FrameTiming::new(Bpm::new(SKETCH_CONFIG.bpm)));

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

    let wr = ctx.window_rect();

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

    ShaderExperiments {
        animation,
        controls,
        gpu,
    }
}

impl Sketch for ShaderExperiments {
    fn update(&mut self, app: &App, _update: Update, ctx: &LatticeContext) {
        let wr = ctx.window_rect();

        let params = ShaderParams {
            resolution: [wr.w(), wr.h(), 0.0, 0.0],
            a: [
                self.controls.float("a1"),
                self.controls.float("a2"),
                self.controls.float("a3"),
                self.controls.float("a4"),
            ],
            b: [
                self.controls.float("b1"),
                self.controls.float("b2"),
                self.controls.float("b3"),
                self.controls.float("b4"),
            ],
            c: [
                self.controls.float("c1"),
                self.controls.float("c2"),
                self.controls.float("c3"),
                self.controls.float("c4"),
            ],
            d: [
                self.controls.float("d1"),
                self.controls.float("d2"),
                self.controls.float("d3"),
                self.controls.float("d4"),
            ],
        };

        self.gpu.update_params(app, wr.resolution_u32(), &params);
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &LatticeContext) {
        frame.clear(BLACK);
        self.gpu.render(&frame);
    }
}
