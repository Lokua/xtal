use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "g25_22_gradients_only",
    display_name: "Genuary 22: Gradients Only",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 1244,
    gui_w: None,
    gui_h: Some(360),
};

#[derive(SketchComponents)]
pub struct Template {
    #[allow(dead_code)]
    animation: Animation<Timing>,
    controls: ControlHub<Timing>,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderParams {
    // w, h, ..unused
    resolution: [f32; 4],

    // t1, t2, t3, t4
    a: [f32; 4],

    // b1, b2, ..unused
    b: [f32; 4],
}

pub fn init(app: &App, ctx: &Context) -> Template {
    let timing = Timing::new(ctx.bpm());
    let animation = Animation::new(timing.clone());

    let controls = ControlHub::from_path(
        to_absolute_path(file!(), "./g25_22_gradients_only.yaml"),
        timing,
    );

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
    };

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        ctx.window_rect().resolution_u32(),
        to_absolute_path(file!(), "./g25_22_gradients_only.wgsl"),
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
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let params = ShaderParams {
            resolution: [
                ctx.window_rect().w(),
                ctx.window_rect().h(),
                0.0,
                0.0,
            ],
            a: [
                self.controls.get("t1"),
                self.controls.get("t2"),
                self.controls.get("t3"),
                self.controls.get("t4"),
            ],
            b: [self.controls.get("b1"), self.controls.get("b2"), 0.0, 0.0],
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
