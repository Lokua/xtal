use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "g25_26_symmetry",
    display_name: "Genuary 26: Symmetry",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(640),
};

#[derive(SketchComponents)]
pub struct Model {
    #[allow(dead_code)]
    animation: Animation<FrameTiming>,
    controls: Controls,
    wr: WindowRect,
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

    // distort_freq, signal_mix, unused, fractal_scale
    c: [f32; 4],

    // signal_contrast, signal_steps, fractal_color_scale, unused
    d: [f32; 4],

    // ...unused
    e: [f32; 4],
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let animation = Animation::new(FrameTiming::new(SKETCH_CONFIG.bpm));

    let controls = Controls::with_previous(vec![
        Control::slider_norm("wave_mix", 0.5),
        Control::slider_norm("wave_freq", 0.5),
        Control::slider_norm("wave_scale", 0.5),
        Control::slider_norm("wave_x", 0.5),
        Control::slider_norm("wave_y", 0.5),
        Control::Separator {}, // -------------------
        Control::slider_norm("distort_mix", 0.5),
        Control::slider_norm("distort_freq", 0.5),
        Control::Separator {}, // -------------------
        Control::slider_norm("fractal_mix", 0.5),
        Control::slider_norm("fractal_count", 0.5),
        Control::slider_norm("fractal_scale", 0.5),
        Control::slider_norm("fractal_color_scale", 0.5),
        Control::Separator {}, // -------------------
        Control::slider_norm("signal_mix", 0.5),
        Control::slider_norm("signal_contrast", 0.5),
        Control::slider_norm("signal_steps", 0.5),
        Control::Separator {}, // -------------------
        Control::slider_norm("c3", 0.5),
        Control::slider_norm("d4", 0.5),
        Control::slider_norm("e1", 0.5),
        Control::slider_norm("e2", 0.5),
        Control::slider_norm("e3", 0.5),
        Control::slider_norm("e4", 0.5),
    ]);

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
        c: [0.0; 4],
        d: [0.0; 4],
        e: [0.0; 4],
    };

    let gpu = gpu::GpuState::new_full_screen(
        app,
        to_absolute_path(file!(), "g25_26_symmetry.wgsl"),
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
            m.controls.float("fractal_mix"),
            m.controls.float("distort_mix"),
            m.controls.float("wave_mix"),
            m.controls.float("fractal_count"),
        ],
        b: [
            m.controls.float("wave_freq"),
            m.controls.float("wave_scale"),
            m.controls.float("wave_x"),
            m.controls.float("wave_y"),
        ],
        c: [
            m.controls.float("distort_freq"),
            m.controls.float("signal_mix"),
            m.controls.float("c3"),
            m.controls.float("fractal_scale"),
        ],
        d: [
            m.controls.float("signal_contrast"),
            m.controls.float("signal_steps"),
            m.controls.float("fractal_color_scale"),
            m.controls.float("d4"),
        ],
        e: [
            m.controls.float("e1"),
            m.controls.float("e2"),
            m.controls.float("e3"),
            m.controls.float("e4"),
        ],
    };

    m.gpu.update_params(app, &params);
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    frame.clear(BLACK);
    m.gpu.render(&frame);
}
