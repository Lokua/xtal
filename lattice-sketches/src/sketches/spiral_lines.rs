use nannou::prelude::*;

use lattice::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "spiral_lines",
    display_name: "Spiral | Lines Version",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 120.0,
    w: 700,
    h: 700,
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
    hub: ControlHub<Timing>,
    gpu: gpu::GpuState<()>,
}

pub fn init(app: &App, ctx: &Context) -> SpiralLines {
    let wr = ctx.window_rect();
    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "spiral_lines.yaml"),
        Timing::new(ctx.bpm()),
    );

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

    SpiralLines { hub, gpu }
}

impl Sketch for SpiralLines {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();

        let params = ShaderParams {
            resolution: [wr.w(), wr.h(), 0.0, 0.0],
            a: [-0.9, 0.0, 0.9, 0.0],
            b: [
                self.hub.get("points_per_segment"),
                self.hub.get("noise_scale"),
                self.hub.get("angle_variation"),
                self.hub.get("n_lines"),
            ],
            c: [
                self.hub.get("point_size"),
                self.hub.get("col_freq"),
                self.hub.get("width"),
                self.hub.get("distortion"),
            ],
            d: [
                self.hub.get("clip_start"),
                self.hub.get("clip_grade"),
                0.0,
                self.hub.get("row_freq"),
            ],
            e: [
                self.hub.get("stripe_step"),
                self.hub.get("stripe_mix"),
                self.hub.get("stripe_amp"),
                self.hub.get("stripe_freq"),
            ],
            f: [
                0.0,
                self.hub.get("circle_radius"),
                self.hub.get("circle_phase"),
                self.hub.get("wave_amp"),
            ],
            g: [
                self.hub.get("center_count"),
                self.hub.get("center_spread"),
                self.hub.get("center_falloff"),
                self.hub.get("circle_force"),
            ],
            h: [
                self.hub.get("stripe_min"),
                self.hub.get("stripe_phase"),
                self.hub.get("harmonic_influence"),
                self.hub.get("stripe_max"),
            ],
        };

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
