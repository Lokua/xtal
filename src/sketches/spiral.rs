use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "spiral",
    display_name: "Spiral",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 90.0,
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

    // point_size, circle_r_min, circle_r_max, offset_mult
    c: [f32; 4],

    // bg_brightness, time, invert, animate_angle_offset
    d: [f32; 4],

    // wave_amp, wave_freq, stripe_amp, stripe_freq
    e: [f32; 4],

    // animate_bg, steep_amp, steep_freq, steepness
    f: [f32; 4],

    // quant_amp, quant_freq, quant_phase, steep_phase
    g: [f32; 4],

    // wave_phase, stripe_phase, harmonic_influence, unused
    h: [f32; 4],
}

#[derive(LegacySketchComponents)]
pub struct Model {
    animation: Animation<Timing>,
    controls: Controls,
    wr: WindowRect,
    gpu: gpu::GpuState<()>,
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let animation = Animation::new(Timing::new(Bpm::new(SKETCH_CONFIG.bpm)));

    let controls = Controls::with_previous(vec![
        // 1 "pass" = 1 million vertices
        Control::slider("passes", 1.0, (1.0, 20.0), 1.0),
        Control::slider("n_lines", 64.0, (1.0, 256.0), 1.0),
        Control::slider("points_per_segment", 100.0, (10.0, 20_000.0), 10.0),
        Control::slider("point_size", 0.001, (0.0005, 0.01), 0.0001),
        Control::slider("harmonic_influence", 0.2, (0.01, 10.0), 0.01),
        Control::Separator {}, // -----------------------------------
        Control::slider("noise_scale", 0.001, (0.0, 0.1), 0.0001),
        Control::slider("angle_variation", 0.2, (0.0, TAU), 0.1),
        Control::checkbox("offset_mult_10", false),
        Control::slider("offset_mult", 0.9, (0.0, 10.0), 0.001),
        Control::slide("circle_r_min", 0.5),
        Control::slide("circle_r_max", 0.9),
        Control::Separator {}, // -----------------------------------
        Control::checkbox("invert", false),
        Control::checkbox("animate_bg", false),
        Control::checkbox("animate_angle_offset", false),
        Control::slider("bg_brightness", 1.5, (0.0, 5.0), 0.01),
        Control::slider("phase_animation_mult", 1.0, (0.0, 1.0), 0.125),
        Control::Separator {}, // -----------------------------------
        Control::slider("wave_amp", 0.0, (0.0, 0.5), 0.0001),
        Control::slider("wave_freq", 10.0, (0.00, 64.0), 1.0),
        Control::checkbox("animate_wave_phase", false),
        Control::checkbox("invert_animate_wave_phase", false),
        Control::slider_x(
            "wave_phase",
            0.0,
            (0.0, TAU),
            0.001,
            |controls: &Controls| controls.bool("animate_wave_phase"),
        ),
        Control::Separator {}, // -----------------------------------
        Control::slider("stripe_amp", 0.0, (0.0, 0.5), 0.0001),
        Control::slider("stripe_freq", 10.0, (0.00, 64.0), 1.0),
        Control::checkbox("animate_stripe_phase", false),
        Control::checkbox("invert_animate_stripe_phase", false),
        Control::slider_x(
            "stripe_phase",
            0.0,
            (0.0, TAU),
            0.001,
            |controls: &Controls| controls.bool("animate_stripe_phase"),
        ),
        Control::Separator {}, // -----------------------------------
        Control::slider("steep_amp", 0.0, (0.0, 0.5), 0.0001),
        Control::slider("steep_freq", 10.0, (0.00, 64.0), 1.0),
        Control::slider("steepness", 10.0, (1.0, 100.0), 1.0),
        Control::checkbox("animate_steep_phase", false),
        Control::checkbox("invert_animate_steep_phase", false),
        Control::slider_x(
            "steep_phase",
            0.0,
            (0.0, TAU),
            0.001,
            |controls: &Controls| controls.bool("animate_steep_phase"),
        ),
        Control::Separator {}, // -----------------------------------
        Control::slider("quant_amp", 0.0, (0.0, 0.5), 0.0001),
        Control::slider("quant_freq", 10.0, (0.00, 64.0), 1.0),
        Control::checkbox("animate_quant_phase", false),
        Control::checkbox("invert_animate_quant_phase", false),
        Control::slider_x(
            "quant_phase",
            0.0,
            (0.0, TAU),
            0.001,
            |controls: &Controls| controls.bool("animate_quant_phase"),
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
        to_absolute_path(file!(), "./spiral.wgsl"),
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
            m.controls.float("circle_r_min"),
            m.controls.float("circle_r_max"),
            if m.controls.bool("offset_mult_10") {
                m.controls.float("offset_mult") * 10.0
            } else {
                m.controls.float("offset_mult")
            },
        ],
        d: [
            m.controls.float("bg_brightness"),
            m.animation.tri(64.0),
            bool_to_f32(m.controls.bool("invert")),
            bool_to_f32(m.controls.bool("animate_angle_offset")),
        ],
        e: [
            m.controls.float("wave_amp"),
            m.controls.float("wave_freq"),
            m.controls.float("stripe_amp"),
            m.controls.float("stripe_freq"),
        ],
        f: [
            bool_to_f32(m.controls.bool("animate_bg")),
            m.controls.float("steep_amp"),
            m.controls.float("steep_freq"),
            m.controls.float("steepness"),
        ],
        g: [
            m.controls.float("quant_amp"),
            m.controls.float("quant_freq"),
            get_phase(&m, "quant", 24.0),
            get_phase(&m, "steep", 48.0),
        ],
        h: [
            get_phase(&m, "wave", 32.0),
            get_phase(&m, "stripe", 56.0),
            m.controls.float("harmonic_influence"),
            0.0,
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

fn get_phase(m: &Model, param_name: &str, animation_time: f32) -> f32 {
    let animate_param = format!("animate_{}_phase", param_name);
    let invert_param = format!("invert_animate_{}_phase", param_name);
    let phase_param = format!("{}_phase", param_name);
    let time = animation_time * m.controls.float("phase_animation_mult");

    if m.controls.bool(&animate_param) {
        if m.controls.bool(&invert_param) {
            m.animation.loop_phase(time) * TAU
        } else {
            (1.0 - m.animation.loop_phase(time)) * TAU
        }
    } else {
        m.controls.float(&phase_param)
    }
}
