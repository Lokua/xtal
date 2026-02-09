//! This sketch is used to test an experimental [`uniforms`] proc macro

use nannou::prelude::*;

use xtal::prelude::*;

const LOGICAL_WIDTH: i32 = crate::sketches::common::MBP_16_WIDTH_LOGICAL
    - crate::sketches::common::CONTROL_PANEL_WIDTH;
const LOGICAL_HEIGHT: i32 = (LOGICAL_WIDTH as f32 * 9.0 / 16.0) as i32;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "auto_dyn_uni",
    display_name: "Auto Dynamic Uniforms",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: LOGICAL_WIDTH,
    h: LOGICAL_HEIGHT,
};

#[derive(SketchComponents)]
pub struct AutoDynUni {
    hub: ControlHub<Timing>,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[uniforms(banks = 8)]
struct ShaderParams {}

pub fn init(app: &App, ctx: &Context) -> AutoDynUni {
    let wr = ctx.window_rect();
    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "auto_dyn_uni.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams::default();

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "auto_dyn_uni.wgsl"),
        &params,
        0,
    );

    AutoDynUni { hub, gpu }
}

impl Sketch for AutoDynUni {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();
        let params = ShaderParams::from((&wr, &self.hub));
        self.gpu.update_params(app, wr.resolution_u32(), &params);
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &Context) {
        self.gpu.render(&frame);
    }
}
