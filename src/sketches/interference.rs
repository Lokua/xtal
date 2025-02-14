use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "interference",
    display_name: "Interference",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(580),
};

#[derive(SketchComponents)]
pub struct Model {
    #[allow(dead_code)]
    animation: Animation<Timing>,
    controls: Controls,
    wr: WindowRect,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderParams {
    // w, h, ..unused
    resolution: [f32; 4],

    // wave1_frequency, wave1_angle, wave2_frequency, wave2_angle
    a: [f32; 4],

    // wave1_phase, wave2_phase, wave1_y_influence, wave2_y_influence
    b: [f32; 4],

    // unused, type_mix, unused, checkerboard
    c: [f32; 4],

    // curve_freq_x, curve_freq_y, wave_distort, smoothing
    d: [f32; 4],

    // wave1_amp, wave2_amp, ..unused
    e: [f32; 4],
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let animation = Animation::new(Timing::new(SKETCH_CONFIG.bpm));

    let controls = Controls::with_previous(vec![
        Control::checkbox("animate_wave1_phase", false),
        Control::slider("wave1_amp", 1.0, (0.0, 2.0), 0.001),
        Control::slider_norm("wave1_frequency", 0.02),
        Control::slider("wave1_angle", 0.0, (0.0, 1.0), 0.125),
        Control::slider_x(
            "wave1_phase",
            0.0,
            (0.0, 1.0),
            0.0001,
            |controls: &Controls| controls.bool("animate_wave1_phase"),
        ),
        Control::slider_norm("wave1_y_influence", 0.5),
        Control::Separator {}, // ------------------------------------------
        Control::checkbox("animate_wave2_phase", false),
        Control::slider("wave2_amp", 1.0, (0.0, 2.0), 0.001),
        Control::slider_norm("wave2_frequency", 0.02),
        Control::slider("wave2_angle", 0.0, (0.0, 1.0), 0.125),
        Control::slider_x(
            "wave2_phase",
            0.0,
            (0.0, 1.0),
            0.0001,
            |controls: &Controls| controls.bool("animate_wave2_phase"),
        ),
        Control::slider_norm("wave2_y_influence", 0.5),
        Control::Separator {}, // ------------------------------------------
        Control::checkbox("checkerboard", false),
        Control::slider_norm("type_mix", 0.0),
        Control::slider("curve_freq_x", 0.3, (0.0, 2.0), 0.001),
        Control::slider("curve_freq_y", 0.3, (0.0, 2.0), 0.001),
        Control::slider_norm("wave_distort", 0.4),
        Control::slider_norm("smoothing", 0.5),
    ]);

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
        wr.resolution_u32(),
        to_absolute_path(file!(), "./interference.wgsl"),
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
        a: [
            m.controls.float("wave1_frequency"),
            m.controls.float("wave1_angle"),
            m.controls.float("wave2_frequency"),
            m.controls.float("wave2_angle"),
        ],
        b: [
            if m.controls.bool("animate_wave1_phase") {
                m.animation.r_ramp(
                    &[kfr((0.0, 1.0), 2.0)],
                    0.0,
                    1.0,
                    Easing::Linear,
                )
            } else {
                m.controls.float("wave1_phase")
            },
            if m.controls.bool("animate_wave2_phase") {
                m.animation.r_ramp(
                    &[kfr((0.0, 1.0), 2.0)],
                    1.0,
                    1.0,
                    Easing::Linear,
                )
            } else {
                m.controls.float("wave2_phase")
            },
            m.controls.float("wave1_y_influence"),
            m.controls.float("wave2_y_influence"),
        ],
        c: [
            0.0,
            m.controls.float("type_mix"),
            0.0,
            bool_to_f32(m.controls.bool("checkerboard")),
        ],
        d: [
            m.controls.float("curve_freq_x"),
            m.controls.float("curve_freq_y"),
            m.controls.float("wave_distort"),
            m.controls.float("smoothing"),
        ],
        e: [
            m.controls.float("wave1_amp"),
            m.controls.float("wave2_amp"),
            0.0,
            0.0,
        ],
    };

    m.gpu.update_params(app, m.wr.resolution_u32(), &params);
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    frame.clear(BLACK);
    m.gpu.render(&frame);
}
