use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;
use crate::sketches::shared::sand_line::*;

// https://github.com/inconvergent/sand-spline/blob/master/main-hlines.py

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "sand_line",
    display_name: "Sand Line",
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(520),
    play_mode: PlayMode::Loop,
};

#[derive(SketchComponents)]
pub struct SandLineSketch {
    controls: Controls,
    ref_line: Vec<Vec2>,
    sand_line: Vec<Vec2>,
}

pub fn init(_app: &App, _ctx: &LatticeContext) -> SandLineSketch {
    let disable_octave =
        |controls: &Controls| controls.string("noise_strategy") != "Octave";

    let trig_fns = [
        "cos", "sin", "tan", "tanh", "sec", "csc", "cot", "sech", "csch",
        "coth",
    ];

    let controls = Controls::with_previous(vec![
        Control::select("noise_strategy", "Gaussian", &["Gaussian", "Octave"]),
        Control::select(
            "distribution_strategy",
            "Perpendicular",
            &["Perpendicular", "Curved", "TrigFn"],
        ),
        Control::checkbox("show_ref_line", false),
        Control::checkbox("show_sand_line", true),
        Control::Separator {},
        Control::slider("ref_segments", 16.0, (2.0, 64.0), 1.0),
        Control::slider("ref_deviation", 10.0, (1.0, 100.0), 1.0),
        Control::slider("ref_smooth", 2.0, (0.0, 10.0), 1.0),
        Control::Separator {},
        Control::slider("noise_scale", 8.0, (0.25, 32.0), 0.25),
        Control::slider_x(
            "noise_octaves",
            4.0,
            (1.0, 10.0),
            1.0,
            disable_octave,
        ),
        Control::slider_x(
            "noise_persistence",
            0.5,
            (0.0, 1.0),
            0.001,
            disable_octave,
        ),
        Control::Separator {},
        Control::slider("angle_variation", 0.5, (0.0, TWO_PI), 0.001),
        Control::slider("points_per_segment", 64.0, (2.0, 256.0), 1.0),
        Control::slider("passes", 50.0, (1.0, 256.0), 1.0),
        Control::slider_x("curvature", 0.5, (0.0, 2.0), 0.0001, |controls| {
            controls.string("distribution_strategy") != "Curved"
                && controls.string("distribution_strategy") != "TrigFn"
        }),
        Control::select_x("trig_fn_a", "cos", &trig_fns, |controls| {
            controls.string("distribution_strategy") != "TrigFn"
        }),
        Control::select_x("trig_fn_b", "sin", &trig_fns, |controls| {
            controls.string("distribution_strategy") != "TrigFn"
        }),
        Control::Separator {},
        Control::slide("alpha", 0.5),
    ]);

    SandLineSketch {
        controls,
        ref_line: vec![],
        sand_line: vec![],
    }
}

impl Sketch for SandLineSketch {
    fn update(&mut self, _app: &App, _update: Update, ctx: &LatticeContext) {
        if self.controls.changed() {
            let noise_strategy = self.controls.string("noise_strategy");
            let distribution_strategy =
                self.controls.string("distribution_strategy");

            let noise_scale = self.controls.float("noise_scale");
            let noise_octaves = self.controls.float("noise_octaves");
            let noise_persistence = self.controls.float("noise_persistence");

            let angle_variation = self.controls.float("angle_variation");
            let points_per_segment = self.controls.float("points_per_segment");
            let passes = self.controls.float("passes");
            let curvature = self.controls.float("curvature");
            let trig_fn_a = self.controls.string("trig_fn_a");
            let trig_fn_b = self.controls.string("trig_fn_b");

            let wr = ctx.window_rect();

            if self.controls.any_changed_in(&[
                "ref_segments",
                "ref_deviation",
                "ref_smooth",
            ]) {
                let ref_segments = self.controls.float("ref_segments");
                let ref_deviation = self.controls.float("ref_deviation");
                let ref_smooth = self.controls.float("ref_smooth");

                let pad = wr.w() / 32.0;
                let start = vec2(-wr.hw() + pad, 0.0);
                let end = vec2(wr.hw() - pad, 0.0);

                self.ref_line = reference_line(
                    start,
                    end,
                    ref_segments as usize,
                    ref_deviation,
                    ref_smooth as usize,
                );
            }

            let sand_line = SandLine::new(
                match noise_strategy.as_str() {
                    "Octave" => Box::new(OctaveNoise::new(
                        noise_octaves as u32,
                        noise_persistence,
                    )),
                    _ => Box::new(GaussianNoise {}),
                },
                match distribution_strategy.as_str() {
                    "Curved" => Box::new(CurvedDistribution::new(curvature)),
                    "TrigFn" => Box::new(TrigFnDistribution::new(
                        curvature,
                        *trig_fn_lookup().get(trig_fn_a.as_str()).unwrap(),
                        *trig_fn_lookup().get(trig_fn_b.as_str()).unwrap(),
                    )),
                    _ => Box::new(PerpendicularDistribution),
                },
            );

            self.sand_line = sand_line.generate(
                &self.ref_line,
                noise_scale,
                points_per_segment as usize,
                angle_variation,
                passes as usize,
            );

            self.controls.mark_unchanged();
        }
    }

    fn view(&self, app: &App, frame: Frame, ctx: &LatticeContext) {
        let draw = app.draw();
        let wr = ctx.window_rect();

        draw.rect()
            .x_y(0.0, 0.0)
            .w_h(wr.w(), wr.h())
            .hsla(0.0, 0.0, 1.0, 1.0);

        let alpha = self.controls.float("alpha");
        let show_ref_line = self.controls.bool("show_ref_line");
        let show_sand_line = self.controls.bool("show_sand_line");

        if show_sand_line {
            for point in &self.sand_line {
                draw.rect()
                    .xy(*point)
                    .w_h(1.0, 1.0)
                    .color(hsla(0.0, 0.0, 0.0, alpha));
            }
        }

        if show_ref_line {
            draw.polyline()
                .weight(2.0)
                .points(self.ref_line.iter().cloned())
                .color(rgba(0.33, 0.45, 0.9, 1.0));
        }

        draw.to_frame(app, &frame).unwrap();
    }
}

fn reference_line(
    start: Vec2,
    end: Vec2,
    segments: usize,
    deviation: f32,
    smoothing_passes: usize,
) -> Vec<Vec2> {
    let length = end.x - start.x;

    let reference_points: Vec<Vec2> = (0..=segments as u32)
        .map(|i| {
            let t = i as f32 / segments as f32;
            let x = start.x + length * t;
            if i == 0 || i == segments as u32 {
                vec2(x, 0.0)
            } else {
                vec2(x, random_normal(deviation))
            }
        })
        .collect();

    average_neighbors(reference_points, smoothing_passes)
}
