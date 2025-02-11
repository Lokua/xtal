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
pub struct Model {
    controls: Controls,
    wr: WindowRect,
    ref_line: Vec<Vec2>,
    sand_line: Vec<Vec2>,
}

pub fn init_model(_app: &App, wr: WindowRect) -> Model {
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
        Control::slider_norm("alpha", 0.5),
    ]);

    Model {
        controls,
        wr,
        ref_line: vec![],
        sand_line: vec![],
    }
}

pub fn update(_app: &App, m: &mut Model, _update: Update) {
    if m.controls.changed() {
        let noise_strategy = m.controls.string("noise_strategy");
        let distribution_strategy = m.controls.string("distribution_strategy");

        let noise_scale = m.controls.float("noise_scale");
        let noise_octaves = m.controls.float("noise_octaves");
        let noise_persistence = m.controls.float("noise_persistence");

        let angle_variation = m.controls.float("angle_variation");
        let points_per_segment = m.controls.float("points_per_segment");
        let passes = m.controls.float("passes");
        let curvature = m.controls.float("curvature");
        let trig_fn_a = m.controls.string("trig_fn_a");
        let trig_fn_b = m.controls.string("trig_fn_b");

        if m.controls.any_changed_in(&[
            "ref_segments",
            "ref_deviation",
            "ref_smooth",
        ]) {
            let ref_segments = m.controls.float("ref_segments");
            let ref_deviation = m.controls.float("ref_deviation");
            let ref_smooth = m.controls.float("ref_smooth");

            let pad = m.wr.w_(32.0);
            let start = vec2(-m.wr.hw() + pad, 0.0);
            let end = vec2(m.wr.hw() - pad, 0.0);

            m.ref_line = reference_line(
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

        m.sand_line = sand_line.generate(
            &m.ref_line,
            noise_scale,
            points_per_segment as usize,
            angle_variation,
            passes as usize,
        );

        m.controls.mark_unchanged();
    }
}

pub fn view(app: &App, m: &Model, frame: Frame) {
    let draw = app.draw();

    draw.rect()
        .x_y(0.0, 0.0)
        .w_h(m.wr.w(), m.wr.h())
        .hsla(0.0, 0.0, 1.0, 1.0);

    let alpha = m.controls.float("alpha");
    let show_ref_line = m.controls.bool("show_ref_line");
    let show_sand_line = m.controls.bool("show_sand_line");

    if show_sand_line {
        for point in &m.sand_line {
            draw.rect()
                .xy(*point)
                .w_h(1.0, 1.0)
                .color(hsla(0.0, 0.0, 0.0, alpha));
        }
    }

    if show_ref_line {
        draw.polyline()
            .weight(2.0)
            .points(m.ref_line.iter().cloned())
            .color(rgba(0.33, 0.45, 0.9, 1.0));
    }

    draw.to_frame(app, &frame).unwrap();
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
