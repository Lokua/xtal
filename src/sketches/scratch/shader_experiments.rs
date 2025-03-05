use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "shader_experiments",
    display_name: "Shader Experiments",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 1244,
    gui_w: None,
    gui_h: Some(500),
};

#[derive(SketchComponents)]
pub struct ShaderExperiments {
    #[allow(dead_code)]
    controls: ControlScript<Timing>,
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
    let controls = ControlScript::from_path(
        to_absolute_path(file!(), "./shader_experiments.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
        c: [0.0; 4],
        d: [0.0; 4],
    };

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        ctx.window_rect().resolution_u32(),
        to_absolute_path(file!(), "./shader_experiments.wgsl"),
        &params,
        true,
    );

    ShaderExperiments { controls, gpu }
}

impl Sketch for ShaderExperiments {
    fn update(&mut self, app: &App, _update: Update, ctx: &LatticeContext) {
        let wr = ctx.window_rect();
        self.controls.update();

        let params = ShaderParams {
            resolution: [wr.w(), wr.h(), 0.0, 0.0],
            a: [
                self.controls.get("a1"),
                self.controls.get("a2"),
                self.controls.get("a3"),
                self.controls.get("a4"),
            ],
            b: [
                self.controls.get("b1"),
                self.controls.get("b2"),
                self.controls.get("b3"),
                self.controls.get("b4"),
            ],
            c: [
                self.controls.get("c1"),
                self.controls.get("c2"),
                self.controls.get("c3"),
                self.controls.get("c4"),
            ],
            d: [
                self.controls.get("d1"),
                self.controls.get("d2"),
                self.controls.get("d3"),
                self.controls.get("d4"),
            ],
        };

        self.gpu.update_params(app, wr.resolution_u32(), &params);
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &LatticeContext) {
        frame.clear(BLACK);
        self.gpu.render(&frame);
    }
}
