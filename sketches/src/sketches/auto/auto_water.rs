use nannou::prelude::*;
use xtal::prelude::*;

use crate::sketches::common::{HD_HEIGHT, HD_WIDTH};

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "auto_water",
    display_name: "Auto Water",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: HD_WIDTH,
    h: HD_HEIGHT,
};

#[derive(SketchComponents)]
pub struct AutoWater {
    hub: ControlHub<Timing>,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[uniforms(banks = 24)]
struct ShaderParams {}

pub fn init(app: &App, ctx: &Context) -> AutoWater {
    let wr = ctx.window_rect();

    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "auto_water.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams::default();

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "auto_water.wgsl"),
        &params,
        0,
    );

    AutoWater { hub, gpu }
}

impl Sketch for AutoWater {
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
