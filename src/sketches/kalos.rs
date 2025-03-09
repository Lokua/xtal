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
    controls: ControlHub<Timing>,
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

pub fn init(app: &App, ctx: &LatticeContext) -> Kalos {
    let wr = ctx.window_rect();

    let controls = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .checkbox("animate", false, None)
        .checkbox("show_center", false, None)
        .checkbox("show_corners", false, None)
        .slider("radius", 0.5, (0.0, 10.0), 0.01, None)
        .slider("strength", 0.5, (0.0, 5.0), 0.001, None)
        .slider("corner_radius", 0.5, (0.0, 10.0), 0.01, None)
        .slider("corner_strength", 0.5, (0.0, 5.0), 0.001, None)
        .separator()
        .slider("scaling_power", 1.0, (0.01, 20.0), 0.01, None)
        .slider_n("offset", 0.2)
        .slider("ring_strength", 20.0, (1.0, 100.0), 0.01, None)
        .slider("angular_variation", 4.0, (1.0, 45.0), 1.0, None)
        .separator()
        .select(
            "alg",
            "distance",
            &["distance", "concentric_waves", "moire"],
            None,
        )
        .slider(
            "j",
            0.5,
            (0.0, 1.0),
            0.0001,
            Some(Box::new(|controls| controls.string("alg") == "distance")),
        )
        .slider(
            "k",
            0.5,
            (0.0, 1.0),
            0.0001,
            Some(Box::new(|controls| controls.string("alg") != "moire")),
        )
        .separator()
        .checkbox("auto_hue_shift", false, None)
        .slider_n("r", 0.5)
        .slider_n("g", 0.0)
        .slider_n("b", 1.0)
        .separator()
        .slider_n("threshold", 0.5)
        .slider_n("mix", 0.5)
        .build();

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

    Kalos { controls, gpu }
}

impl Sketch for Kalos {
    fn update(&mut self, app: &App, _update: Update, ctx: &LatticeContext) {
        self.controls.update();
        let wr = ctx.window_rect();
        let strength = self.controls.get("strength");
        let strength_range = self.controls.ui_controls.slider_range("strength");
        let strength_swing = 0.05;

        let params = ShaderParams {
            resolution: [wr.w(), wr.h(), 0.0, 0.0],
            show_center: self.controls.bool("show_center") as i32 as f32,
            show_corners: self.controls.bool("show_corners") as i32 as f32,
            radius: self.controls.get("radius"),
            strength: if self.controls.bool("animate") {
                self.controls.animation.lrp(
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
            corner_radius: self.controls.get("corner_radius"),
            corner_strength: self.controls.get("corner_strength"),
            scaling_power: self.controls.get("scaling_power"),
            auto_hue_shift: self.controls.bool("auto_hue_shift") as i32 as f32,
            r: self.controls.get("r"),
            g: self.controls.get("g"),
            b: self.controls.get("b"),
            offset: self.controls.get("offset"),
            ring_strength: self.controls.get("ring_strength"),
            angular_variation: self.controls.get("angular_variation"),
            threshold: self.controls.get("threshold"),
            mix: self.controls.get("mix"),
            alg: match self.controls.string("alg").as_str() {
                "distance" => 0.0,
                "concentric_waves" => 1.0,
                "moire" => 2.0,
                _ => unreachable!(),
            },
            j: self.controls.get("j"),
            k: self.controls.get("k"),
            time: app.time,
        };

        self.gpu.update_params(app, wr.resolution_u32(), &params);
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &LatticeContext) {
        self.gpu.render(&frame);
    }
}
