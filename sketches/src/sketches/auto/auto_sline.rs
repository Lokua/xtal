use nannou::prelude::*;

use xtal::prelude::*;

use crate::sketches::common::{HD_HEIGHT, HD_WIDTH};

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "auto_sline",
    display_name: "Spiral | Lines Version",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 120.0,
    w: HD_WIDTH,
    h: HD_HEIGHT,
};

#[uniforms(banks = 8)]
struct ShaderParams {}

#[derive(SketchComponents)]
pub struct AutoSline {
    hub: ControlHub<Timing>,
    gpu: gpu::GpuState<()>,
}

pub fn init(app: &App, ctx: &Context) -> AutoSline {
    let wr = ctx.window_rect();
    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "auto_sline.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams::default();

    let gpu = gpu::GpuState::new_procedural(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "auto_sline.wgsl"),
        &params,
    );

    AutoSline { hub, gpu }
}

impl Sketch for AutoSline {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();
        let mut params = ShaderParams::from((&wr, &self.hub));
        params.set("a4", self.hub.animation.beats());

        self.gpu.update_params(app, wr.resolution_u32(), &params);
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &Context) {
        frame.clear(WHITE);

        let points_per_line = self.hub.get("samples_per_line") as u32;
        let n_lines = self.hub.get("line_count") as u32;
        let total_points = points_per_line * n_lines;
        let density = self.hub.get("density") as u32;
        let spiral_vertices = total_points * 6 * density;
        let background_vertices = 3;
        let total_vertices = background_vertices + spiral_vertices;

        self.gpu.render_procedural(&frame, total_vertices);
    }
}
