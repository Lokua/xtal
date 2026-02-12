use nannou::prelude::*;
use xtal::prelude::*;

use crate::sketches::common::{HD_HEIGHT, HD_WIDTH};

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "auto_wave_fract",
    display_name: "Auto Wave Fract",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: HD_WIDTH,
    h: HD_HEIGHT,
};

#[derive(SketchComponents)]
pub struct AutoWaveFract {
    hub: ControlHub<Timing>,
    gpu: gpu::GpuState<()>,
}

#[uniforms(banks = 24)]
struct ShaderParams {}

pub fn init(app: &App, ctx: &Context) -> AutoWaveFract {
    let wr = ctx.window_rect();

    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "auto_wave_fract.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams::default();

    let gpu = gpu::GpuState::new_procedural(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "auto_wave_fract.wgsl"),
        &params,
    );

    AutoWaveFract { hub, gpu }
}

impl Sketch for AutoWaveFract {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();
        let mut params = ShaderParams::from((&wr, &self.hub));
        params.set("a3", self.hub.animation.beats());
        self.gpu.update_params(app, wr.resolution_u32(), &params);
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &Context) {
        frame.clear(BLACK);

        let n_lines = self.hub.get("n_lines").max(1.0) as u32;
        let segments = self.hub.get("segments").max(1.0) as u32;
        let points_per_segment =
            self.hub.get("points_per_segment").max(1.0) as u32;
        let passes = self.hub.get("passes").max(1.0) as u32;

        let total_points = n_lines
            .saturating_mul(segments)
            .saturating_mul(points_per_segment)
            .saturating_mul(passes);
        let point_vertices = total_points.saturating_mul(6);
        let total_vertices = 3u32.saturating_add(point_vertices).min(6_000_000);

        self.gpu.render_procedural(&frame, total_vertices);
    }
}
