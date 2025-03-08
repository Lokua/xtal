use nannou::prelude::*;

use crate::framework::prelude::*;

// ~/Documents/Live/2025/2025.01.11 Lattice - Spiral

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "g25_10_11_12",
    display_name: "Genuary 10-12: Spiral (Automated)",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(680),
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderParams {
    // w, h, ..unused
    resolution: [f32; 4],

    // start_x, start_y, end_x, end_y
    a: [f32; 4],

    // points_per_segment, noise_scale, angle_variation, n_lines
    b: [f32; 4],

    // point_size, circle_r_min, circle_r_max, offset_mult
    c: [f32; 4],

    // bg_brightness, time, invert, animate_angle_offset
    d: [f32; 4],

    // wave_amp, wave_freq, stripe_amp, stripe_freq
    e: [f32; 4],

    // animate_bg, steep_amp, steep_freq, steepness
    f: [f32; 4],

    // quant_amp, quant_freq, quant_phase, steep_phase
    g: [f32; 4],

    // wave_phase, stripe_phase, harmonic_influence, unused
    h: [f32; 4],
}

#[derive(SketchComponents)]
pub struct Template {
    animation: Animation<Timing>,
    controls: Controls,
    gpu: gpu::GpuState<()>,
    midi: MidiControls,
}

pub fn init(app: &App, ctx: &LatticeContext) -> Template {
    let animation = Animation::new(Timing::new(ctx.bpm()));

    let disabled = |_controls: &Controls| true;

    let controls = Controls::with_previous(vec![
        // 1 "pass" = 1 million vertices
        Control::slider("passes", 1.0, (1.0, 20.0), 1.0),
        Control::slider_x("n_lines", 64.0, (1.0, 256.0), 1.0, disabled),
        Control::slider_x(
            "points_per_segment",
            100.0,
            (10.0, 20_000.0),
            10.0,
            disabled,
        ),
        Control::slider("point_size", 0.001, (0.0005, 0.01), 0.0001),
        Control::slider("harmonic_influence", 0.2, (0.01, 10.0), 0.01),
        Control::Separator {}, // -----------------------------------
        Control::checkbox("invert", false),
        Control::checkbox("animate_bg", false),
        Control::checkbox("animate_angle_offset", false),
        Control::slider("bg_brightness", 1.5, (0.0, 5.0), 0.01),
        Control::slider("phase_animation_mult", 1.0, (0.0, 1.0), 0.125),
        Control::Separator {}, // -----------------------------------
        Control::checkbox("animate_wave_phase", false),
        Control::checkbox("invert_animate_wave_phase", false),
        Control::slider_x(
            "wave_phase",
            0.0,
            (0.0, TAU),
            0.001,
            |controls: &Controls| controls.bool("animate_wave_phase"),
        ),
        Control::Separator {}, // -----------------------------------
        Control::checkbox("animate_stripe_phase", false),
        Control::checkbox("invert_animate_stripe_phase", false),
        Control::slider_x(
            "stripe_phase",
            0.0,
            (0.0, TAU),
            0.001,
            |controls: &Controls| controls.bool("animate_stripe_phase"),
        ),
        Control::Separator {}, // -----------------------------------
        Control::slider("steepness", 10.0, (1.0, 100.0), 1.0),
        Control::checkbox("animate_steep_phase", false),
        Control::checkbox("invert_animate_steep_phase", false),
        Control::slider_x(
            "steep_phase",
            0.0,
            (0.0, TAU),
            0.001,
            |controls: &Controls| controls.bool("animate_steep_phase"),
        ),
        Control::Separator {}, // -----------------------------------
        Control::checkbox("animate_quant_phase", false),
        Control::checkbox("invert_animate_quant_phase", false),
        Control::slider_x(
            "quant_phase",
            0.0,
            (0.0, TAU),
            0.001,
            |controls: &Controls| controls.bool("animate_quant_phase"),
        ),
    ]);

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
        c: [0.0; 4],
        d: [0.0; 4],
        e: [0.0; 4],
        f: [0.0; 4],
        g: [0.0; 4],
        h: [0.0; 4],
    };

    let gpu = gpu::GpuState::new_procedural(
        app,
        ctx.window_rect().resolution_u32(),
        to_absolute_path(file!(), "./g25_10_11_12.wgsl"),
        &params,
        true,
    );

    let midi = MidiControlBuilder::new()
        .control("n_lines", (0, 1), (1.0, 256.0), 0.5)
        .control("points_per_segment", (0, 2), (10.0, 20_000.0), 0.75)
        // ---
        .control("noise_scale", (0, 3), (0.0, 0.1), 0.025)
        .control("angle_variation", (0, 4), (0.0, TAU), 0.2)
        .control("offset_mult", (0, 5), (0.0, 10.0), 0.0)
        .control_n("circle_r_min", (0, 6), 0.0)
        .control_n("circle_r_max", (0, 7), 1.0)
        // ---
        .control_n("wave_amp", (0, 8), 0.0)
        .control_n("steep_amp", (0, 9), 0.0)
        .control_n("quant_amp", (0, 10), 0.0)
        .control_n("stripe_amp", (0, 11), 0.0)
        .control("steep_freq", (0, 12), (0.00, 64.0), 1.0)
        .control("quant_freq", (0, 13), (0.00, 64.0), 1.0)
        .control("stripe_freq", (0, 14), (0.00, 64.0), 1.0)
        .control("wave_freq", (0, 15), (0.00, 64.0), 1.0)
        .build();

    Template {
        animation,
        controls,
        gpu,
        midi,
    }
}

impl Sketch for Template {
    fn update(&mut self, app: &App, _update: Update, ctx: &LatticeContext) {
        let params = ShaderParams {
            resolution: [
                ctx.window_rect().w(),
                ctx.window_rect().h(),
                0.0,
                0.0,
            ],
            a: [-0.9, 0.0, 0.9, 0.0],
            b: [
                self.midi.get("points_per_segment"),
                self.midi.get("noise_scale"),
                self.midi.get("angle_variation"),
                self.midi.get("n_lines"),
            ],
            c: [
                self.controls.float("point_size"),
                self.midi.get("circle_r_min"),
                self.midi.get("circle_r_max"),
                self.midi.get("offset_mult"),
            ],
            d: [
                self.controls.float("bg_brightness"),
                self.animation.tri(64.0),
                bool_to_f32(self.controls.bool("invert")),
                bool_to_f32(self.controls.bool("animate_angle_offset")),
            ],
            e: [
                self.midi.get("wave_amp"),
                self.midi.get("wave_freq").ceil(),
                self.midi.get("stripe_amp"),
                self.midi.get("stripe_freq").ceil(),
            ],
            f: [
                bool_to_f32(self.controls.bool("animate_bg")),
                self.midi.get("steep_amp"),
                self.midi.get("steep_freq").ceil(),
                self.controls.float("steepness"),
            ],
            g: [
                self.midi.get("quant_amp"),
                self.midi.get("quant_freq").ceil(),
                get_phase(self, "quant", 24.0),
                get_phase(self, "steep", 48.0),
            ],
            h: [
                get_phase(self, "wave", 32.0),
                get_phase(self, "stripe", 56.0),
                self.controls.float("harmonic_influence"),
                0.0,
            ],
        };

        self.gpu.update_params(
            app,
            ctx.window_rect().resolution_u32(),
            &params,
        );
        self.controls.mark_unchanged();
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &LatticeContext) {
        frame.clear(WHITE);

        let points_per_line = self.midi.get("points_per_segment") as u32;
        let n_lines = self.midi.get("n_lines") as u32;
        let total_points = points_per_line * n_lines;
        let density = self.controls.float("passes") as u32;
        let spiral_vertices = total_points * 6 * density;
        let background_vertices = 3;
        let total_vertices = background_vertices + spiral_vertices;

        self.gpu.render_procedural(&frame, total_vertices);
    }
}

fn get_phase(
    template: &Template,
    param_name: &str,
    animation_time: f32,
) -> f32 {
    let animate_param = format!("animate_{}_phase", param_name);
    let invert_param = format!("invert_animate_{}_phase", param_name);
    let phase_param = format!("{}_phase", param_name);
    let time = animation_time * template.controls.float("phase_animation_mult");

    if template.controls.bool(&animate_param) {
        if template.controls.bool(&invert_param) {
            template.animation.loop_phase(time) * TAU
        } else {
            (1.0 - template.animation.loop_phase(time)) * TAU
        }
    } else {
        template.controls.float(&phase_param)
    }
}
