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

#[derive(LegacySketchComponents)]
pub struct Model {
    #[allow(dead_code)]
    animation: Animation<Timing>,
    controls: Controls,
    wr: WindowRect,
    gpu: gpu::GpuState<()>,
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let animation = Animation::new(Timing::new(Bpm::new(SKETCH_CONFIG.bpm)));

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

    Model {
        animation,
        controls,
        wr,
        gpu,
    }
}

pub fn update(app: &App, m: &mut Model, _update: Update) {
    let params = ShaderParams {
        resolution: [m.wr.w(), m.wr.h(), 0.0, 0.0],
        a: [-0.9, 0.0, 0.9, 0.0],
        b: [
            m.controls.float("points_per_segment"),
            m.controls.float("noise_scale"),
            m.controls.float("angle_variation"),
            m.controls.float("n_lines"),
        ],
        c: [
            m.controls.float("point_size"),
            m.controls.float("col_freq"),
            m.controls.float("width"),
            m.controls.float("distortion"),
        ],
        d: [
            m.controls.float("clip_start"),
            m.controls.float("clip_grade"),
            0.0,
            m.controls.float("row_freq"),
        ],
        e: [
            m.controls.float("stripe_step"),
            m.controls.float("stripe_mix"),
            m.controls.float("stripe_amp"),
            m.controls.float("stripe_freq"),
        ],
        f: [
            0.0,
            m.controls.float("circle_radius"),
            m.controls.float("circle_phase"),
            m.controls.float("wave_amp"),
        ],
        g: [
            m.controls.float("center_count"),
            m.controls.float("center_spread"),
            m.controls.float("center_falloff"),
            m.controls.float("circle_force"),
        ],
        h: [
            m.controls.float("stripe_min"),
            m.controls.float("stripe_phase"),
            m.controls.float("harmonic_influence"),
            m.controls.float("stripe_max"),
        ],
    };

    m.gpu.update_params(app, m.wr.resolution_u32(), &params);
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    frame.clear(WHITE);

    let points_per_line = m.controls.float("points_per_segment") as u32;
    let n_lines = m.controls.float("n_lines") as u32;
    let total_points = points_per_line * n_lines;
    let density = m.controls.float("passes") as u32;
    let spiral_vertices = total_points * 6 * density;
    let background_vertices = 3;
    let total_vertices = background_vertices + spiral_vertices;

    m.gpu.render_procedural(&frame, total_vertices);
}
