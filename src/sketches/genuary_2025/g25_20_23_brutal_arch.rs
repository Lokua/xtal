use bevy_reflect::Reflect;
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "g25_20_23_brutal_arch",
    display_name:
        "Genuary 20, 23 - Generative Architecture, Inspired by Brutalism",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(360),
};

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
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderParams {
    // w, h, ..unused
    resolution: [f32; 4],

    // rotation, ...unused
    a: [f32; 4],
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let controls = ControlScript::new(
        to_absolute_path(file!(), "g25_20_23_brutal_arch.yaml"),
        FrameTiming::new(SKETCH_CONFIG.bpm),
    );

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
    };

    // 6 faces * 6 vertices each
    let vertices = vec![Vertex { position: [0.0; 3] }; 36];

    let gpu = gpu::GpuState::new(
        app,
        to_absolute_path(file!(), "./g25_20_23_brutal_arch.wgsl"),
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
            m.controls.get("rotation"),
            m.controls.get("z_offset"),
            m.controls.get("a3"),
            m.controls.get("a4"),
        ],
    };

    // 6 faces * 6 vertices per face
    let mut vertices = Vec::with_capacity(36);

    let front_face = vec![
        Vertex {
            position: [-0.5, -0.5, 0.0],
        },
        Vertex {
            position: [0.5, -0.5, 0.0],
        },
        Vertex {
            position: [0.5, 0.5, 0.0],
        },
        Vertex {
            position: [-0.5, -0.5, 0.0],
        },
        Vertex {
            position: [0.5, 0.5, 0.0],
        },
        Vertex {
            position: [-0.5, 0.5, 0.0],
        },
    ];

    let back_face = vec![
        Vertex {
            position: [-0.5, -0.5, -0.5],
        },
        Vertex {
            position: [-0.5, 0.5, -0.5],
        },
        Vertex {
            position: [0.5, 0.5, -0.5],
        },
        Vertex {
            position: [-0.5, -0.5, -0.5],
        },
        Vertex {
            position: [0.5, 0.5, -0.5],
        },
        Vertex {
            position: [0.5, -0.5, -0.5],
        },
    ];

    let top_face = vec![
        Vertex {
            position: [-0.5, 0.5, -0.5],
        },
        Vertex {
            position: [-0.5, 0.5, 0.0],
        },
        Vertex {
            position: [0.5, 0.5, 0.0],
        },
        Vertex {
            position: [-0.5, 0.5, -0.5],
        },
        Vertex {
            position: [0.5, 0.5, 0.0],
        },
        Vertex {
            position: [0.5, 0.5, -0.5],
        },
    ];

    let bottom_face = vec![
        Vertex {
            position: [-0.5, -0.5, -0.5],
        },
        Vertex {
            position: [0.5, -0.5, -0.5],
        },
        Vertex {
            position: [0.5, -0.5, 0.0],
        },
        Vertex {
            position: [-0.5, -0.5, -0.5],
        },
        Vertex {
            position: [0.5, -0.5, 0.0],
        },
        Vertex {
            position: [-0.5, -0.5, 0.0],
        },
    ];

    let right_face = vec![
        Vertex {
            position: [0.5, -0.5, -0.5],
        },
        Vertex {
            position: [0.5, 0.5, -0.5],
        },
        Vertex {
            position: [0.5, 0.5, 0.0],
        },
        Vertex {
            position: [0.5, -0.5, -0.5],
        },
        Vertex {
            position: [0.5, 0.5, 0.0],
        },
        Vertex {
            position: [0.5, -0.5, 0.0],
        },
    ];

    let left_face = vec![
        Vertex {
            position: [-0.5, -0.5, -0.5],
        },
        Vertex {
            position: [-0.5, -0.5, 0.0],
        },
        Vertex {
            position: [-0.5, 0.5, 0.0],
        },
        Vertex {
            position: [-0.5, -0.5, -0.5],
        },
        Vertex {
            position: [-0.5, 0.5, 0.0],
        },
        Vertex {
            position: [-0.5, 0.5, -0.5],
        },
    ];

    vertices.extend(front_face);
    vertices.extend(back_face);
    vertices.extend(top_face);
    vertices.extend(bottom_face);
    vertices.extend(right_face);
    vertices.extend(left_face);

    m.gpu.update(app, &params, &vertices);
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    frame.clear(BLACK);
    m.gpu.render(&frame);
}
