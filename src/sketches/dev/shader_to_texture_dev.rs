use crate::framework::prelude::*;
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

#[derive(SketchComponents)]
pub struct Model {
    #[allow(dead_code)]
    animation: Animation<Timing>,
    controls: Controls,
    wr: WindowRect,
    first_pass_gpu: gpu::GpuState<gpu::BasicPositionVertex>,
    post_process_gpu: gpu::GpuState<gpu::BasicPositionVertex>,
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

    // time, distort_amount, ..unused
    a: [f32; 4],
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let animation = Animation::new(Timing::new(SKETCH_CONFIG.bpm));

    let controls = Controls::with_previous(vec![
        Control::select("mode", "smooth", &["smooth", "step"]),
        Control::slider_norm("radius", 0.5),
        Control::slider_norm("distort_amount", 0.5),
    ]);

    let first_pass_params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
    };

    let post_process_params = PostProcessParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
    };

    let first_pass_gpu = gpu::GpuState::new_full_screen(
        app,
        to_absolute_path(file!(), "shader_to_texture_dev.wgsl"),
        &first_pass_params,
        true,
    );

    let post_process_gpu = gpu::GpuState::new_full_screen(
        app,
        to_absolute_path(file!(), "shader_to_texture_dev2.wgsl"),
        &post_process_params,
        true,
    );

    Model {
        animation,
        controls,
        wr,
        first_pass_gpu,
        post_process_gpu,
    }
}

pub fn update(app: &App, m: &mut Model, _update: Update) {
    let first_pass_params = ShaderParams {
        resolution: [m.wr.w(), m.wr.h(), 0.0, 0.0],
        a: [
            match m.controls.string("mode").as_str() {
                "smooth" => 0.0,
                "step" => 1.0,
                _ => unreachable!(),
            },
            m.controls.float("radius"),
            0.0,
            0.0,
        ],
    };

    let post_process_params = PostProcessParams {
        resolution: [m.wr.w(), m.wr.h(), 0.0, 0.0],
        a: [app.time, m.controls.float("distort_amount"), 0.0, 0.0],
    };

    let texture_view = m.first_pass_gpu.render_to_texture(app);
    m.post_process_gpu.set_input_texture(app, &texture_view);

    m.first_pass_gpu.update_params(app, &first_pass_params);
    m.post_process_gpu.update_params(app, &post_process_params);
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    frame.clear(BLACK);
    m.post_process_gpu.render(&frame);
}
