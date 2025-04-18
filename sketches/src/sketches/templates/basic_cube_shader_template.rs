use bevy_reflect::Reflect;
use xtal::prelude::*;
use nannou::prelude::*;

use crate::util::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "basic_cube_shader_template",
    display_name: "Template | Basic Cube Shader",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
};

const BACKGROUND: f32 = 0.0;
const FOREGROUND: f32 = 1.0;

#[derive(SketchComponents)]
pub struct BasicCubeShader {
    controls: ControlHub<Timing>,
    gpu: gpu::GpuState<Vertex>,
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

    // rotation, ...unused
    a: [f32; 4],
}

pub fn init(app: &App, ctx: &Context) -> BasicCubeShader {
    let controls = ControlHub::from_path(
        to_absolute_path(file!(), "basic_cube_shader_template.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
    };

    let vertices = create_vertices();

    let gpu = gpu::GpuState::new(
        app,
        ctx.window_rect().resolution_u32(),
        to_absolute_path(file!(), "basic_cube_shader_template.wgsl"),
        &params,
        Some(&vertices),
        wgpu::PrimitiveTopology::TriangleList,
        Some(wgpu::BlendState::ALPHA_BLENDING),
        true,
        true,
    );

    BasicCubeShader { controls, gpu }
}

impl Sketch for BasicCubeShader {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();

        let params = ShaderParams {
            resolution: [wr.w(), wr.h(), 0.0, 0.0],
            a: [
                self.controls.get("rotation"),
                self.controls.get("z_offset"),
                self.controls.get("scale"),
                self.controls.get("a4"),
            ],
        };

        let vertices = create_vertices();

        self.gpu
            .update(app, wr.resolution_u32(), &params, &vertices);
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &Context) {
        frame.clear(BLACK);
        self.gpu.render(&frame);
    }
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
