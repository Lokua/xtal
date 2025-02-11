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
pub struct Model {
    #[allow(dead_code)]
    animation: Animation<Timing>,
    controls: Controls,
    wr: WindowRect,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderParams {
    // w, h ..unused
    resolution: [f32; 4],

    // constrast, smooth_mix, time, time_2
    a: [f32; 4],

    // t1, t2, t3, post_mix
    b: [f32; 4],

    // r1, r2, r3, unused
    c: [f32; 4],

    // g1, g2, g3, unused
    d: [f32; 4],
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let animation = Animation::new(Timing::new(SKETCH_CONFIG.bpm));

    let disable = |_controls: &Controls| true;

    let controls = Controls::with_previous(vec![
        Control::slider_norm("smooth_mix", 0.5),
        Control::slider("contrast", 1.5, (0.1, 5.0), 0.1),
        Control::Separator {},
        Control::slider_x("g1", 2.0, (2.0, 32.0), 1.0, disable),
        Control::slider_x("g2", 4.0, (2.0, 32.0), 1.0, disable),
        Control::slider_x("g3", 8.0, (2.0, 32.0), 1.0, disable),
        Control::Separator {},
        Control::slider_norm("post_mix", 0.5),
    ]);

    let params = ShaderParams {
        resolution: [0.0, 0.0, 0.0, 0.0],
        a: [0.0, 0.0, 0.0, 0.0],
        b: [0.0, 0.0, 0.0, 0.0],
        c: [0.0, 0.0, 0.0, 0.0],
        d: [0.0, 0.0, 0.0, 0.0],
    };

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "./g25_2_layers.wgsl"),
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
    let time = 8.0;
    let kfs = [kfr((0.0, 1.0), time)];

    let params = ShaderParams {
        resolution: [m.wr.w(), m.wr.h(), 0.0, 0.0],
        a: [
            m.controls.float("contrast"),
            m.controls.float("smooth_mix"),
            m.animation.tri(8.0),
            m.animation.tri(12.0),
        ],
        b: [
            m.animation.r_ramp(&kfs, 0.0, time * 0.5, ease_in_out),
            m.animation.r_ramp(&kfs, 0.5, time * 0.5, ease_in_out),
            m.animation.r_ramp(&kfs, 0.1, time * 0.5, ease_in_out),
            m.controls.float("post_mix"),
        ],
        c: [
            m.animation.r_ramp(&kfs, 0.0, time * 0.5, ease_in_out),
            m.animation.r_ramp(&kfs, 0.5, time * 0.5, ease_in_out),
            m.animation.r_ramp(&kfs, 0.1, time * 0.5, ease_in_out),
            0.0,
        ],
        d: [
            // m.controls.float("g1"),
            // m.controls.float("g2"),
            // m.controls.float("g3"),
            // ---
            2.0,
            m.animation.lrp(
                &[
                    kf(2.0, 4.0),  // stay
                    kf(2.0, 4.0),  // transition
                    kf(3.0, 24.0), // stay
                    kf(3.0, 6.0),  // transition
                ],
                0.0,
            ),
            m.animation.lrp(
                &[
                    kf(2.0, 4.0), // stay
                    kf(2.0, 4.0), // transition
                    kf(4.0, 8.0), // stay
                    kf(4.0, 8.0), // transition
                    kf(8.0, 8.0), // stay
                    kf(8.0, 6.0), // transition
                ],
                0.0,
            ),
            0.0,
        ],
    };

    m.gpu.update_params(app, m.wr.resolution_u32(), &params);
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    frame.clear(BLACK);
    m.gpu.render(&frame);
}
