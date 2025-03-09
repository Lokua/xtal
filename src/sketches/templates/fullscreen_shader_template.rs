use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "fullscreen_shader_template",
    display_name: "Template | Fullscreen Quad",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(360),
};

#[derive(SketchComponents)]
pub struct FullscreenShader {
    controls: ControlScript<Timing>,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderParams {
    // w, h, ..unused
    resolution: [f32; 4],

    // mode, radius, ..unused
    a: [f32; 4],
}

pub fn init(app: &App, ctx: &LatticeContext) -> FullscreenShader {
    let controls = ControlScriptBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .select("mode", "smooth", &["smooth", "step"], None)
        .slider_n("radius", 0.5)
        .build();

    let wr = ctx.window_rect();

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
    };

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "fullscreen_shader_template.wgsl"),
        &params,
        true,
    );

    FullscreenShader { controls, gpu }
}

impl Sketch for FullscreenShader {
    fn update(&mut self, app: &App, _update: Update, ctx: &LatticeContext) {
        let wr = ctx.window_rect();

        let params = ShaderParams {
            resolution: [wr.w(), wr.h(), 0.0, 0.0],
            a: [
                match self.controls.string("mode").as_str() {
                    "smooth" => 0.0,
                    "step" => 1.0,
                    _ => unreachable!(),
                },
                self.controls.get("radius"),
                0.0,
                0.0,
            ],
        };

        self.gpu.update_params(app, wr.resolution_u32(), &params);
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &LatticeContext) {
        frame.clear(BLACK);
        self.gpu.render(&frame);
    }
}
