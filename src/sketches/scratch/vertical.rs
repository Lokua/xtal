use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "vertical",
    display_name: "Vertical",
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(500),
    play_mode: PlayMode::Loop,
};

#[derive(SketchComponents)]
pub struct Vertical {
    controls: ControlHub<Timing>,
    lines: Vec<Vec<Point2>>,
    patterns: Vec<XModFn>,
}

pub fn init(_app: &App, ctx: &Context) -> Vertical {
    let mode_options = [str_vec!["multi_lerp"], XMods::to_names()].concat();

    fn disabled_unless_modes(modes: Vec<String>) -> DisabledFn {
        Some(Box::new(move |controls| {
            !modes.contains(&controls.string("mode"))
        }))
    }

    let controls = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .slider("scale", 1.0, (0.1, 4.0), 0.1, None)
        .select("mode", "per_line", &mode_options, None)
        .slider("n_lines", 64.0, (16.0, 256.0), 2.0, None)
        .slider("amplitude", 20.0, (0.0, 300.0), 1.0, None)
        .slider("frequency", 0.1, (0.0, 0.1), 0.00001, None)
        .slider("weight", 1.0, (0.1, 4.0), 0.1, None)
        .slider(
            "x_line_scaling",
            0.1,
            (0.0, 0.5),
            0.01,
            disabled_unless_modes(str_vec![
                "multi_lerp",
                "per_line",
                "harmonic_cascade",
                "quantum_ripples",
            ]),
        )
        .slider(
            "x_phase_shift",
            0.1,
            (0.0, 1.0),
            0.01,
            disabled_unless_modes(str_vec![
                "multi_lerp",
                "wave_interference",
                "fractal_waves",
                "moire",
                "standing_waves",
            ]),
        )
        .slider(
            "x_harmonic_ratio",
            2.0,
            (1.0, 4.0),
            0.1,
            disabled_unless_modes(str_vec![
                "multi_lerp",
                "line_phase",
                "wave_interference",
                "harmonic_cascade",
                "fractal_waves",
                "moire",
                "quantum_ripples",
            ]),
        )
        .slider(
            "x_distance_scaling",
            0.05,
            (0.0, 0.2),
            0.01,
            disabled_unless_modes(str_vec!["multi_lerp", "ripples"]),
        )
        .slider(
            "x_complexity",
            1.0,
            (0.1, 3.0),
            0.1,
            disabled_unless_modes(str_vec![
                "multi_lerp",
                "spiral",
                "wave_interference",
                "fractal_waves",
                "moire",
                "standing_waves",
                "quantum_ripples",
            ]),
        )
        .build();

    let lines = Vec::with_capacity(controls.get("n_lines") as usize);

    Vertical {
        controls,
        lines,
        patterns: XMods::to_vec(),
    }
}

impl Sketch for Vertical {
    fn update(&mut self, _app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();

        let n_lines = self.controls.get("n_lines") as usize;
        let a = self.controls.get("amplitude");
        let f = self.controls.get("frequency");

        let params = XModParams {
            line_scaling: self.controls.get("x_line_scaling"),
            phase_shift: self.controls.get("x_phase_shift"),
            harmonic_ratio: self.controls.get("x_harmonic_ratio"),
            distance_scaling: self.controls.get("x_distance_scaling"),
            complexity: self.controls.get("x_complexity"),
        };

        self.lines = Vec::new();
        let step = wr.w() / n_lines as f32;
        let start_x = -(wr.w() / 2.0) + (step / 2.0);
        let n_points = (wr.h() / 2.0).floor() as usize;

        for i in 0..n_lines {
            let x = start_x + i as f32 * step;
            let mut points = Vec::new();

            for j in 0..n_points {
                let y =
                    map_range(j, 0, n_points - 1, -wr.h() / 2.0, wr.h() / 2.0);

                let x = match self.controls.string("mode").as_str() {
                    "multi_lerp" => {
                        let values = self
                            .patterns
                            .iter()
                            .map(|func| {
                                func(
                                    x,
                                    y,
                                    i as f32,
                                    a,
                                    f,
                                    n_lines as f32,
                                    &params,
                                )
                            })
                            .collect::<Vec<f32>>();

                        multi_lerp(&values, self.controls.animation.tri(24.0))
                    }
                    _ => {
                        let func =
                            XMods::func_by_name(self.controls.string("mode"));
                        func(x, y, i as f32, a, f, n_lines as f32, &params)
                    }
                };

                points.push(pt2(x, y));
            }

            self.lines.push(points);
        }
    }

    fn view(&self, app: &App, frame: Frame, ctx: &Context) {
        let window_rect = ctx.window_rect();
        let draw = app.draw();

        draw.rect()
            .x_y(0.0, 0.0)
            .w_h(window_rect.w(), window_rect.h())
            .hsla(0.0, 0.0, 1.0, 1.0);

        let zoomed_draw = draw.scale(self.controls.get("scale"));

        for line in self.lines.iter() {
            zoomed_draw
                .polyline()
                .weight(self.controls.get("weight"))
                .points(line.iter().cloned())
                .color(hsla(0.4, 0.0, 0.0, 0.9));
        }

        draw.to_frame(app, &frame).unwrap();
    }
}

type XModFn = fn(f32, f32, f32, f32, f32, f32, &XModParams) -> f32;

struct XModParams {
    line_scaling: f32,
    phase_shift: f32,
    harmonic_ratio: f32,
    distance_scaling: f32,
    complexity: f32,
}

impl XModParams {
    #[allow(dead_code)]
    fn default() -> Self {
        Self {
            line_scaling: 0.1,
            phase_shift: 0.1,
            harmonic_ratio: 2.0,
            distance_scaling: 0.05,
            complexity: 1.0,
        }
    }
}

struct XMods {}

impl XMods {
    fn to_names() -> Vec<String> {
        str_vec![
            "per_line",
            "ripples",
            "line_phase",
            "spiral",
            "wave_interference",
            "harmonic_cascade",
            "fractal_waves",
            "moire",
            "standing_waves",
            "quantum_ripples"
        ]
    }

    fn func_by_name(name: String) -> XModFn {
        match name.as_str() {
            "per_line" => XMods::per_line,
            "ripples" => XMods::ripples,
            "line_phase" => XMods::line_phase,
            "spiral" => XMods::spiral,
            "wave_interference" => XMods::wave_interference,
            "harmonic_cascade" => XMods::harmonic_cascade,
            "fractal_waves" => XMods::fractal_waves,
            "moire" => XMods::moire,
            "standing_waves" => XMods::standing_waves,
            "quantum_ripples" => XMods::quantum_ripples,
            _ => panic!("No function named '{name}'"),
        }
    }

    fn to_vec() -> Vec<XModFn> {
        XMods::to_names()
            .into_iter()
            .map(XMods::func_by_name)
            .collect()
    }

    fn per_line(
        x: f32,
        y: f32,
        i: f32,
        a: f32,
        f: f32,
        _n: f32,
        p: &XModParams,
    ) -> f32 {
        let freq = f * (1.0 + i * p.line_scaling);
        x + a * (y * freq).sin()
    }

    fn ripples(
        x: f32,
        y: f32,
        i: f32,
        a: f32,
        f: f32,
        n: f32,
        p: &XModParams,
    ) -> f32 {
        let distance_from_center = (i - n / 2.0).abs();
        let freq = f * (1.0 + distance_from_center * p.distance_scaling);
        x + a * (y * freq).sin()
    }

    fn line_phase(
        x: f32,
        y: f32,
        i: f32,
        a: f32,
        f: f32,
        _n: f32,
        p: &XModParams,
    ) -> f32 {
        let phase = i * p.phase_shift;
        x + a * (y * f + phase).sin() * (y * f * p.harmonic_ratio + phase).cos()
    }

    fn spiral(
        x: f32,
        y: f32,
        i: f32,
        a: f32,
        f: f32,
        n: f32,
        p: &XModParams,
    ) -> f32 {
        let angle = (i / n) * TAU;
        let radius = ((x * x + y * y).sqrt() * f).sin();
        x + a * (radius * angle * p.complexity).sin()
    }

    fn wave_interference(
        x: f32,
        y: f32,
        i: f32,
        a: f32,
        f: f32,
        _n: f32,
        p: &XModParams,
    ) -> f32 {
        let wave1 = (x * f + i * p.phase_shift).sin();
        let wave2 = (y * f * p.harmonic_ratio + i * p.phase_shift).sin();
        x + a * wave1 * wave2 * p.complexity
    }

    fn harmonic_cascade(
        x: f32,
        y: f32,
        i: f32,
        a: f32,
        f: f32,
        _n: f32,
        p: &XModParams,
    ) -> f32 {
        let base_wave = (y * f).sin();
        let harmonic1 = (y * f * p.harmonic_ratio).sin() * 0.5;
        let harmonic2 = (y * f * p.harmonic_ratio * 2.0).sin() * 0.25;
        x + a * (base_wave + harmonic1 + harmonic2) * (1.0 + i * p.line_scaling)
    }

    fn fractal_waves(
        x: f32,
        y: f32,
        i: f32,
        a: f32,
        f: f32,
        _n: f32,
        p: &XModParams,
    ) -> f32 {
        let mut sum = 0.0;
        let mut amplitude = a;
        let mut frequency = f;

        for _ in 0..3 {
            sum += (y * frequency + i * p.phase_shift).sin() * amplitude;
            amplitude *= 0.5;
            frequency *= p.harmonic_ratio;
        }
        x + sum * p.complexity
    }

    fn moire(
        x: f32,
        y: f32,
        i: f32,
        a: f32,
        f: f32,
        n: f32,
        p: &XModParams,
    ) -> f32 {
        let pattern1 = (x * f + i * p.phase_shift).sin();
        let pattern2 =
            (y * f * p.harmonic_ratio + (n - i) * p.phase_shift).sin();
        x + a * pattern1 * pattern2 * p.complexity
    }

    fn standing_waves(
        x: f32,
        y: f32,
        i: f32,
        a: f32,
        f: f32,
        _n: f32,
        p: &XModParams,
    ) -> f32 {
        let spatial = (x * f).sin() * (y * f).cos();
        let temporal = (i * p.phase_shift).cos();
        x + a * spatial * temporal * p.complexity
    }

    fn quantum_ripples(
        x: f32,
        y: f32,
        i: f32,
        a: f32,
        f: f32,
        n: f32,
        p: &XModParams,
    ) -> f32 {
        let distance = ((x * x + y * y).sqrt() + i * p.line_scaling) * f;
        let interference =
            (distance * p.harmonic_ratio).sin() * (distance / n).cos();
        x + a * interference * p.complexity
    }
}
