use nannou::prelude::*;

use xtal::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "kalos",
    display_name: "Kalos",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
};

#[derive(SketchComponents)]
pub struct Kalos {
    hub: ControlHub<Timing>,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[uniforms(banks = 7)]
struct ShaderParams {}

pub fn init(app: &App, ctx: &Context) -> Kalos {
    let wr = ctx.window_rect();

    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "kalos.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams::default();

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "kalos.wgsl"),
        &params,
        0,
    );

    Kalos { hub, gpu }
}

impl Sketch for Kalos {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();
        let mut params = ShaderParams::from((&wr, &self.hub));

        params.set(
            "b3",
            self.hub.select(
                "animate_strength",
                "strength_animation",
                "strength",
            ),
        );

        self.gpu.update_params(app, wr.resolution_u32(), &params);
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &Context) {
        self.gpu.render(&frame);
    }
}
