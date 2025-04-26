use nannou::prelude::*;

use xtal::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "wave_fract",
    display_name: "Wave Fract",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
};

#[derive(SketchComponents)]
pub struct WaveFract {
    hub: ControlHub<Timing>,
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

    // bg_invert, unused, mix_mode, x_off
    d: [f32; 4],

    // r, g, b, y_off
    e: [f32; 4],

    // unused
    f: [f32; 4],
}

pub fn init(app: &App, ctx: &Context) -> WaveFract {
    let window_rect = ctx.window_rect().clone();

    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "wave_fract.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
        c: [0.0; 4],
        d: [0.0; 4],
        e: [0.0; 4],
        f: [0.0; 4],
    };

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        window_rect.resolution_u32(),
        to_absolute_path(file!(), "wave_fract.wgsl"),
        &params,
        0,
        true,
    );

    WaveFract { hub, gpu }
}

impl Sketch for WaveFract {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();

        let params = ShaderParams {
            resolution: [wr.w(), wr.h(), 0.0, 0.0],
            a: [
                self.hub.select(
                    "animate_wave_phase",
                    "wave_phase_animation",
                    "wave_phase",
                ),
                self.hub.get("wave_radial_freq"),
                self.hub.get("wave_horiz_freq"),
                self.hub.select(
                    "link_axes",
                    "wave_horiz_freq",
                    "wave_vert_freq",
                ),
            ],
            b: [
                self.hub.get("bg_freq"),
                self.hub.get("bg_radius"),
                self.hub.get("bg_gradient_strength"),
                self.hub.get("wave_power"),
            ],
            c: [
                self.hub.get("reduce_mix"),
                self.hub.get("map_mix"),
                self.hub.get("wave_bands"),
                self.hub.get("wave_threshold"),
            ],
            d: [
                self.hub.get("bg_invert"),
                0.0,
                self.hub.get("mix_mode"),
                self.hub.get("x_off"),
            ],
            e: [
                self.hub.get("r"),
                self.hub.get("g"),
                self.hub.get("b"),
                self.hub.get("y_off"),
            ],
            f: [
                self.hub.get("f1"),
                self.hub.get("f2"),
                self.hub.get("f3"),
                self.hub.get("f4"),
            ],
        };

        self.gpu.update_params(
            app,
            ctx.window_rect().resolution_u32(),
            &params,
        );
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &Context) {
        frame.clear(BLACK);
        self.gpu.render(&frame);
    }
}
