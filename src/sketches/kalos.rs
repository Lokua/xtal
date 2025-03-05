use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "kalos",
    display_name: "Kalos",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(600),
};

#[derive(SketchComponents)]
pub struct Kalos {
    animation: Animation<Timing>,
    controls: Controls,
    #[allow(dead_code)]
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderParams {
    resolution: [f32; 4],

    show_center: f32,
    show_corners: f32,
    radius: f32,
    strength: f32,

    corner_radius: f32,
    corner_strength: f32,
    scaling_power: f32,
    auto_hue_shift: f32,

    r: f32,
    g: f32,
    b: f32,
    offset: f32,

    ring_strength: f32,
    angular_variation: f32,
    threshold: f32,
    mix: f32,

    alg: f32,
    j: f32,
    k: f32,
    time: f32,
}

pub fn init(app: &App, ctx: LatticeContext) -> Kalos {
    let wr = ctx.window_rect();
    let animation = Animation::new(Timing::new(ctx.bpm()));

    let controls = Controls::with_previous(vec![
        Control::checkbox("animate", false),
        Control::checkbox("show_center", false),
        Control::checkbox("show_corners", false),
        Control::slider("radius", 0.5, (0.0, 10.0), 0.01),
        Control::slider("strength", 0.5, (0.0, 5.0), 0.001),
        Control::slider("corner_radius", 0.5, (0.0, 10.0), 0.01),
        Control::slider("corner_strength", 0.5, (0.0, 5.0), 0.001),
        Control::Separator {},
        Control::slider("scaling_power", 1.0, (0.01, 20.0), 0.01),
        Control::slide("offset", 0.2),
        Control::slider("ring_strength", 20.0, (1.0, 100.0), 0.01),
        Control::slider("angular_variation", 4.0, (1.0, 45.0), 1.0),
        Control::Separator {},
        Control::select(
            "alg",
            "distance",
            &["distance", "concentric_waves", "moire"],
        ),
        Control::slider_x("j", 0.5, (0.0, 1.0), 0.0001, |controls| {
            controls.string("alg") == "distance"
        }),
        Control::slider_x("k", 0.5, (0.0, 1.0), 0.0001, |controls| {
            controls.string("alg") != "moire"
        }),
        Control::Separator {},
        Control::checkbox("auto_hue_shift", false),
        Control::slide("r", 0.5),
        Control::slide("g", 0.0),
        Control::slide("b", 1.0),
        Control::Separator {},
        Control::slide("threshold", 0.5),
        Control::slide("mix", 0.5),
    ]);

    let params = ShaderParams {
        resolution: [0.0; 4],
        show_center: 1.0,
        show_corners: 1.0,
        radius: 0.0,
        strength: 0.0,
        corner_radius: 0.0,
        corner_strength: 0.0,
        scaling_power: 0.0,
        auto_hue_shift: 0.0,
        r: 0.0,
        g: 0.0,
        b: 0.0,
        offset: 0.0,
        ring_strength: 0.0,
        angular_variation: 0.0,
        threshold: 0.0,
        mix: 0.0,
        alg: 0.0,
        j: 0.0,
        k: 0.0,
        time: app.time,
    };

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "kalos.wgsl"),
        &params,
        true,
    );

    Kalos {
        animation,
        controls,
        gpu,
    }
}

impl Sketch for Kalos {
    fn update(&mut self, app: &App, _update: Update, ctx: &LatticeContext) {
        let wr = ctx.window_rect();
        let strength = self.controls.float("strength");
        let strength_range = self.controls.slider_range("strength");
        let strength_swing = 0.05;

        let params = ShaderParams {
            resolution: [wr.w(), wr.h(), 0.0, 0.0],
            show_center: self.controls.bool("show_center") as i32 as f32,
            show_corners: self.controls.bool("show_corners") as i32 as f32,
            radius: self.controls.float("radius"),
            strength: if self.controls.bool("animate") {
                self.animation.lrp(
                    &[
                        kf(
                            clamp(
                                strength - strength_swing,
                                strength_range.0,
                                strength_range.1,
                            ),
                            1.0,
                        ),
                        kf(
                            clamp(
                                strength + strength_swing,
                                strength_range.0,
                                strength_range.1,
                            ),
                            1.0,
                        ),
                    ],
                    0.0,
                )
            } else {
                strength
            },
            corner_radius: self.controls.float("corner_radius"),
            corner_strength: self.controls.float("corner_strength"),
            scaling_power: self.controls.float("scaling_power"),
            auto_hue_shift: self.controls.bool("auto_hue_shift") as i32 as f32,
            r: self.controls.float("r"),
            g: self.controls.float("g"),
            b: self.controls.float("b"),
            offset: self.controls.float("offset"),
            ring_strength: self.controls.float("ring_strength"),
            angular_variation: self.controls.float("angular_variation"),
            threshold: self.controls.float("threshold"),
            mix: self.controls.float("mix"),
            alg: match self.controls.string("alg").as_str() {
                "distance" => 0.0,
                "concentric_waves" => 1.0,
                "moire" => 2.0,
                _ => unreachable!(),
            },
            j: self.controls.float("j"),
            k: self.controls.float("k"),
            time: app.time,
        };

        self.gpu.update_params(app, wr.resolution_u32(), &params);
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &LatticeContext) {
        self.gpu.render(&frame);
    }
}
