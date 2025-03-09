use nannou::prelude::*;

use crate::framework::prelude::*;

// Live/2025/Lattice - Gen 26 - Symmetry

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "g25_26_symmetry",
    display_name: "Genuary 26: Symmetry",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    // h: 700,
    h: 1244,
    gui_w: None,
    gui_h: Some(640),
};

#[derive(SketchComponents)]
pub struct Template {
    controls: ControlHub<Timing>,
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

pub fn init(app: &App, ctx: &LatticeContext) -> Template {
    let controls = ControlHub::from_path(
        to_absolute_path(file!(), "g25_26_symmetry.yaml"),
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
        to_absolute_path(file!(), "g25_26_symmetry.wgsl"),
        &params,
        true,
    );

    Template { controls, gpu }
}

impl Sketch for Template {
    fn update(&mut self, app: &App, _update: Update, ctx: &LatticeContext) {
        self.controls.update();

        let params = ShaderParams {
            resolution: [
                ctx.window_rect().w(),
                ctx.window_rect().h(),
                0.0,
                0.0,
            ],
            a: [
                self.controls.get("fractal_mix"),
                self.controls.get("distort_mix"),
                self.controls.get("wave_mix"),
                self.controls.get("fractal_count"),
            ],
            b: [
                self.controls.get("wave_freq"),
                self.controls.get("wave_scale"),
                self.controls.get("wave_x"),
                self.controls.get("wave_y"),
            ],
            c: [
                self.controls.get("distort_freq"),
                self.controls.get("signal_mix"),
                self.controls.get("fractal_grid_scale"),
                self.controls.get("fractal_scale"),
            ],
            d: [
                self.controls.get("distort_angle_offset"),
                self.controls.get("signal_steps"),
                self.controls.get("fractal_color_scale"),
                self.controls.get("fractal_grid_mix"),
            ],
            e: [
                self.controls.get("mask_radius"),
                self.controls.get("mask_falloff"),
                self.controls.get("mask_x"),
                self.controls.get("mask_y"),
            ],
        };

        self.gpu.update_params(
            app,
            ctx.window_rect().resolution_u32(),
            &params,
        );
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &LatticeContext) {
        frame.clear(BLACK);
        self.gpu.render(&frame);
    }
}
