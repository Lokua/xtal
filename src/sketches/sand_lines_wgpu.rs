use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "sand_lines_wgpu",
    display_name: "Sand Lines WGPU",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(400),
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderParams {
    // w, h, ..unused
    resolution: [f32; 4],

    // start_x, start_y, end_x, end_y
    ref_points: [f32; 4],

    // points_per_segment, noise_scale, angle_variation, n_lines
    settings: [f32; 4],

    // point_size, ...unused
    settings2: [f32; 4],

    // straight_weight, circle_weight, sine_weight, unused
    effect_weights: [f32; 4],

    // circle_r_min, circle_r_max, sine_amp, sine_freq
    effect_params: [f32; 4],
}

#[derive(SketchComponents)]
pub struct Model {
    controls: Controls,
    wr: WindowRect,
    gpu: gpu::GpuState,
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let controls = Controls::with_previous(vec![
        Control::slider("v_count_millions", 6.0, (1.0, 100.0), 1.0),
        Control::slider("n_lines", 64.0, (1.0, 128.0), 1.0),
        Control::slider("points_per_segment", 100.0, (10.0, 10_000.0), 10.0),
        Control::slider("noise_scale", 0.001, (0.0, 0.1), 0.0001),
        Control::slider("angle_variation", 0.2, (0.0, TWO_PI), 0.1),
        Control::slider("point_size", 0.001, (0.0005, 0.01), 0.0001),
        Control::Separator {},
        Control::slider_norm("straight_weight", 1.0),
        Control::slider_norm("circle_weight", 0.0),
        Control::slider_norm("circle_r_min", 0.5),
        Control::slider_norm("circle_r_max", 0.9),
        Control::slider_norm("sine_weight", 0.0),
        Control::slider("sine_freq", 1.0, (1.0, 100.0), 1.0),
        Control::slider_norm("sine_amp", 0.5),
    ]);

    let params = ShaderParams {
        resolution: [0.0; 4],
        ref_points: [0.0; 4],
        settings: [0.0; 4],
        settings2: [0.0; 4],
        effect_weights: [0.0; 4],
        effect_params: [0.0; 4],
    };

    let shader = wgpu::include_wgsl!("./sand_lines_wgpu.wgsl");
    let gpu = gpu::GpuState::new_with_config(
        app,
        shader,
        &params,
        gpu::PipelineConfig {
            vertex_data: None,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            ..Default::default()
        },
    );

    Model { controls, wr, gpu }
}

pub fn update(app: &App, m: &mut Model, _update: Update) {
    if m.controls.changed() {
        let points_per_segment = m.controls.float("points_per_segment") as u32;

        let params = ShaderParams {
            resolution: [m.wr.w(), m.wr.h(), 0.0, 0.0],
            ref_points: [-0.9, 0.0, 0.9, 0.0],
            settings: [
                points_per_segment as f32,
                m.controls.float("noise_scale"),
                m.controls.float("angle_variation"),
                m.controls.float("n_lines"),
            ],
            settings2: [m.controls.float("point_size"), 0.0, 0.0, 0.0],
            effect_weights: [
                m.controls.float("straight_weight"),
                m.controls.float("circle_weight"),
                m.controls.float("sine_weight"),
                0.0,
            ],
            effect_params: [
                m.controls.float("circle_r_min"),
                m.controls.float("circle_r_max"),
                m.controls.float("sine_amp"),
                m.controls.float("sine_freq"),
            ],
        };

        m.gpu.update_params(app, &params);
        m.controls.mark_unchanged();
    }
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    frame.clear(WHITE);

    // Calculate base geometry
    let points_per_line = m.controls.float("points_per_segment") as u32;
    let n_lines = m.controls.float("n_lines") as u32;
    let total_points = points_per_line * n_lines;

    // Multiply by density factor from v_count_millions
    let density = m.controls.float("v_count_millions") as u32;
    let total_vertices = total_points * 6 * density;

    m.gpu.render_procedural(&frame, total_vertices);
}
