use nannou::prelude::*;
use xtal::prelude::*;

use crate::sketches::common::{HD_HEIGHT, HD_WIDTH};

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "auto_ink",
    display_name: "Auto Ink",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: HD_WIDTH,
    h: HD_HEIGHT,
};

#[derive(SketchComponents)]
pub struct Ink {
    hub: ControlHub<Timing>,
    shader_1: gpu::GpuState<gpu::BasicPositionVertex>,
    shader_2: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[uniforms(banks = 8)]
struct ShaderParams {}

pub fn init(app: &App, ctx: &Context) -> Ink {
    let wr = ctx.window_rect();

    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "auto_ink.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams::default();

    let shader_1 = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "auto_ink.wgsl"),
        &params,
        0,
    );

    let shader_2 = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "auto_ink_post.wgsl"),
        &params,
        1,
    );

    Ink {
        hub,
        shader_1,
        shader_2,
    }
}

impl Sketch for Ink {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();
        let mut params = ShaderParams::from((&wr, &self.hub));
        params.set("a3", self.hub.animation.beats());
        let res = wr.resolution_u32();

        self.shader_1.update_params(app, res, &params);
        self.shader_2.update_params(app, res, &params);

        let texture = self.shader_1.render_to_texture(app);
        self.shader_2.set_texture(app, &texture);
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &Context) {
        self.shader_2.render(&frame);
    }
}
