use nannou::prelude::*;
use xtal::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "g26_13_portrait",
    display_name: "Genuary 2026 | 13 - Self Portrait",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
};

#[repr(C)]
#[derive(
    Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, bevy_reflect::Reflect,
)]
struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
    brightness: f32,
}

#[derive(SketchComponents)]
pub struct SelfPortrait {
    hub: ControlHub<Timing>,
    gpu: gpu::GpuState<Vertex>,
}

#[uniforms(banks = 8)]
struct ShaderParams {}

pub fn init(app: &App, ctx: &Context) -> SelfPortrait {
    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "g26_13_portrait.yaml"),
        Timing::new(ctx.bpm()),
    );

    let image_data = ImageData::from_json_file(to_absolute_path(
        file!(),
        "g26_13_portrait.json",
    ))
    .expect("Failed to load portrait image data");

    let params = ShaderParams::default();
    let vertices = create_grid_vertices(&image_data);

    let gpu = gpu::GpuState::new(
        app,
        ctx.window_rect().resolution_u32(),
        to_absolute_path(file!(), "g26_13_portrait.wgsl"),
        &params,
        Some(&vertices),
        wgpu::PrimitiveTopology::TriangleList,
        Some(wgpu::BlendState::ALPHA_BLENDING),
        true,
        0,
        true,
    );

    SelfPortrait { hub, gpu }
}

impl Sketch for SelfPortrait {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();
        let params = ShaderParams::from((&wr, &self.hub));
        self.gpu.update_params(app, wr.resolution_u32(), &params);
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &Context) {
        frame.clear(BLACK);
        self.gpu.render(&frame);
    }
}

fn create_grid_vertices(image_data: &ImageData) -> Vec<Vertex> {
    let brightness_grid = image_data
        .brightness_grid()
        .expect("Expected grayscale image data");

    let resolution = image_data.resolution;
    let mut vertices = Vec::new();

    let cell_size = 2.0 / resolution as f32;

    (0..resolution).for_each(|y| {
        for x in 0..resolution {
            let brightness = brightness_grid[y][x];

            let x0 = -1.0 + (x as f32) * cell_size;
            let y0 = 1.0 - (y as f32) * cell_size;
            let x1 = x0 + cell_size;
            let y1 = y0 - cell_size;

            let u0 = x as f32 / resolution as f32;
            let v0 = y as f32 / resolution as f32;
            let u1 = (x + 1) as f32 / resolution as f32;
            let v1 = (y + 1) as f32 / resolution as f32;

            vertices.extend_from_slice(&[
                Vertex {
                    position: [x0, y0, 0.0],
                    uv: [u0, v0],
                    brightness,
                },
                Vertex {
                    position: [x1, y0, 0.0],
                    uv: [u1, v0],
                    brightness,
                },
                Vertex {
                    position: [x1, y1, 0.0],
                    uv: [u1, v1],
                    brightness,
                },
                Vertex {
                    position: [x0, y0, 0.0],
                    uv: [u0, v0],
                    brightness,
                },
                Vertex {
                    position: [x1, y1, 0.0],
                    uv: [u1, v1],
                    brightness,
                },
                Vertex {
                    position: [x0, y1, 0.0],
                    uv: [u0, v1],
                    brightness,
                },
            ]);
        }
    });

    vertices
}
