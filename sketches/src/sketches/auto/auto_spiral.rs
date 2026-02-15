use nannou::prelude::*;
use xtal::prelude::*;

use crate::sketches::common::{HD_HEIGHT, HD_WIDTH};

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "auto_spiral",
    display_name: "Auto Spiral",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: HD_WIDTH,
    h: HD_HEIGHT,
};

#[derive(SketchComponents)]
pub struct AutoSpiral {
    hub: ControlHub<Timing>,
    gpu: gpu::GpuState<()>,
}

#[uniforms(banks = 9)]
struct ShaderParams {}

pub fn init(app: &App, ctx: &Context) -> AutoSpiral {
    let wr = ctx.window_rect();

    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "auto_spiral.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams::default();

    let gpu = gpu::GpuState::new_procedural(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "auto_spiral.wgsl"),
        &params,
    );

    AutoSpiral { hub, gpu }
}

impl Sketch for AutoSpiral {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();
        let mut params = ShaderParams::from((&wr, &self.hub));
        params.set("a3", self.hub.animation.beats());

        params.set("b1", -0.9);
        params.set("b2", 0.0);
        params.set("b3", 0.9);
        params.set("b4", 0.0);

        let offset_mult = ternary!(
            self.hub.bool("offset_mult_10"),
            self.hub.get("offset_mult") * 10.0,
            self.hub.get("offset_mult")
        );
        params.set("d4", offset_mult);

        params.set("h3", get_phase(self, "quant", 96.0));
        params.set("h4", get_phase(self, "steep", 192.0));
        params.set("i1", get_phase(self, "wave", 128.0));
        params.set("i2", get_phase(self, "stripe", 224.0));

        self.gpu.update_params(app, wr.resolution_u32(), &params);
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &Context) {
        frame.clear(WHITE);

        let points_per_line = self.hub.get("points_per_segment") as u32;
        let n_lines = self.hub.get("n_lines") as u32;
        let total_points = points_per_line * n_lines;
        let density = self.hub.get("passes") as u32;
        let spiral_vertices = total_points * 6 * density;
        let background_vertices = 3;
        let total_vertices = background_vertices + spiral_vertices;

        self.gpu.render_procedural(&frame, total_vertices);
    }
}

fn get_phase(
    spiral: &AutoSpiral,
    param_name: &str,
    animation_time: f32,
) -> f32 {
    let animate_param = format!("animate_{}_phase", param_name);
    let invert_param = format!("invert_animate_{}_phase", param_name);
    let phase_param = format!("{}_phase", param_name);

    let phase = spiral.hub.get(&phase_param);
    let phase_animation_mult = spiral.hub.get("phase_animation_mult").max(0.25);
    let global_slowdown = 8.0;
    let time = animation_time * phase_animation_mult * global_slowdown;

    if spiral.hub.bool(&animate_param) {
        if !time.is_finite() || time <= 0.0 {
            return phase;
        }

        let ramp = spiral.hub.animation.ramp(time);
        if !ramp.is_finite() {
            return phase;
        }

        if spiral.hub.bool(&invert_param) {
            ramp * TAU
        } else {
            (1.0 - ramp) * TAU
        }
    } else {
        phase
    }
}
