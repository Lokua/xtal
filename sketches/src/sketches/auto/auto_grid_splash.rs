use nannou::prelude::*;

use xtal::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "grid_splash",
    display_name: "Grid Splash",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
};

#[derive(SketchComponents)]
pub struct GridSplash {
    hub: ControlHub<Timing>,
    shader: gpu::GpuState<gpu::BasicPositionVertex>,
    texture: Option<wgpu::TextureView>,
}

#[uniforms(banks = 10)]
struct ShaderParams {}

pub fn init(app: &App, ctx: &Context) -> GridSplash {
    let wr = ctx.window_rect();

    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "grid_splash.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams::default();

    let shader = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "grid_splash.wgsl"),
        &params,
        1,
    );

    GridSplash {
        hub,
        shader,
        texture: None,
    }
}

impl Sketch for GridSplash {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();
        let mut params = ShaderParams::from((&wr, &self.hub));
        params.set("a3", self.hub.animation.beats());
        self.shader.update_params(app, wr.resolution_u32(), &params);

        if let Some(ref texture) = self.texture {
            self.shader.set_textures(app, &[texture]);
        }
        self.texture = Some(self.shader.render_to_texture(app));
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &Context) {
        self.shader.render(&frame);
    }
}
