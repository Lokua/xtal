use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "spiral_lines",
    display_name: "Spiral | Lines Version",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 120.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(960),
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderParams {
    // w, h, ..unused
    resolution: [f32; 4],

    // start_x, start_y, end_x, end_y
    a: [f32; 4],

    // points_per_segment, noise_scale, angle_variation, n_lines
    b: [f32; 4],

    // point_size, col_freq, width, distortion
    c: [f32; 4],

    // clip_start, clip_grade, distortion_intensity, row_freq
    d: [f32; 4],

    // stripe_step, stripe_mix, stripe_amp, stripe_freq
    e: [f32; 4],

    // unused, circle_radius, circle_phase, wave_amp
    f: [f32; 4],

    // center_count, center_spread, center_falloff, circle_force
    g: [f32; 4],

    // stripe_min, stripe_phase, harmonic_influence, stripe_max
    h: [f32; 4],
}

#[derive(SketchComponents)]
pub struct SpiralLines {
    #[allow(dead_code)]
    animation: Animation<Timing>,
    controls: Controls,
    gpu: gpu::GpuState<()>,
}

pub fn init(app: &App, ctx: LatticeContext) -> SpiralLines {
    let wr = ctx.window_rect();
    let animation = Animation::new(Timing::new(ctx.bpm()));

    let controls = Controls::with_previous(vec![
        Control::slider("passes", 1.0, (1.0, 20.0), 1.0),
        Control::slider("n_lines", 64.0, (1.0, 256.0), 1.0),
        Control::slider("points_per_segment", 100.0, (10.0, 20_000.0), 10.0),
        Control::slider("point_size", 0.001, (0.0005, 0.01), 0.0001),
        Control::slider("harmonic_influence", 0.2, (0.01, 10.0), 0.01),
        Control::Separator {}, // -----------------------------------
        Control::slider("noise_scale", 0.00001, (0.0, 0.002), 0.00001),
        Control::slider("angle_variation", 0.2, (0.0, TAU), 0.1),
        Control::Separator {}, // -----------------------------------
        Control::slider("col_freq", 0.5, (0.01, 256.0), 0.01),
        Control::slider("row_freq", 0.5, (0.01, 256.0), 0.01),
        Control::slider("width", 1.0, (0.01, 2.00), 0.01),
        Control::slider("distortion", 0.9, (0.0, 10.0), 0.01),
        Control::slider("wave_amp", 1.0, (0.0001, 0.5), 0.0001),
        Control::Separator {}, // -----------------------------------
        Control::slider("center_count", 1.0, (0.0, 10.0), 1.0),
        Control::slider("center_spread", 1.0, (0.0, 2.0), 0.001),
        Control::slider("center_falloff", 1.0, (0.01, 10.0), 0.01),
        Control::slider("circle_radius", 0.5, (0.001, 2.0), 0.001),
        Control::slider("circle_force", 0.5, (0.001, 5.0), 0.001),
        Control::slider("circle_phase", 0.0, (0.0, TAU), 0.1),
        Control::Separator {}, // -----------------------------------
        Control::slide("clip_start", 0.8),
        Control::slide("clip_grade", 0.3),
        Control::Separator {}, // -----------------------------------
        // Control::checkbox("animate_stripe_phase", false),
        // Control::checkbox("invert_animate_stripe_phase", false),
        Control::slider("stripe_amp", 0.0, (0.0, 0.5), 0.0001),
        Control::slider("stripe_freq", 10.0, (0.00, 64.0), 1.0),
        Control::slide("stripe_mix", 0.5),
        Control::slide("stripe_step", 0.0),
        Control::slide("stripe_min", 0.0),
        Control::slide("stripe_max", 1.0),
        Control::slider_x(
            "stripe_phase",
            0.0,
            (0.0, TAU),
            0.001,
            |controls: &Controls| controls.bool("animate_stripe_phase"),
        ),
    ]);

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
        c: [0.0; 4],
        d: [0.0; 4],
        e: [0.0; 4],
        f: [0.0; 4],
        g: [0.0; 4],
        h: [0.0; 4],
    };

    let gpu = gpu::GpuState::new_procedural(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "spiral_lines.wgsl"),
        &params,
        true,
    );

    SpiralLines {
        animation,
        controls,
        gpu,
    }
}

impl Sketch for SpiralLines {
    fn update(&mut self, app: &App, _update: Update, ctx: &LatticeContext) {
        let wr = ctx.window_rect();
        let params = ShaderParams {
            resolution: [wr.w(), wr.h(), 0.0, 0.0],
            a: [-0.9, 0.0, 0.9, 0.0],
            b: [
                self.controls.float("points_per_segment"),
                self.controls.float("noise_scale"),
                self.controls.float("angle_variation"),
                self.controls.float("n_lines"),
            ],
            c: [
                self.controls.float("point_size"),
                self.controls.float("col_freq"),
                self.controls.float("width"),
                self.controls.float("distortion"),
            ],
            d: [
                self.controls.float("clip_start"),
                self.controls.float("clip_grade"),
                0.0,
                self.controls.float("row_freq"),
            ],
            e: [
                self.controls.float("stripe_step"),
                self.controls.float("stripe_mix"),
                self.controls.float("stripe_amp"),
                self.controls.float("stripe_freq"),
            ],
            f: [
                0.0,
                self.controls.float("circle_radius"),
                self.controls.float("circle_phase"),
                self.controls.float("wave_amp"),
            ],
            g: [
                self.controls.float("center_count"),
                self.controls.float("center_spread"),
                self.controls.float("center_falloff"),
                self.controls.float("circle_force"),
            ],
            h: [
                self.controls.float("stripe_min"),
                self.controls.float("stripe_phase"),
                self.controls.float("harmonic_influence"),
                self.controls.float("stripe_max"),
            ],
        };

        self.gpu.update_params(app, wr.resolution_u32(), &params);
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &LatticeContext) {
        frame.clear(WHITE);

        let points_per_line = self.controls.float("points_per_segment") as u32;
        let n_lines = self.controls.float("n_lines") as u32;
        let total_points = points_per_line * n_lines;
        let density = self.controls.float("passes") as u32;
        let spiral_vertices = total_points * 6 * density;
        let background_vertices = 3;
        let total_vertices = background_vertices + spiral_vertices;

        self.gpu.render_procedural(&frame, total_vertices);
    }
}
