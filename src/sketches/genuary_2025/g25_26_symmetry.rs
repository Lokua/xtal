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
pub struct Model {
    controls: ControlScript<Timing>,
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

    // distort_freq, signal_mix, fractal_grid_scale, fractal_scale
    c: [f32; 4],

    // unused, signal_steps, fractal_color_scale, fractal_grid_mix
    d: [f32; 4],

    // mask_radius, mask_falloff, mask_x, mask_y
    e: [f32; 4],
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let controls = ControlScript::new(
        to_absolute_path(file!(), "g25_26_symmetry.yaml"),
        Timing::new(SKETCH_CONFIG.bpm),
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
        wr.resolution_u32(),
        to_absolute_path(file!(), "g25_26_symmetry.wgsl"),
        &params,
        true,
    );

    Model { controls, wr, gpu }
}

pub fn update(app: &App, m: &mut Model, _update: Update) {
    m.controls.update();

    let params = ShaderParams {
        resolution: [m.wr.w(), m.wr.h(), 0.0, 0.0],
        a: [
            m.controls.get("fractal_mix"),
            m.controls.get("distort_mix"),
            m.controls.get("wave_mix"),
            m.controls.get("fractal_count"),
        ],
        b: [
            m.controls.get("wave_freq"),
            m.controls.get("wave_scale"),
            m.controls.get("wave_x"),
            m.controls.get("wave_y"),
        ],
        c: [
            m.controls.get("distort_freq"),
            m.controls.get("signal_mix"),
            m.controls.get("fractal_grid_scale"),
            m.controls.get("fractal_scale"),
        ],
        d: [
            m.controls.get("distort_angle_offset"),
            m.controls.get("signal_steps"),
            m.controls.get("fractal_color_scale"),
            m.controls.get("fractal_grid_mix"),
        ],
        e: [
            m.controls.get("mask_radius"),
            m.controls.get("mask_falloff"),
            m.controls.get("mask_x"),
            m.controls.get("mask_y"),
        ],
    };

    m.gpu.update_params(app, m.wr.resolution_u32(), &params);
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    frame.clear(BLACK);
    m.gpu.render(&frame);
}
