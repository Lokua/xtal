use nannou::prelude::*;

use xtal::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "dyn_uni_fs_template",
    display_name: "dyn_uni_fs_template",
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

#[uniforms(banks = 4)]
struct ShaderParams {}

pub fn init(app: &App, ctx: &Context) -> DynamicUniformsDev {
    let wr = ctx.window_rect();

    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "dyn_uni_fs_template.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams::default();

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "dyn_uni_fs_template.wgsl"),
        &params,
        true,
    );

    DynamicUniformsDev { hub, gpu }
}

impl Sketch for DynamicUniformsDev {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();
        let params = ShaderParams::from((&wr, &self.hub));
        self.gpu.update_params(app, wr.resolution_u32(), &params);
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &Context) {
        self.gpu.render(&frame);
    }
}
