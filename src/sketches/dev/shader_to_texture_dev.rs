use crate::framework::prelude::*;
use bevy_reflect::Reflect;
use nannou::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "shader_to_texture_dev",
    display_name: "Shader to Texture Development",
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
    #[allow(dead_code)]
    animation: Animation<Timing>,
    controls: Controls,
    wr: WindowRect,
    first_pass: gpu::GpuState<Vertex>,
    second_pass: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Reflect)]
struct Vertex {
    position: [f32; 3],
    layer: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderParams {
    // w, h, ..unused
    resolution: [f32; 4],
    // mode, radius, ..unused
    a: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PostProcessParams {
    // w, h, ..unused
    resolution: [f32; 4],

    // unused
    a: [f32; 4],
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let animation = Animation::new(Timing::new(SKETCH_CONFIG.bpm));

    let controls = Controls::with_previous(vec![
        Control::slide("a1", 0.0),
        Control::slide("a2", 0.0),
        Control::slide("a3", 0.0),
        Control::slide("a4", 0.0),
    ]);

    let first_pass_params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
    };

    let post_process_params = PostProcessParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
    };

    let vertices = create_vertices();

    let first_pass = gpu::GpuState::new(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "shader_to_texture_dev.wgsl"),
        &first_pass_params,
        Some(&vertices),
        wgpu::PrimitiveTopology::TriangleList,
        Some(wgpu::BlendState::ALPHA_BLENDING),
        true,
        true,
    );

    let second_pass = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "shader_to_texture_dev2.wgsl"),
        &post_process_params,
        true,
    );

    Model {
        animation,
        controls,
        wr,
        first_pass,
        second_pass,
    }
}

pub fn update(app: &App, m: &mut Model, _update: Update) {
    let first_pass_params = ShaderParams {
        resolution: [m.wr.w(), m.wr.h(), 0.0, 0.0],
        a: [
            m.controls.float("a1"),
            m.controls.float("a2"),
            m.controls.float("a3"),
            m.controls.float("a4"),
        ],
    };

    let post_process_params = PostProcessParams {
        resolution: [m.wr.w(), m.wr.h(), 0.0, 0.0],
        a: [
            m.controls.float("a1"),
            m.controls.float("a2"),
            m.controls.float("a3"),
            m.controls.float("a4"),
        ],
    };

    let texture_view = m.first_pass.render_to_texture(app);
    m.second_pass.set_input_texture(app, &texture_view);

    let vertices = create_vertices();
    m.first_pass.update(
        app,
        m.wr.resolution_u32(),
        &first_pass_params,
        &vertices,
    );
    m.second_pass.update_params(
        app,
        m.wr.resolution_u32(),
        &post_process_params,
    );
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    frame.clear(BLACK);
    // m.first_pass.render(&frame);
    m.second_pass.render(&frame);
}

fn create_vertices() -> Vec<Vertex> {
    let mut vertices = vec![];
    vertices.extend(create_fullscreen_quad());
    vertices.extend(create_cube());
    vertices
}

fn create_fullscreen_quad() -> Vec<Vertex> {
    QUAD_POSITIONS
        .iter()
        .map(|&position| Vertex {
            position,
            layer: BACKGROUND,
        })
        .collect()
}

fn create_cube() -> Vec<Vertex> {
    CUBE_POSITIONS
        .iter()
        .map(|&position| Vertex {
            position,
            layer: FOREGROUND,
        })
        .collect()
}
