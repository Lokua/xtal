use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "genuary_19",
    display_name: "Genuary 19: Op Art",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(540),
};

#[derive(SketchComponents)]
pub struct Model {
    #[allow(dead_code)]
    animation: Animation<OscTransportTiming>,
    osc: OscControls,
    controls: Controls,
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
    let animation = Animation::new(OscTransportTiming::new(SKETCH_CONFIG.bpm));

    let controls = Controls::with_previous(vec![
        Control::slider("wave_power", 5.0, (0.0, 10.0), 0.01),
        Control::slider("wave_bands", 0.0, (2.0, 10.0), 1.0),
        Control::slider("wave_threshold", 0.0, (-1.0, 1.0), 0.001),
        Control::Separator {}, // -------------------
        Control::checkbox("bg_invert", false),
        Control::slider("bg_freq", 10.0, (0.0, 100.0), 1.0),
        Control::slider_norm("bg_radius", 0.5),
        Control::slider_norm("bg_gradient_strength", 0.5),
        Control::Separator {}, // -------------------
        Control::slider_norm("reduce_mix", 0.5),
        Control::select("mix_mode", "mix", &["mix", "min_max"]),
        Control::slider_norm("map_mix", 0.5),
    ]);

    let osc = OscControlBuilder::new()
        .control_mapped("/wave_phase", (0.0, TAU), 0.0)
        .control_mapped("/wave_radial_freq", (0.0, 100.0), 0.0)
        .control_mapped("/wave_horiz_freq", (0.0, 100.0), 0.0)
        .control_mapped("/wave_vert_freq", (0.0, 100.0), 0.0)
        .build();

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
        c: [0.0; 4],
        d: [0.0; 4],
    };

    let gpu = gpu::GpuState::new_full_screen(
        app,
        to_absolute_path(file!(), "./genuary_19.wgsl"),
        &params,
        true,
    );

    Model {
        animation,
        osc,
        controls,
        wr,
        gpu,
    }
}

pub fn update(app: &App, m: &mut Model, _update: Update) {
    let params = ShaderParams {
        resolution: [m.wr.w(), m.wr.h(), 0.0, 0.0],
        a: [
            m.osc.get("/wave_phase"),
            m.osc.get("/wave_radial_freq"),
            m.osc.get("/wave_horiz_freq"),
            m.osc.get("/wave_vert_freq"),
        ],
        b: [
            m.controls.float("bg_freq"),
            m.controls.float("bg_radius"),
            m.controls.float("bg_gradient_strength"),
            m.controls.float("wave_power"),
        ],
        c: [
            m.controls.float("reduce_mix"),
            m.controls.float("map_mix"),
            m.controls.float("wave_bands"),
            m.controls.float("wave_threshold"),
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

    m.gpu.update_params(app, &params);
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    frame.clear(BLACK);
    m.gpu.render(&frame);
}
