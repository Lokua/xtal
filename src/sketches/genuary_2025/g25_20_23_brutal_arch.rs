use bevy_reflect::Reflect;
use nannou::prelude::*;

use crate::framework::prelude::*;

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
    gui_h: Some(360),
};

const BACKGROUND: f32 = 0.0;
const FOREGROUND: f32 = 1.0;

#[derive(SketchComponents)]
pub struct Model {
    controls: ControlScript<FrameTiming>,
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

    // unused
    b: [f32; 4],
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let controls = ControlScript::new(
        to_absolute_path(file!(), "g25_20_23_brutal_arch.yaml"),
        FrameTiming::new(SKETCH_CONFIG.bpm),
    );

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
    };

    let vertices = create_vertices();

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
            m.controls.get("b2"),
            m.controls.get("b3"),
            m.controls.get("b4"),
        ],
    };

    let vertices = create_vertices();

    m.gpu.update(app, &params, &vertices);
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    frame.clear(BLACK);
    m.gpu.render(&frame);
}

fn create_vertices() -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(42);
    vertices.extend(create_fullscreen_quad());
    vertices.extend(create_cube([-0.5, 0.0, 0.999]));
    vertices.extend(create_cube([0.5, 0.0, 0.999]));
    vertices.extend(create_cube([0.0, -0.5, 0.999]));
    vertices.extend(create_cube([0.0, 0.5, 0.999]));
    vertices
}

const QUAD_POSITIONS: [[f32; 3]; 6] = [
    // Bottom-left
    [-1.0, -1.0, 0.0],
    // Bottom-right
    [1.0, -1.0, 0.0],
    // Top-right
    [1.0, 1.0, 0.0],
    // Bottom-left
    [-1.0, -1.0, 0.0],
    // Top-right
    [1.0, 1.0, 0.0],
    // Top-left
    [-1.0, 1.0, 0.0],
];

const CUBE_POSITIONS: [[f32; 3]; 36] = [
    // Front face
    [-0.5, -0.5, 0.5],
    [0.5, -0.5, 0.5],
    [0.5, 0.5, 0.5],
    [-0.5, -0.5, 0.5],
    [0.5, 0.5, 0.5],
    [-0.5, 0.5, 0.5],
    // Back face
    [-0.5, -0.5, -0.5],
    [-0.5, 0.5, -0.5],
    [0.5, 0.5, -0.5],
    [-0.5, -0.5, -0.5],
    [0.5, 0.5, -0.5],
    [0.5, -0.5, -0.5],
    // Top face
    [-0.5, 0.5, -0.5],
    [-0.5, 0.5, 0.5],
    [0.5, 0.5, 0.5],
    [-0.5, 0.5, -0.5],
    [0.5, 0.5, 0.5],
    [0.5, 0.5, -0.5],
    // Bottom face
    [-0.5, -0.5, -0.5],
    [0.5, -0.5, -0.5],
    [0.5, -0.5, 0.5],
    [-0.5, -0.5, -0.5],
    [0.5, -0.5, 0.5],
    [-0.5, -0.5, 0.5],
    // Right face
    [0.5, -0.5, -0.5],
    [0.5, 0.5, -0.5],
    [0.5, 0.5, 0.5],
    [0.5, -0.5, -0.5],
    [0.5, 0.5, 0.5],
    [0.5, -0.5, 0.5],
    // Left face
    [-0.5, -0.5, -0.5],
    [-0.5, -0.5, 0.5],
    [-0.5, 0.5, 0.5],
    [-0.5, -0.5, -0.5],
    [-0.5, 0.5, 0.5],
    [-0.5, 0.5, -0.5],
];

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
