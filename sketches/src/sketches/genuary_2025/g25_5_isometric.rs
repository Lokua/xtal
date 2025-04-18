use gpu::GpuState;
use nannou::prelude::*;

use xtal::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "g25_5_isometric",
    display_name: "Genuary 5: Isometric Art",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
};

#[derive(SketchComponents)]
pub struct Template {
    controls: ControlHub<Timing>,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderParams {
    // w, h, ..unused
    resolution: [f32; 4],
    a: [f32; 4],
}

pub fn init(app: &App, ctx: &Context) -> Template {
    let controls = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .slider("a1", 0.5, (0.0, 1.0), 0.01, None)
        .slider("a2", 0.5, (0.0, 1.0), 0.01, None)
        .slider("a3", 0.5, (0.0, 1.0), 0.01, None)
        .slider("a4", 0.5, (0.0, 1.0), 0.01, None)
        .build();

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
    };

    let window_size = ctx.window_rect().resolution_u32();

    let gpu = GpuState::new_fullscreen(
        app,
        window_size,
        to_absolute_path(file!(), "./g25_5_isometric.wgsl"),
        &params,
        true,
    );

    Template { controls, gpu }
}

impl Sketch for Template {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();

        let params = ShaderParams {
            resolution: [wr.w(), wr.h(), 0.0, 0.0],
            a: [
                self.controls.get("a1"),
                self.controls.get("a2"),
                self.controls.get("a3"),
                self.controls.get("a4"),
            ],
        };

        self.gpu.update_params(
            app,
            ctx.window_rect().resolution_u32(),
            &params,
        );
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &Context) {
        frame.clear(BLACK);
        self.gpu.render(&frame);
    }
}
