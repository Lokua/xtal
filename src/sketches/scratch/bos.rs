use nannou::prelude::*;

use crate::framework::prelude::*;

// Scratch sketch to follow along with https://thebookofshaders.com

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "bos",
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
pub struct Bos {
    controls: ControlHub<Timing>,
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

pub fn init(app: &App, ctx: &Ctx) -> Bos {
    let controls = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .slider_n("a", 0.5)
        .slider_n("b", 0.5)
        .build();

    let params = ShaderParams {
        resolution: ctx.window_rect().resolution(),
        a: 0.0,
        b: 0.0,
        t: 0.0,
        _pad: 0.0,
    };

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        ctx.window_rect().resolution_u32(),
        to_absolute_path(file!(), "./bos.wgsl"),
        &params,
        true,
    );

    Bos { controls, gpu }
}

impl Sketch for Bos {
    fn update(&mut self, app: &App, _update: Update, ctx: &Ctx) {
        let wr = ctx.window_rect();

        let params = ShaderParams {
            resolution: wr.resolution(),
            a: self.controls.get("a"),
            b: self.controls.get("b"),
            t: self.controls.animation.tri(4.0),
            _pad: 0.0,
        };

        self.gpu.update_params(app, wr.resolution_u32(), &params);
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &Ctx) {
        frame.clear(BLACK);
        self.gpu.render(&frame);
    }
}
