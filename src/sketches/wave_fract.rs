use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "wave_fract",
    display_name: "Wave Fract",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(540),
};

#[derive(LegacySketchComponents)]
pub struct Model {
    controls: ControlScript<Timing>,
    wr: WindowRect,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderParams {
    // w, h, ..unused
    resolution: [f32; 4],

    // wave_phase, wave_radial_freq, wave_horiz_freq, wave_vert_freq
    a: [f32; 4],

    // bg_freq, bg_radius, bg_gradient_strength, wave_power
    b: [f32; 4],

    // reduce_mix, map_mix, wave_bands, wave_threshold
    c: [f32; 4],

    // bg_invert, unused, mix_mode, unused
    d: [f32; 4],
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let controls = ControlScript::from_path(
        to_absolute_path(file!(), "wave_fract.yaml"),
        Timing::new(SKETCH_CONFIG.bpm),
    );

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
        c: [0.0; 4],
        d: [0.0; 4],
    };

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "wave_fract.wgsl"),
        &params,
        true,
    );

    Model { controls, wr, gpu }
}

pub fn update(app: &App, m: &mut Model, _update: Update) {
    m.controls.update();

    let params = ShaderParams {
        resolution: [m.wr.w(), m.wr.h(), 0.0, 0.0],
        a: [
            if m.controls.bool("animate_wave_phase") {
                m.controls.get("wave_phase_animation")
            } else {
                m.controls.get("wave_phase")
            },
            m.controls.get("wave_radial_freq"),
            m.controls.get("wave_horiz_freq"),
            if m.controls.bool("link_axes") {
                m.controls.get("wave_horiz_freq")
            } else {
                m.controls.get("wave_vert_freq")
            },
        ],
        b: [
            m.controls.get("bg_freq"),
            m.controls.get("bg_radius"),
            m.controls.get("bg_gradient_strength"),
            m.controls.get("wave_power"),
        ],
        c: [
            m.controls.get("reduce_mix"),
            m.controls.get("map_mix"),
            m.controls.get("wave_bands"),
            m.controls.get("wave_threshold"),
        ],
        d: [
            bool_to_f32(m.controls.bool("bg_invert")),
            0.0,
            match m.controls.string("mix_mode").as_str() {
                "mix" => 0.0,
                "min_max" => 1.0,
                _ => unreachable!(),
            },
            0.0,
        ],
    };

    m.gpu.update_params(app, m.wr.resolution_u32(), &params);
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    frame.clear(BLACK);
    m.gpu.render(&frame);
}
