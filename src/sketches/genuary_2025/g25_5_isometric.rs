use gpu::GpuState;
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "g25_5_isometric",
    display_name: "Genuary 5: Isometric Art",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(360),
};

#[derive(SketchComponents)]
pub struct Template {
    #[allow(dead_code)]
    animation: Animation<Timing>,
    controls: Controls,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderParams {
    // w, h, ..unused
    resolution: [f32; 4],
    a: [f32; 4],
}

pub fn init(app: &App, ctx: &LatticeContext) -> Template {
    let animation = Animation::new(Timing::new(ctx.bpm()));

    let controls = Controls::new(vec![
        Control::slider("a1", 0.5, (0.0, 1.0), 0.01),
        Control::slider("a2", 0.5, (0.0, 1.0), 0.01),
        Control::slider("a3", 0.5, (0.0, 1.0), 0.01),
        Control::slider("a4", 0.5, (0.0, 1.0), 0.01),
    ]);

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

    Template {
        animation,
        controls,
        gpu,
    }
}

impl Sketch for Template {
    fn update(&mut self, app: &App, _update: Update, ctx: &LatticeContext) {
        let params = ShaderParams {
            resolution: [
                ctx.window_rect().w(),
                ctx.window_rect().h(),
                0.0,
                0.0,
            ],
            a: [
                self.controls.float("a1"),
                self.controls.float("a2"),
                self.controls.float("a3"),
                self.controls.float("a4"),
            ],
        };

        self.gpu.update_params(
            app,
            ctx.window_rect().resolution_u32(),
            &params,
        );
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &LatticeContext) {
        frame.clear(BLACK);
        self.gpu.render(&frame);
    }
}
