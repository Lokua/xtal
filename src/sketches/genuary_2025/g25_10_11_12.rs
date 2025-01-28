use nannou::prelude::*;

use crate::framework::prelude::*;

// ~/Documents/Live/2025/2025.01.11 Lattice - Spiral

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "g25_10_11_12",
    display_name: "Genuary 10-12: Spiral (Automated)",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(680),
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

#[derive(SketchComponents)]
pub struct Model {
    animation: Animation<FrameTiming>,
    controls: Controls,
    wr: WindowRect,
    gpu: gpu::GpuState<()>,
    midi: MidiControls,
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let animation = Animation::new(FrameTiming::new(SKETCH_CONFIG.bpm));

    let disabled = |_controls: &Controls| true;

    let controls = Controls::with_previous(vec![
        // 1 "pass" = 1 million vertices
        Control::slider("passes", 1.0, (1.0, 20.0), 1.0),
        Control::slider_x("n_lines", 64.0, (1.0, 256.0), 1.0, disabled),
        Control::slider_x(
            "points_per_segment",
            100.0,
            (10.0, 20_000.0),
            10.0,
            disabled,
        ),
        Control::slider("point_size", 0.001, (0.0005, 0.01), 0.0001),
        Control::slider("harmonic_influence", 0.2, (0.01, 10.0), 0.01),
        Control::Separator {}, // -----------------------------------
        Control::checkbox("invert", false),
        Control::checkbox("animate_bg", false),
        Control::checkbox("animate_angle_offset", false),
        Control::slider("bg_brightness", 1.5, (0.0, 5.0), 0.01),
        Control::slider("phase_animation_mult", 1.0, (0.0, 1.0), 0.125),
        Control::Separator {}, // -----------------------------------
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
        to_absolute_path(file!(), "./g25_10_11_12.wgsl"),
        &params,
        true,
    );

    let midi = MidiControlBuilder::new()
        .control_mapped("n_lines", (0, 1), (1.0, 256.0), 0.5)
        .control_mapped("points_per_segment", (0, 2), (10.0, 20_000.0), 0.75)
        // ---
        .control_mapped("noise_scale", (0, 3), (0.0, 0.1), 0.025)
        .control_mapped("angle_variation", (0, 4), (0.0, TAU), 0.2)
        .control_mapped("offset_mult", (0, 5), (0.0, 10.0), 0.0)
        .control("circle_r_min", (0, 6), 0.0)
        .control("circle_r_max", (0, 7), 1.0)
        // ---
        .control("wave_amp", (0, 8), 0.0)
        .control("steep_amp", (0, 9), 0.0)
        .control("quant_amp", (0, 10), 0.0)
        .control("stripe_amp", (0, 11), 0.0)
        .control_mapped("steep_freq", (0, 12), (0.00, 64.0), 1.0)
        .control_mapped("quant_freq", (0, 13), (0.00, 64.0), 1.0)
        .control_mapped("stripe_freq", (0, 14), (0.00, 64.0), 1.0)
        .control_mapped("wave_freq", (0, 15), (0.00, 64.0), 1.0)
        .build();

    Model {
        animation,
        controls,
        wr,
        gpu,
        midi,
    }
}

pub fn update(app: &App, m: &mut Model, _update: Update) {
    let params = ShaderParams {
        resolution: [m.wr.w(), m.wr.h(), 0.0, 0.0],
        a: [-0.9, 0.0, 0.9, 0.0],
        b: [
            m.midi.get("points_per_segment"),
            m.midi.get("noise_scale"),
            m.midi.get("angle_variation"),
            m.midi.get("n_lines"),
        ],
        c: [
            m.controls.float("point_size"),
            m.midi.get("circle_r_min"),
            m.midi.get("circle_r_max"),
            m.midi.get("offset_mult"),
        ],
        d: [
            m.controls.float("bg_brightness"),
            m.animation.ping_pong(64.0),
            bool_to_f32(m.controls.bool("invert")),
            bool_to_f32(m.controls.bool("animate_angle_offset")),
        ],
        e: [
            m.midi.get("wave_amp"),
            m.midi.get("wave_freq").ceil(),
            m.midi.get("stripe_amp"),
            m.midi.get("stripe_freq").ceil(),
        ],
        f: [
            bool_to_f32(m.controls.bool("animate_bg")),
            m.midi.get("steep_amp"),
            m.midi.get("steep_freq").ceil(),
            m.controls.float("steepness"),
        ],
        g: [
            m.midi.get("quant_amp"),
            m.midi.get("quant_freq").ceil(),
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

    m.gpu.update_params(app, &params);
    m.controls.mark_unchanged();
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    frame.clear(WHITE);

    let points_per_line = m.midi.get("points_per_segment") as u32;
    let n_lines = m.midi.get("n_lines") as u32;
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
            m.animation.loop_progress(time) * TAU
        } else {
            (1.0 - m.animation.loop_progress(time)) * TAU
        }
    } else {
        m.controls.float(&phase_param)
    }
}
