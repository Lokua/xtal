use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "kalos_2",
    display_name: "Kalos 2",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(440),
};

#[derive(SketchComponents)]
pub struct Kalos2 {
    animation: Animation<Timing>,
    controls: Controls,
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

pub fn init(app: &App, ctx: LatticeContext) -> Kalos2 {
    let resolution = ctx.window_rect().resolution_u32();
    let animation = Animation::new(Timing::new(ctx.bpm()));

    let disable = |_controls: &Controls| true;

    let controls = Controls::with_previous(vec![
        Control::slider_x("offset", 0.2, (0.0, 1.0), 0.0001, disable),
        Control::slider_x("radius", 0.5, (0.0, 10.0), 0.01, disable),
        Control::slider_x("strength", 0.5, (0.0, 5.0), 0.001, disable),
        Control::slider("scaling_power", 1.0, (0.01, 20.0), 0.01),
        Control::Separator {},
        Control::slide("r", 0.5),
        Control::slide("g", 0.0),
        Control::slide("b", 1.0),
        Control::Separator {},
        Control::slider("ring_strength", 20.0, (1.0, 100.0), 0.01),
        Control::slider("ring_harmonics", 1.0, (1.0, 10.0), 1.0),
        Control::slider("ring_harm_amt", 1.0, (1.0, 100.0), 1.0),
        Control::slider("angular_variation", 4.0, (1.0, 45.0), 1.0),
        Control::slider("frequency", 1.0, (0.0, 1000.0), 1.0),
        Control::slide("lerp", 0.0),
        Control::slide("threshold", 0.5),
        Control::slide("mix", 0.5),
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

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        resolution,
        to_absolute_path(file!(), "kalos_2.wgsl"),
        &params,
        true,
    );

    Kalos2 {
        animation,
        controls,
        gpu,
    }
}

impl Sketch for Kalos2 {
    fn update(&mut self, app: &App, _update: Update, ctx: &LatticeContext) {
        let wr = ctx.window_rect();
        let a = &self.animation;

        let r_range = self.controls.slider_range("radius");
        let s_range = self.controls.slider_range("strength");

        let gen_anim = |dur: f32, delay: f32, anim_scaling: bool| {
            [
                // radius
                a.r_ramp(
                    &[kfr(r_range, dur)],
                    delay,
                    dur * 0.5,
                    Easing::Linear,
                ),
                // strength
                a.r_ramp(
                    &[kfr(s_range, dur * 1.5)],
                    delay + 1.0,
                    dur * 0.75,
                    Easing::Linear,
                ),
                // scaling_power
                if anim_scaling {
                    self.controls.float("scaling_power")
                } else {
                    (a.tri(8.0) + 1.0) * 4.0
                },
                // offset
                a.r_ramp(&[kfr((0.0, 1.0), 16.0)], 0.0, 8.0, Easing::Linear),
            ]
        };

        let corner = gen_anim(16.0, 0.0, true);

        let params = ShaderParams {
            resolution: [wr.w(), wr.h(), 0.0, 0.0],
            d_0: gen_anim(32.0, 0.0, false),
            d_1: corner,
            d_2: corner,
            d_3: corner,
            d_4: corner,
            radius: self.controls.float("radius"),
            strength: self.controls.float("strength"),
            scaling_power: self.controls.float("scaling_power"),
            r: self.controls.float("r"),
            g: self.controls.float("g"),
            b: self.controls.float("b"),
            offset: a.tri(64.0),
            ring_strength: self.controls.float("ring_strength"),
            ring_harmonics: self.controls.float("ring_harmonics"),
            ring_harm_amt: self.controls.float("ring_harm_amt"),
            angular_variation: self.controls.float("angular_variation"),
            frequency: self.controls.float("frequency"),
            lerp: self.controls.float("lerp"),
            threshold: self.controls.float("threshold"),
            mix: self.controls.float("mix"),
            time: app.time,
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
