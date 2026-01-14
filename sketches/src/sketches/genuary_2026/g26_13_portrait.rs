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

#[derive(SketchComponents)]
pub struct SelfPortrait {
    hub: ControlHub<Timing>,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
    image_data: ImageData,
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

    let mut gpu = gpu::GpuState::new_fullscreen(
        app,
        ctx.window_rect().resolution_u32(),
        to_absolute_path(file!(), "g26_13_portrait.wgsl"),
        &params,
        1,
    );

    let texture_view = create_brightness_texture(app, &image_data);
    gpu.set_texture(app, &texture_view);

    SelfPortrait {
        hub,
        gpu,
        image_data,
    }
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

fn create_brightness_texture(
    app: &App,
    image_data: &ImageData,
) -> wgpu::TextureView {
    let window = app.main_window();
    let device = window.device();
    let queue = window.queue();

    let brightness_grid = image_data
        .brightness_grid()
        .expect("Expected grayscale image data");

    let resolution = image_data.resolution;

    let rgba_data: Vec<u8> = brightness_grid
        .iter()
        .flat_map(|row| {
            row.iter().flat_map(|&brightness| {
                let byte_val = (brightness * 255.0).clamp(0.0, 255.0) as u8;
                [byte_val, byte_val, byte_val, 255]
            })
        })
        .collect();

    let texture = wgpu::TextureBuilder::new()
        .size([resolution as u32, resolution as u32])
        .format(wgpu::TextureFormat::Rgba8Unorm)
        .usage(
            wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
        )
        .build(device);

    let texture_size = wgpu::Extent3d {
        width: resolution as u32,
        height: resolution as u32,
        depth_or_array_layers: 1,
    };

    queue.write_texture(
        texture.as_image_copy(),
        &rgba_data,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * resolution as u32),
            rows_per_image: Some(resolution as u32),
        },
        texture_size,
    );

    texture.view().build()
}
