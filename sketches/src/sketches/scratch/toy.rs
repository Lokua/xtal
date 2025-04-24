use nannou::prelude::*;

use xtal::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "toy",
    display_name: "Toy",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
};

#[derive(SketchComponents)]
pub struct DynamicUniformsDev {
    hub: ControlHub<Timing>,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[uniforms(banks = 6)]
struct ShaderParams {}

pub fn init(app: &App, ctx: &Context) -> DynamicUniformsDev {
    let wr = ctx.window_rect();

    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "toy.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams::default();

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "toy.wgsl"),
        &params,
        true,
    );

    DynamicUniformsDev { hub, gpu }
}

impl Sketch for DynamicUniformsDev {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();
        let mut params = ShaderParams::from((&wr, &self.hub));
        params.set("a3", self.hub.animation.beats());
        self.gpu.update_params(app, wr.resolution_u32(), &params);
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &Context) {
        self.gpu.render(&frame);
    }
}
