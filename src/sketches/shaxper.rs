use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "shaxper",
    display_name: "Shaxper",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 1244,
};

#[derive(SketchComponents)]
pub struct ShaderExperiments {
    #[allow(dead_code)]
    controls: ControlHub<Timing>,
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
    e: [f32; 4],
    f: [f32; 4],
}

pub fn init(app: &App, ctx: &Context) -> ShaderExperiments {
    let controls = ControlHub::from_path(
        to_absolute_path(file!(), "shaxper.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
        c: [0.0; 4],
        d: [0.0; 4],
        e: [0.0; 4],
        f: [0.0; 4],
    };

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        ctx.window_rect().resolution_u32(),
        to_absolute_path(file!(), "shaxper.wgsl"),
        &params,
        true,
    );

    ShaderExperiments { controls, gpu }
}

impl Sketch for ShaderExperiments {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();

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
            e: [
                self.controls.get("e1"),
                self.controls.get("e2"),
                self.controls.get("e3"),
                self.controls.get("e4"),
            ],
            f: [
                self.controls.get("f1"),
                self.controls.get("f2"),
                self.controls.get("f3"),
                self.controls.get("f4"),
            ],
        };

        self.gpu.update_params(app, wr.resolution_u32(), &params);
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &Context) {
        frame.clear(BLACK);
        self.gpu.render(&frame);
    }
}
