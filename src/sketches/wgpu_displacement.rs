use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "wgpu_displacement",
    display_name: "WGPU Displacement",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(560),
};

#[derive(SketchComponents)]
pub struct Model {
    #[allow(dead_code)]
    animation: Animation<FrameTiming>,
    controls: Controls,
    #[allow(dead_code)]
    wr: WindowRect,
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

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let animation = Animation::new(FrameTiming::new(SKETCH_CONFIG.bpm));

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
        Control::slider_norm("offset", 0.2),
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
        Control::slider_norm("r", 0.5),
        Control::slider_norm("g", 0.0),
        Control::slider_norm("b", 1.0),
        Control::Separator {},
        Control::slider_norm("threshold", 0.5),
        Control::slider_norm("mix", 0.5),
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

    let gpu = gpu::GpuState::new_full_screen(
        app,
        to_absolute_path(file!(), "./wgpu_displacement.wgsl"),
        &params,
        true,
    );

    Model {
        animation,
        controls,
        wr,
        gpu,
    }
}

pub fn update(app: &App, m: &mut Model, _update: Update) {
    let strength = m.controls.float("strength");
    let strength_range = m.controls.slider_range("strength");
    let strength_swing = 0.05;

    let params = ShaderParams {
        resolution: [m.wr.w(), m.wr.h(), 0.0, 0.0],
        show_center: m.controls.bool("show_center") as i32 as f32,
        show_corners: m.controls.bool("show_corners") as i32 as f32,
        radius: m.controls.float("radius"),
        strength: if m.controls.bool("animate") {
            m.animation.lrp(
                &[
                    (
                        clamp(
                            strength - strength_swing,
                            strength_range.0,
                            strength_range.1,
                        ),
                        1.0,
                    ),
                    (
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
        corner_radius: m.controls.float("corner_radius"),
        corner_strength: m.controls.float("corner_strength"),
        scaling_power: m.controls.float("scaling_power"),
        auto_hue_shift: m.controls.bool("auto_hue_shift") as i32 as f32,
        r: m.controls.float("r"),
        g: m.controls.float("g"),
        b: m.controls.float("b"),
        offset: m.controls.float("offset"),
        ring_strength: m.controls.float("ring_strength"),
        angular_variation: m.controls.float("angular_variation"),
        threshold: m.controls.float("threshold"),
        mix: m.controls.float("mix"),
        alg: match m.controls.string("alg").as_str() {
            "distance" => 0.0,
            "concentric_waves" => 1.0,
            "moire" => 2.0,
            _ => unreachable!(),
        },
        j: m.controls.float("j"),
        k: m.controls.float("k"),
        time: app.time,
    };

    m.gpu.update_params(app, &params);
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    m.gpu.render(&frame);
}
