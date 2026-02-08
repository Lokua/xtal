use nannou::prelude::*;
use xtal::prelude::*;

const LOGICAL_WIDTH: i32 = crate::sketches::common::MBP_16_WIDTH_LOGICAL
    - crate::sketches::common::CONTROL_PANEL_WIDTH;
const LOGICAL_HEIGHT: i32 = (LOGICAL_WIDTH as f32 * 9.0 / 16.0) as i32;
// const LOGICAL_WIDTH: i32 = crate::sketches::common::MBP_16_WIDTH_LOGICAL;
// const LOGICAL_HEIGHT: i32 = (LOGICAL_WIDTH as f32 * 9.0 / 16.0) as i32;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "auto_dreams",
    display_name: "Genuary 2026 - Day 14 - Dreams",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: LOGICAL_WIDTH,
    h: LOGICAL_HEIGHT,
};

#[derive(SketchComponents)]
pub struct AutoDreams {
    hub: ControlHub<Timing>,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[uniforms(banks = 8)]
struct ShaderParams {}

pub fn init(app: &App, ctx: &Context) -> AutoDreams {
    let wr = ctx.window_rect();

    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "auto_dreams.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams::default();

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "auto_dreams.wgsl"),
        &params,
        0,
    );

    AutoDreams { hub, gpu }
}

impl Sketch for AutoDreams {
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
