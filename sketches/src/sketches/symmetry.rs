use nannou::prelude::*;

use xtal::prelude::*;

// Live/2025/Xtal - Gen 26 - Symmetry

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "symmetry",
    display_name: "Symmetry",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    // h: 700,
    h: 1244,
};

#[derive(SketchComponents)]
pub struct Template {
    hub: ControlHub<Timing>,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderParams {
    // w, h, ..unused
    resolution: [f32; 4],

    // fractal_mix, distort_mix, wave_mix, fractal_count
    a: [f32; 4],

    // wave_freq, wave_scale, wave_x, wave_y
    b: [f32; 4],

    // distort_freq, signal_mix, fractal_grid_scale, fractal_scale
    c: [f32; 4],

    // unused, signal_steps, fractal_color_scale, fractal_grid_mix
    d: [f32; 4],

    // mask_radius, mask_falloff, mask_x, mask_y
    e: [f32; 4],
}

pub fn init(app: &App, ctx: &Context) -> Template {
    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "symmetry.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
        c: [0.0; 4],
        d: [0.0; 4],
        e: [0.0; 4],
    };

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        ctx.window_rect().resolution_u32(),
        to_absolute_path(file!(), "symmetry.wgsl"),
        &params,
        0,
        true,
    );

    Template { hub, gpu }
}

impl Sketch for Template {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let params = ShaderParams {
            resolution: [
                ctx.window_rect().w(),
                ctx.window_rect().h(),
                0.0,
                0.0,
            ],
            a: [
                self.hub.get("fractal_mix"),
                self.hub.get("distort_mix"),
                ternary!(
                    self.hub.bool("animate_wave_mix"),
                    self.hub.get("wave_mix_animation"),
                    self.hub.get("wave_mix")
                ),
                self.hub.get("fractal_count"),
            ],
            b: [
                self.hub.get("wave_freq"),
                self.hub.get("wave_scale"),
                ternary!(
                    self.hub.bool("animate_wave_x"),
                    self.hub.get("wave_x_animation"),
                    self.hub.get("wave_x")
                ),
                self.hub.get("wave_y"),
            ],
            c: [
                ternary!(
                    self.hub.bool("animate_distort_freq"),
                    self.hub.get("distort_freq_animation"),
                    self.hub.get("distort_freq")
                ),
                self.hub.get("signal_mix"),
                self.hub.get("fractal_grid_scale"),
                self.hub.get("fractal_scale"),
            ],
            d: [
                ternary!(
                    self.hub.bool("animate_distort_angle_offset"),
                    self.hub.get("distort_angle_offset_animation"),
                    self.hub.get("distort_angle_offset")
                ),
                self.hub.get("signal_steps"),
                self.hub.get("fractal_color_scale"),
                self.hub.get("fractal_grid_mix"),
            ],
            e: [
                ternary!(
                    self.hub.bool("animate_mask_radius"),
                    self.hub.get("mask_radius_animation"),
                    self.hub.get("mask_radius")
                ),
                self.hub.get("mask_falloff"),
                ternary!(
                    self.hub.bool("animate_mask_x"),
                    self.hub.get("mask_x_animation"),
                    self.hub.get("mask_x")
                ),
                ternary!(
                    self.hub.bool("animate_mask_y"),
                    self.hub.get("mask_y_animation"),
                    self.hub.get("mask_y")
                ),
            ],
        };

        self.gpu.update_params(
            app,
            ctx.window_rect().resolution_u32(),
            &params,
        );
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &Context) {
        frame.clear(BLACK);
        self.gpu.render(&frame);
    }
}
