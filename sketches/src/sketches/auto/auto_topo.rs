use nannou::prelude::*;
use xtal::prelude::*;

use crate::sketches::common::{HD_HEIGHT, HD_WIDTH};

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "auto_topo",
    display_name: "Terrain Scanner",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: HD_WIDTH,
    h: HD_HEIGHT,
};

#[derive(SketchComponents)]
pub struct AutoTopo {
    hub: ControlHub<Timing>,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[uniforms(banks = 8)]
struct ShaderParams {}

pub fn init(app: &App, ctx: &Context) -> AutoTopo {
    let wr = ctx.window_rect();

    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "auto_topo.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams::default();

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "auto_topo.wgsl"),
        &params,
        0,
    );

    AutoTopo { hub, gpu }
}

impl Sketch for AutoTopo {
    fn update(
        &mut self,
        app: &App,
        _update: Update,
        ctx: &Context,
    ) {
        let wr = ctx.window_rect();
        let mut params =
            ShaderParams::from((&wr, &self.hub));
        params.set("a3", self.hub.animation.beats());
        self.gpu
            .update_params(app, wr.resolution_u32(), &params);
    }

    fn view(
        &self,
        _app: &App,
        frame: Frame,
        _ctx: &Context,
    ) {
        self.gpu.render(&frame);
    }
}
