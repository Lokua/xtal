use nannou::prelude::*;

use crate::framework::prelude::*;

// b/w Ableton 2025/Lattice - Wave Fract

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "g25_19_op_art",
    display_name: "Genuary 19: Op Art",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 127.0,
    w: 700,
    h: 700,
    // h: 1244,
    gui_w: None,
    gui_h: Some(540),
};

#[derive(SketchComponents)]
pub struct Template {
    controls: ControlHub<OscTransportTiming>,
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

    // bg_invert, wave1_mod, mix_mode, wave_scale
    d: [f32; 4],

    // wave1_mix, wave2_mix, wave3_mix, unused
    e: [f32; 4],
}

pub fn init(app: &App, ctx: &LatticeContext) -> Template {
    let controls = ControlHubBuilder::new()
        .timing(OscTransportTiming::new(ctx.bpm()))
        .slider("wave_power", 5.0, (0.0, 10.0), 0.01, None)
        .slider("wave_bands", 0.0, (2.0, 10.0), 1.0, None)
        .slider("wave_threshold", 0.0, (-1.0, 1.0), 0.001, None)
        .separator()
        .checkbox("bg_invert", false, None)
        .slider_n("bg_radius", 0.5)
        .slider_n("bg_gradient_strength", 0.5)
        .separator()
        .select("mix_mode", "mix", &["mix", "min_max"], None)
        .osc("wave_phase", (0.0, TAU), 0.0)
        .osc("wave_radial_freq", (0.0, 100.0), 0.0)
        .osc("wave_horiz_freq", (0.0, 100.0), 0.0)
        .osc("wave_vert_freq", (0.0, 100.0), 0.0)
        .osc_n("reduce_mix", 0.0)
        .osc_n("map_mix", 0.0)
        .osc_n("wave_scale", 0.0)
        .osc("bg_freq", (0.0, 100.0), 90.0)
        .osc_n("wave1_mix", 0.0)
        .osc_n("wave2_mix", 0.0)
        .osc_n("wave3_mix", 0.0)
        .build();

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
        c: [0.0; 4],
        d: [0.0; 4],
        e: [0.0; 4],
    };

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        ctx.window_rect().resolution_u32(),
        to_absolute_path(file!(), "./g25_19_op_art.wgsl"),
        &params,
        true,
    );

    Template { controls, gpu }
}

impl Sketch for Template {
    fn update(&mut self, app: &App, _update: Update, ctx: &LatticeContext) {
        let wr = ctx.window_rect();

        let params = ShaderParams {
            resolution: [wr.w(), wr.h(), 0.0, 0.0],
            a: [
                self.controls.get("wave_phase"),
                self.controls.get("wave_radial_freq"),
                self.controls.get("wave_horiz_freq"),
                self.controls.get("wave_vert_freq"),
            ],
            b: [
                self.controls.get("bg_freq"),
                self.controls.get("bg_radius"),
                self.controls.get("bg_gradient_strength"),
                self.controls.get("wave_power"),
            ],
            c: [
                self.controls.get("reduce_mix"),
                self.controls.get("map_mix"),
                self.controls.get("wave_bands"),
                self.controls.get("wave_threshold"),
            ],
            d: [
                bool_to_f32(self.controls.bool("bg_invert")),
                self.controls.animation.tri(8.0) * 2.0 - 1.0,
                match self.controls.string("mix_mode").as_str() {
                    "mix" => 0.0,
                    "min_max" => 1.0,
                    _ => unreachable!(),
                },
                self.controls.get("wave_scale"),
            ],
            e: [
                self.controls.get("wave1_mix"),
                self.controls.get("wave2_mix"),
                self.controls.get("wave3_mix"),
                0.0,
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
