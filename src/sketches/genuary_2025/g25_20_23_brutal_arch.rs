use bevy_reflect::Reflect;
use bytemuck::{Pod, Zeroable};
use nannou::prelude::*;

use crate::framework::{gpu::BasicPositionVertex, prelude::*};

// b/w ~/Live/2025/Lattice - Inspired by Brutalism
// Original non-automated version lives at sketches/brutalism

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
    gui_h: Some(840),
};

const BACKGROUND: f32 = 0.0;
const FOREGROUND: f32 = 1.0;

#[derive(SketchComponents)]
pub struct Template {
    controls: ControlScript<OscTransportTiming>,
    main_shader: gpu::GpuState<Vertex>,
    post_shader: gpu::GpuState<BasicPositionVertex>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Reflect)]
struct Vertex {
    position: [f32; 3],
    center: [f32; 3],
    layer: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct ShaderParams {
    // w, h, ..unused
    resolution: [f32; 4],

    // rot_x, rot_y, rot_z, scale
    a: [f32; 4],

    // scale, texture_strength, texture_scale, echo_time
    b: [f32; 4],

    // echo_threshold, echo_intensity, grid_contrast, grid_size
    c: [f32; 4],

    // grid_border_size, corner_offset, middle_translate, middle_size
    d: [f32; 4],

    // corner_t_1 - corner_t_4
    e: [f32; 4],
    // corner_t_5 - corner_t_8
    f: [f32; 4],

    // stag, diag, bulge, offs
    g: [f32; 4],

    // bg_noise, bg_noise_scale, color_spread, corner_translate
    h: [f32; 4],

    // twist, explode, wave, phase_twist
    i: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct PostShaderParams {
    // w, h, ..unused
    resolution: [f32; 4],

    // edge_mix, edge_size, edge_thresh, geo_mix
    z: [f32; 4],

    // geo_size, geo_offs, contrast, brightness
    y: [f32; 4],
}

pub fn init(app: &App, ctx: &LatticeContext) -> Template {
    let controls = ControlScript::from_path(
        to_absolute_path(file!(), "g25_20_23_brutal_arch.yaml"),
        OscTransportTiming::new(ctx.bpm()),
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

    let post_params = PostShaderParams {
        resolution: [0.0; 4],
        z: [0.0; 4],
        y: [0.0; 4],
    };

    let vertices = create_vertices(0.0);

    let main_shader = gpu::GpuState::new(
        app,
        ctx.window_rect().resolution_u32(),
        to_absolute_path(file!(), "g25_20_23_brutal_arch_shader1.wgsl"),
        &params,
        Some(&vertices),
        wgpu::PrimitiveTopology::TriangleList,
        Some(wgpu::BlendState::ALPHA_BLENDING),
        true,
        true,
    );
    let post_shader = gpu::GpuState::new_fullscreen(
        app,
        ctx.window_rect().resolution_u32(),
        to_absolute_path(file!(), "g25_20_23_brutal_arch_shader2.wgsl"),
        &post_params,
        true,
    );

    Template {
        controls,
        main_shader,
        post_shader,
    }
}

impl Sketch for Template {
    fn update(&mut self, app: &App, _update: Update, ctx: &LatticeContext) {
        self.controls.update();

        // Modulate the modulator
        let corner_offset_meta = self.controls.get("corner_offset_meta");
        let corner_t_meta = self.controls.get("corner_t_meta");
        let middle_size_meta = self.controls.get("middle_size_meta");
        let rot_z_meta = self.controls.get("rot_z_meta");

        let params = ShaderParams {
            resolution: [
                ctx.window_rect().w(),
                ctx.window_rect().h(),
                0.0,
                0.0,
            ],
            a: [
                self.controls.get("rot_x"),
                self.controls.get("rot_y"),
                rot_z_meta * self.controls.get("rot_z"),
                self.controls.get("z_offset"),
            ],
            b: [
                self.controls.get("scale"),
                self.controls.get("texture_strength"),
                self.controls.get("texture_scale"),
                self.controls.get("echo_time"),
            ],
            c: [
                self.controls.get("echo_threshold"),
                self.controls.get("echo_intensity"),
                self.controls.get("grid_contrast"),
                self.controls.get("grid_size"),
            ],
            d: [
                self.controls.get("grid_border_size"),
                corner_offset_meta * self.controls.get("corner_offset"),
                self.controls.get("middle_translate"),
                middle_size_meta * self.controls.get("middle_size"),
            ],
            e: [
                corner_t_meta * self.controls.get("corner_t_1"),
                corner_t_meta * self.controls.get("corner_t_2"),
                corner_t_meta * self.controls.get("corner_t_3"),
                corner_t_meta * self.controls.get("corner_t_4"),
            ],
            f: [
                corner_t_meta * self.controls.get("corner_t_5"),
                corner_t_meta * self.controls.get("corner_t_6"),
                corner_t_meta * self.controls.get("corner_t_7"),
                corner_t_meta * self.controls.get("corner_t_8"),
            ],
            g: [
                self.controls.get("stag"),
                self.controls.get("diag"),
                self.controls.get("bulge"),
                self.controls.get("offs"),
            ],
            h: [
                self.controls.get("bg_noise"),
                self.controls.get("bg_noise_scale"),
                self.controls.get("color_spread"),
                self.controls.get("corner_translate"),
            ],
            i: [
                self.controls.get("twist"),
                self.controls.get("explode"),
                self.controls.get("wave"),
                self.controls.get("phase_twist"),
            ],
        };

        let post_params = PostShaderParams {
            resolution: [
                ctx.window_rect().w(),
                ctx.window_rect().h(),
                0.0,
                0.0,
            ],
            z: [
                self.controls.get("edge_mix"),
                self.controls.get("edge_size"),
                self.controls.get("edge_thresh"),
                self.controls.get("geo_mix"),
            ],
            y: [
                self.controls.get("geo_size"),
                self.controls.get("geo_offs"),
                self.controls.get("contrast"),
                self.controls.get("brightness"),
            ],
        };

        let vertices = create_vertices(self.controls.get("scale"));

        let window_size = ctx.window_rect().resolution_u32();

        self.main_shader
            .update(app, window_size, &params, &vertices);

        let texture = self.main_shader.render_to_texture(app);
        self.post_shader.set_input_texture(app, &texture);
        self.post_shader
            .update_params(app, window_size, &post_params);
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &LatticeContext) {
        frame.clear(WHITE);
        self.post_shader.render(&frame);
    }
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
