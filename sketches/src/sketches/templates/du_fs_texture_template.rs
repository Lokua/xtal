use nannou::prelude::*;

use xtal::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "du_fs_texture_template",
    display_name: "Dynamic Uniforms w/ Texture Pass Template",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
};

#[derive(SketchComponents)]
pub struct DynamicUniformsDev {
    hub: ControlHub<Timing>,
    shader_1: gpu::GpuState<gpu::BasicPositionVertex>,
    shader_2: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[uniforms(banks = 4)]
struct ShaderParams {}

pub fn init(app: &App, ctx: &Context) -> DynamicUniformsDev {
    let wr = ctx.window_rect();

    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "du_fs_texture_template.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams::default();

    let shader_1 = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "du_fs_texture_template_1.wgsl"),
        &params,
        0,
        true,
    );

    let shader_2 = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "du_fs_texture_template_2.wgsl"),
        &params,
        1,
        true,
    );

    DynamicUniformsDev {
        hub,
        shader_1,
        shader_2,
    }
}

impl Sketch for DynamicUniformsDev {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();

        let params = ShaderParams::from((&wr, &self.hub));

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
