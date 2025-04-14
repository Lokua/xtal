use nannou::prelude::*;

use crate::framework::prelude::*;

// b/w 2025 DD Running Free

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "g25_2_layers",
    display_name: "Genuary 2: Layers Upon Layers",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(360),
};

#[derive(SketchComponents)]
pub struct G25_2Layers {
    controls: ControlHub<Timing>,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderParams {
    resolution: [f32; 4],
    a: [f32; 4], // contrast, smooth_mix, time, time_2
    b: [f32; 4], // t1, t2, t3, post_mix
    c: [f32; 4], // r1, r2, r3, unused
    d: [f32; 4], // g1, g2, g3, unused
}

pub fn init(app: &App, ctx: &Ctx) -> G25_2Layers {
    fn create_disabled_fn() -> DisabledFn {
        Some(Box::new(|_controls| true))
    }

    let controls = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .slider_n("smooth_mix", 0.5)
        .slider("contrast", 1.5, (0.1, 5.0), 0.1, None)
        .separator()
        .slider("g1", 2.0, (2.0, 32.0), 1.0, create_disabled_fn())
        .slider("g2", 4.0, (2.0, 32.0), 1.0, create_disabled_fn())
        .slider("g3", 8.0, (2.0, 32.0), 1.0, create_disabled_fn())
        .separator()
        .slider_n("post_mix", 0.5)
        .build();

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
        c: [0.0; 4],
        d: [0.0; 4],
    };

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        ctx.window_rect().resolution_u32(),
        to_absolute_path(file!(), "./g25_2_layers.wgsl"),
        &params,
        true,
    );

    G25_2Layers { controls, gpu }
}

impl Sketch for G25_2Layers {
    fn update(&mut self, app: &App, _update: Update, ctx: &Ctx) {
        let wr = ctx.window_rect();
        let time = 8.0;
        let slew = 0.8;

        let params = ShaderParams {
            resolution: [wr.w(), wr.h(), 0.0, 0.0],
            a: [
                self.controls.float("contrast"),
                self.controls.float("smooth_mix"),
                self.controls.animation.tri(8.0),
                self.controls.animation.tri(12.0),
            ],
            b: [
                self.controls.animation.random_slewed(
                    time,
                    (0.0, 1.0),
                    slew,
                    0.0,
                    887353,
                ),
                self.controls.animation.random_slewed(
                    time,
                    (0.0, 1.0),
                    slew,
                    0.5,
                    9886262,
                ),
                self.controls.animation.random_slewed(
                    time,
                    (0.0, 1.0),
                    slew,
                    0.1,
                    3907823,
                ),
                self.controls.float("post_mix"),
            ],
            c: [
                self.controls.animation.random_slewed(
                    time,
                    (0.0, 1.0),
                    slew,
                    0.0,
                    2837461,
                ),
                self.controls.animation.random_slewed(
                    time,
                    (0.0, 1.0),
                    slew,
                    0.5,
                    12383,
                ),
                self.controls.animation.random_slewed(
                    time,
                    (0.0, 1.0),
                    slew,
                    0.1,
                    12344,
                ),
                0.0,
            ],
            d: [
                2.0,
                self.controls.animation.automate(
                    &[
                        Breakpoint::step(0.0, 2.0),
                        Breakpoint::ramp(4.0, 2.0, Easing::Linear),
                        Breakpoint::step(28.0, 3.0),
                        Breakpoint::ramp(34.0, 3.0, Easing::Linear),
                        Breakpoint::end(38.0, 2.0),
                    ],
                    Mode::Loop,
                ),
                self.controls.animation.automate(
                    &[
                        Breakpoint::step(0.0, 2.0),
                        Breakpoint::ramp(4.0, 2.0, Easing::Linear),
                        Breakpoint::step(8.0, 4.0),
                        Breakpoint::ramp(16.0, 4.0, Easing::Linear),
                        Breakpoint::step(24.0, 8.0),
                        Breakpoint::ramp(32.0, 6.0, Easing::Linear),
                        Breakpoint::end(38.0, 2.0),
                    ],
                    Mode::Loop,
                ),
                0.0,
            ],
        };

        self.gpu.update_params(
            app,
            ctx.window_rect().resolution_u32(),
            &params,
        );
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &Ctx) {
        frame.clear(BLACK);
        self.gpu.render(&frame);
    }
}
