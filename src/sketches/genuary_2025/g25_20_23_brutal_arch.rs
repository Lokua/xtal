use bevy_reflect::Reflect;
use nannou::prelude::*;

use crate::framework::prelude::*;

// b/w ~/Live/2025/Lattice - Inspired by Brutalism

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "g25_20_23_brutal_arch",
    display_name:
        "Genuary 20, 23 | Generative Architecture, Inspired by Brutalism",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(620),
};

const BACKGROUND: f32 = 0.0;
const FOREGROUND: f32 = 1.0;

#[derive(SketchComponents)]
pub struct Model {
    controls: ControlScript<Timing>,
    wr: WindowRect,
    gpu: gpu::GpuState<Vertex>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Reflect)]
struct Vertex {
    position: [f32; 3],
    center: [f32; 3],
    layer: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderParams {
    // w, h, ..unused
    resolution: [f32; 4],

    // rot_x, rot_y, rot_z, scale
    a: [f32; 4],

    // scale, texture_strength, texture_scale, echo_time
    b: [f32; 4],

    // echo_threshold, echo_intensity, grid_contrast, grid_size
    c: [f32; 4],

    // grid_border_size, corner_offset, middle_offset, middle_size
    d: [f32; 4],

    // corner_t_1 - corner_t_4
    e: [f32; 4],
    // corner_t_5 - corner_t_8
    f: [f32; 4],

    // stag, diag, bulge, offs
    g: [f32; 4],

    // bg_noise, bg_noise_scale, color_spread, unused
    h: [f32; 4],

    // twist, explode, wave, phase_twist
    i: [f32; 4],
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let controls = ControlScript::new(
        to_absolute_path(file!(), "g25_20_23_brutal_arch.yaml"),
        Timing::new(SKETCH_CONFIG.bpm),
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
        i: [0.0; 4],
    };

    let vertices = create_vertices(0.0);

    let gpu = gpu::GpuState::new(
        app,
        to_absolute_path(file!(), "g25_20_23_brutal_arch.wgsl"),
        &params,
        Some(&vertices),
        wgpu::PrimitiveTopology::TriangleList,
        Some(wgpu::BlendState::ALPHA_BLENDING),
        true,
        true,
    );

    Model { controls, wr, gpu }
}

pub fn update(app: &App, m: &mut Model, _update: Update) {
    m.controls.update();

    let params = ShaderParams {
        resolution: [m.wr.w(), m.wr.h(), 0.0, 0.0],
        a: [
            m.controls.get("rot_x"),
            m.controls.get("rot_y"),
            m.controls.get("rot_z"),
            m.controls.get("z_offset"),
        ],
        b: [
            m.controls.get("scale"),
            m.controls.get("texture_strength"),
            m.controls.get("texture_scale"),
            m.controls.get("echo_time"),
        ],
        c: [
            m.controls.get("echo_threshold"),
            m.controls.get("echo_intensity"),
            m.controls.get("grid_contrast"),
            m.controls.get("grid_size"),
        ],
        d: [
            m.controls.get("grid_border_size"),
            m.controls.get("corner_offset"),
            m.controls.get("d3"),
            m.controls.get("middle_size"),
        ],
        e: [
            (m.controls.get("corner_t_1")),
            (m.controls.get("corner_t_2")),
            (m.controls.get("corner_t_3")),
            (m.controls.get("corner_t_4")),
        ],
        f: [
            m.controls.get("corner_t_5"),
            m.controls.get("corner_t_6"),
            m.controls.get("corner_t_7"),
            m.controls.get("corner_t_8"),
        ],
        g: [
            m.controls.get("stag"),
            m.controls.get("diag"),
            m.controls.get("bulge"),
            m.controls.get("offs"),
        ],
        h: [
            m.controls.get("bg_noise"),
            m.controls.get("bg_noise_scale"),
            m.controls.get("color_spread"),
            m.controls.get("h4"),
        ],
        i: [
            m.controls.get("twist"),
            m.controls.get("explode"),
            m.controls.get("wave"),
            m.controls.get("phase_twist"),
        ],
    };

    let vertices = create_vertices(m.controls.get("scale"));

    m.gpu.update(app, &params, &vertices);
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    frame.clear(BLACK);
    m.gpu.render(&frame);
}

fn create_vertices(scale: f32) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    vertices.extend(create_fullscreen_quad());

    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                if x.abs() + y.abs() + z.abs() > 1 {
                    vertices.extend(create_cube([
                        x as f32 * scale,
                        y as f32 * scale,
                        z as f32 * scale,
                    ]));
                }
            }
        }
    }

    vertices
}

fn create_fullscreen_quad() -> Vec<Vertex> {
    QUAD_POSITIONS
        .iter()
        .map(|&position| Vertex {
            position,
            center: [0.0, 0.0, 0.999],
            layer: BACKGROUND,
        })
        .collect()
}

fn create_cube(center: [f32; 3]) -> Vec<Vertex> {
    CUBE_POSITIONS
        .iter()
        .map(|&position| Vertex {
            position,
            center,
            layer: FOREGROUND,
        })
        .collect()
}
