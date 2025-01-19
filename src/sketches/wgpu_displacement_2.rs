use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "wgpu_displacement_2",
    display_name: "WGPU Displacement 2",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(440),
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
    // 4 since 2 gives alignment problems for some unknown reason
    resolution: [f32; 4],

    // displacer "instance" params
    // center, top-right, bottom-right, bottom-left, top-left
    // [radius, strength, scale, offset]
    d_0: [f32; 4],
    d_1: [f32; 4],
    d_2: [f32; 4],
    d_3: [f32; 4],
    d_4: [f32; 4],

    radius: f32,
    strength: f32,
    scaling_power: f32,
    r: f32,
    g: f32,
    b: f32,
    offset: f32,
    ring_strength: f32,
    ring_harmonics: f32,
    ring_harm_amt: f32,
    angular_variation: f32,
    lerp: f32,
    frequency: f32,
    threshold: f32,
    mix: f32,
    time: f32,
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let animation = Animation::new(FrameTiming::new(SKETCH_CONFIG.bpm));

    let disable = |_controls: &Controls| true;

    let controls = Controls::with_previous(vec![
        Control::slider_x("offset", 0.2, (0.0, 1.0), 0.0001, disable),
        Control::slider_x("radius", 0.5, (0.0, 10.0), 0.01, disable),
        Control::slider_x("strength", 0.5, (0.0, 5.0), 0.001, disable),
        Control::slider("scaling_power", 1.0, (0.01, 20.0), 0.01),
        Control::Separator {},
        Control::slider_norm("r", 0.5),
        Control::slider_norm("g", 0.0),
        Control::slider_norm("b", 1.0),
        Control::Separator {},
        Control::slider("ring_strength", 20.0, (1.0, 100.0), 0.01),
        Control::slider("ring_harmonics", 1.0, (1.0, 10.0), 1.0),
        Control::slider("ring_harm_amt", 1.0, (1.0, 100.0), 1.0),
        Control::slider("angular_variation", 4.0, (1.0, 45.0), 1.0),
        Control::slider("frequency", 1.0, (0.0, 1000.0), 1.0),
        Control::slider_norm("lerp", 0.0),
        Control::slider_norm("threshold", 0.5),
        Control::slider_norm("mix", 0.5),
    ]);

    let params = ShaderParams {
        resolution: [0.0; 4],
        d_0: [0.0; 4],
        d_1: [0.0; 4],
        d_2: [0.0; 4],
        d_3: [0.0; 4],
        d_4: [0.0; 4],
        radius: 0.0,
        strength: 0.0,
        scaling_power: 0.0,
        r: 0.0,
        g: 0.0,
        b: 0.0,
        offset: 0.0,
        ring_strength: 0.0,
        ring_harmonics: 0.0,
        ring_harm_amt: 0.0,
        angular_variation: 0.0,
        frequency: 0.0,
        lerp: 0.0,
        threshold: 0.0,
        mix: 0.0,
        time: app.time,
    };

    let gpu = gpu::GpuState::new_full_screen(
        app,
        to_absolute_path(file!(), "./wgpu_displacement_2.wgsl"),
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
    let a = &m.animation;

    let r_range = m.controls.slider_range("radius");
    let s_range = m.controls.slider_range("strength");

    let gen_anim = |dur: f32, delay: f32, anim_scaling: bool| {
        [
            // radius
            a.r_ramp(&[kfr(r_range, dur)], delay, dur * 0.5, linear),
            // strength
            a.r_ramp(
                &[kfr(s_range, dur * 1.5)],
                delay + 1.0,
                dur * 0.75,
                linear,
            ),
            // scaling_power
            if anim_scaling {
                m.controls.float("scaling_power")
            } else {
                (a.ping_pong(8.0) + 1.0) * 4.0
            },
            // offset
            a.r_ramp(&[kfr((0.0, 1.0), 16.0)], 0.0, 8.0, linear),
        ]
    };

    let corner = gen_anim(16.0, 0.0, true);

    let params = ShaderParams {
        resolution: [m.wr.w(), m.wr.h(), 0.0, 0.0],
        d_0: gen_anim(32.0, 0.0, false),
        d_1: corner,
        d_2: corner,
        d_3: corner,
        d_4: corner,
        radius: m.controls.float("radius"),
        strength: m.controls.float("strength"),
        scaling_power: m.controls.float("scaling_power"),
        r: m.controls.float("r"),
        g: m.controls.float("g"),
        b: m.controls.float("b"),
        offset: a.ping_pong(64.0),
        ring_strength: m.controls.float("ring_strength"),
        ring_harmonics: m.controls.float("ring_harmonics"),
        ring_harm_amt: m.controls.float("ring_harm_amt"),
        angular_variation: m.controls.float("angular_variation"),
        frequency: m.controls.float("frequency"),
        lerp: m.controls.float("lerp"),
        threshold: m.controls.float("threshold"),
        mix: m.controls.float("mix"),
        time: app.time,
    };

    m.gpu.update_params(app, &params);
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    frame.clear(BLACK);
    m.gpu.render(&frame);
}
