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
};

const BACKGROUND: f32 = 0.0;
const FOREGROUND: f32 = 1.0;

#[derive(SketchComponents)]
pub struct ShaderToTextureDev {
    controls: ControlHub<Timing>,
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
    resolution: [f32; 4],
    a: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PostProcessParams {
    resolution: [f32; 4],
    a: [f32; 4],
}

pub fn init(app: &App, ctx: &Context) -> ShaderToTextureDev {
    let controls = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .slider_n("a1", 0.0)
        .slider_n("a2", 0.0)
        .slider_n("a3", 0.0)
        .slider_n("a4", 0.0)
        .build();

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
        ctx.window_rect().resolution_u32(),
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
        ctx.window_rect().resolution_u32(),
        to_absolute_path(file!(), "shader_to_texture_dev2.wgsl"),
        &post_process_params,
        true,
    );

    ShaderToTextureDev {
        controls,
        first_pass,
        second_pass,
    }
}

impl Sketch for ShaderToTextureDev {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();

        let first_pass_params = ShaderParams {
            resolution: [wr.w(), wr.h(), 0.0, 0.0],
            a: [
                self.controls.get("a1"),
                self.controls.get("a2"),
                self.controls.get("a3"),
                self.controls.get("a4"),
            ],
        };

        let post_process_params = PostProcessParams {
            resolution: [wr.w(), wr.h(), 0.0, 0.0],
            a: [
                self.controls.get("a1"),
                self.controls.get("a2"),
                self.controls.get("a3"),
                self.controls.get("a4"),
            ],
        };

        let texture_view = self.first_pass.render_to_texture(app);
        self.second_pass.set_input_texture(app, &texture_view);

        let vertices = create_vertices();
        self.first_pass.update(
            app,
            ctx.window_rect().resolution_u32(),
            &first_pass_params,
            &vertices,
        );
        self.second_pass.update_params(
            app,
            ctx.window_rect().resolution_u32(),
            &post_process_params,
        );
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &Context) {
        frame.clear(BLACK);
        self.second_pass.render(&frame);
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
