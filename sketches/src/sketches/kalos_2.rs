use nannou::prelude::*;

use xtal::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "kalos_2",
    display_name: "Kalos 2",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
};

#[derive(SketchComponents)]
pub struct Kalos2Dyn {
    hub: ControlHub<Timing>,
    shader_1: gpu::GpuState<gpu::BasicPositionVertex>,
    shader_2: gpu::GpuState<gpu::BasicPositionVertex>,
    prev_texture: Option<wgpu::TextureView>,
}

#[uniforms(banks = 8)]
struct ShaderParams {}

pub fn init(app: &App, ctx: &Context) -> Kalos2Dyn {
    let wr = ctx.window_rect();

    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "kalos_2.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams::default();

    let shader_1 = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "kalos_2_shader_1.wgsl"),
        &params,
        0,
        true,
    );

    let shader_2 = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "kalos_2_shader_2.wgsl"),
        &params,
        2,
        true,
    );

    Kalos2Dyn {
        hub,
        shader_1,
        shader_2,
        prev_texture: None,
    }
}

impl Sketch for Kalos2Dyn {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();

        let mut params = ShaderParams::from((&wr, &self.hub));
        params.set("g2", self.hub.animation.beats());

        let res = wr.resolution_u32();
        self.shader_1.update_params(app, res, &params);
        self.shader_2.update_params(app, res, &params);

        let texture = self.shader_1.render_to_texture(app);

        if let Some(ref prev_texture) = self.prev_texture {
            self.shader_2.set_textures(app, &[&texture, prev_texture]);
        } else {
            self.shader_2.set_textures(app, &[&texture, &texture]);
        }

        let shader_2_output = self.shader_2.render_to_texture(app);

        self.prev_texture = Some(shader_2_output);
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &Context) {
        self.shader_2.render(&frame);
    }
}
