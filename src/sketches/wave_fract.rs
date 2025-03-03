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

#[derive(SketchComponents)]
pub struct WaveFract {
    controls: ControlScript<Timing>,
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

pub fn init(app: &App, ctx: LatticeContext) -> WaveFract {
    let window_rect = ctx.window_rect().clone();

    let controls = ControlScript::from_path(
        to_absolute_path(file!(), "wave_fract.yaml"),
        Timing::new(ctx.bpm),
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
        window_rect.resolution_u32(),
        to_absolute_path(file!(), "wave_fract.wgsl"),
        &params,
        true,
    );

    WaveFract { controls, gpu }
}

impl Sketch for WaveFract {
    fn update(&mut self, app: &App, _update: Update, ctx: &LatticeContext) {
        let wr = ctx.window_rect();
        self.controls.update();

        let params = ShaderParams {
            resolution: [wr.w(), wr.h(), 0.0, 0.0],
            a: [
                if self.controls.bool("animate_wave_phase") {
                    self.controls.get("wave_phase_animation")
                } else {
                    self.controls.get("wave_phase")
                },
                self.controls.get("wave_radial_freq"),
                self.controls.get("wave_horiz_freq"),
                if self.controls.bool("link_axes") {
                    self.controls.get("wave_horiz_freq")
                } else {
                    self.controls.get("wave_vert_freq")
                },
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
                0.0,
                match self.controls.string("mix_mode").as_str() {
                    "mix" => 0.0,
                    "min_max" => 1.0,
                    _ => unreachable!(),
                },
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
