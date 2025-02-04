use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

// https://github.com/inconvergent/sand-spline/blob/master/main-hlines.py

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "sand_lines",
    display_name: "Sand Lines",

    // The sketch absolutely kills the CPU so is only good for static drawings
    play_mode: PlayMode::ManualAdvance,
    fps: 60.0,
    bpm: 134.0,
    w: 1000,
    h: 1000,
    gui_w: None,
    gui_h: Some(790),
};

const N_LINES: usize = 64;

type Line = Vec<Vec2>;

#[derive(SketchComponents)]
pub struct Model {
    controls: Controls,
    wr: WindowRect,
    ref_lines: Vec<Line>,
    sand_lines: Vec<Line>,
}

pub fn init_model(_app: &App, wr: WindowRect) -> Model {
    let disable_octave =
        |controls: &Controls| controls.string("noise_strategy") != "Octave";

    let disable_curve = |controls: &Controls| {
        controls.string("distribution_strategy") != "Curved"
            && controls.string("distribution_strategy") != "TrigFn"
    };

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
        Control::checkbox("chasm_mode", false),
        Control::Separator {},
        Control::select(
            "wave_type",
            "sine",
            &["sine", "triangle", "square", "saw"],
        ),
        Control::slider("wave_freq", 1.0, (0.25, 50.0), 0.25),
        Control::slider("wave_amp", 1.0, (0.0, 1.0), 0.01),
        Control::slider("wave_phase", 0.0, (0.0, TWO_PI), 0.1),
        Control::slider_x("wave_drift", 0.0, (0.0, 1.0), 0.001, |controls| {
            !controls.bool("chasm_mode")
        }),
        Control::Separator {},
        Control::slider("pad", 18.0, (1.0, 32.0), 1.0),
        Control::slider("ref_segments", 16.0, (2.0, 32.0), 1.0),
        Control::slider("ref_deviation", 10.0, (1.0, 100.0), 1.0),
        Control::slider("ref_smooth", 2.0, (0.0, 10.0), 1.0),
        Control::Separator {},
        Control::select(
            "noise_map_mode",
            "linear",
            &[
                "linear",
                "reversed",
                "triangle",
                "triangle_reversed",
                "none",
            ],
        ),
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
        Control::select_x("trig_fn_a", "cos", &trig_fns, |controls| {
            controls.string("distribution_strategy") != "TrigFn"
        }),
        Control::select_x("trig_fn_b", "sin", &trig_fns, |controls| {
            controls.string("distribution_strategy") != "TrigFn"
        }),
        Control::select(
            "angle_map_mode",
            "none",
            &[
                "linear",
                "reversed",
                "triangle",
                "triangle_reversed",
                "none",
            ],
        ),
        Control::slider("angle_variation", 0.5, (0.0, TWO_PI), 0.001),
        Control::slider("points_per_segment", 64.0, (2.0, 128.0), 1.0),
        Control::slider("passes", 50.0, (1.0, 128.0), 1.0),
        Control::select_x(
            "curve_map_mode",
            "none",
            &[
                "linear",
                "reversed",
                "triangle",
                "triangle_reversed",
                "none",
            ],
            disable_curve,
        ),
        Control::slider_x(
            "curvature",
            0.5,
            (0.0, 100.0),
            0.0001,
            disable_curve,
        ),
        Control::slider_x("curve_mult", 1.0, (1.0, 10.0), 1.0, disable_curve),
        Control::checkbox("curve_wtf", false),
        Control::slider_x("curve_exp", 1.0, (0.1, 11.0), 0.1, disable_curve),
        Control::Separator {},
        Control::slider_norm("alpha", 0.5),
    ]);

    Model {
        controls,
        wr,
        ref_lines: vec![],
        sand_lines: vec![],
    }
}

pub fn update(_app: &App, m: &mut Model, _update: Update) {
    if m.controls.changed() {
        let noise_strategy = m.controls.string("noise_strategy");
        let distribution_strategy = m.controls.string("distribution_strategy");
        let chasm_mode = m.controls.bool("chasm_mode");
        let wave_type = m.controls.string("wave_type");
        let wave_freq = m.controls.float("wave_freq");
        let wave_amp = m.controls.float("wave_amp");
        let wave_phase = m.controls.float("wave_phase");
        let wave_drift = m.controls.float("wave_drift");

        let noise_map_mode = m.controls.string("noise_map_mode");
        let noise_scale = m.controls.float("noise_scale");
        let (ns_min, _ns_max) = m.controls.slider_range("noise_scale");
        let noise_octaves = m.controls.float("noise_octaves");
        let noise_persistence = m.controls.float("noise_persistence");

        let angle_map_mode = m.controls.string("angle_map_mode");
        let (angle_min, _angle_max) =
            m.controls.slider_range("angle_variation");
        let angle_variation = m.controls.float("angle_variation");
        let points_per_segment = m.controls.float("points_per_segment");
        let passes = m.controls.float("passes");
        let curve_map_mode = m.controls.string("curve_map_mode");
        let (curve_min, _curve_max) = m.controls.slider_range("curvature");
        let curvature = m.controls.float("curvature");
        let curve_mult = m.controls.float("curve_mult");
        let curve_wtf = m.controls.bool("curve_wtf");
        let curve_exp = m.controls.float("curve_exp");
        let curvature = curvature * curve_mult;
        let trig_fn_a = m.controls.string("trig_fn_a");
        let trig_fn_b = m.controls.string("trig_fn_b");

        if m.controls.any_changed_in(&[
            "chasm_mode",
            "wave_type",
            "wave_freq",
            "wave_amp",
            "wave_phase",
            "wave_drift",
            "pad",
            "ref_segments",
            "ref_deviation",
            "ref_smooth",
        ]) {
            let ref_segments = m.controls.float("ref_segments");
            let ref_deviation = m.controls.float("ref_deviation");
            let ref_smooth = m.controls.float("ref_smooth");
            let pad = m.wr.w_(m.controls.float("pad"));

            m.ref_lines = (0..N_LINES)
                .map(|i| {
                    let t = i as f32 / (N_LINES - 1) as f32;

                    let base_scale = match wave_type.as_str() {
                        "sine" => {
                            let angle = t * TWO_PI * wave_freq;
                            let shifted_angle = angle + wave_phase;
                            let raw_sine = shifted_angle.sin();
                            let normalized_sine = raw_sine * 0.5 + 0.5;
                            normalized_sine
                        }
                        "triangle" => {
                            let angle = (t * wave_freq + wave_phase) % 1.0;
                            if angle < 0.5 {
                                angle * 2.0
                            } else {
                                2.0 - (angle * 2.0)
                            }
                        }
                        "square" => {
                            let angle = (t * TWO_PI * wave_freq) + wave_phase;
                            if (angle % TWO_PI) < PI {
                                1.0
                            } else {
                                0.0
                            }
                        }
                        "saw" => (t * wave_freq + wave_phase) % 1.0,
                        _ => 0.5,
                    };

                    if chasm_mode {
                        let scale = lerp(1.0, base_scale, wave_amp);
                        let gap_size = scale * m.wr.hw();

                        let full_start = vec2(-m.wr.hw() + pad, 0.0);
                        let full_end = vec2(m.wr.hw() - pad, 0.0);

                        let offset =
                            ((t * TWO_PI * (wave_freq * PHI_F32) * 0.5)
                                + wave_phase)
                                .sin()
                                * wave_drift;

                        let left_gap_point =
                            vec2(-gap_size / 2.0 + (offset * m.wr.hw()), 0.0);
                        let right_gap_point =
                            vec2(gap_size / 2.0 + (offset * m.wr.hw()), 0.0);

                        let left_segment = reference_line(
                            full_start,
                            left_gap_point,
                            ref_segments as usize / 2,
                            ref_deviation,
                            ref_smooth as usize,
                        );

                        let right_segment = reference_line(
                            right_gap_point,
                            full_end,
                            ref_segments as usize / 2,
                            ref_deviation,
                            ref_smooth as usize,
                        );

                        [left_segment, right_segment].concat()
                    } else {
                        let scale = lerp(1.0, base_scale, wave_amp);

                        let start = vec2(
                            -m.wr.hw() + pad + (1.0 - scale) * m.wr.hw(),
                            0.0,
                        );
                        let end = vec2(
                            m.wr.hw() - pad - (1.0 - scale) * m.wr.hw(),
                            0.0,
                        );

                        reference_line(
                            start,
                            end,
                            ref_segments as usize,
                            ref_deviation,
                            ref_smooth as usize,
                        )
                    }
                })
                .collect::<Vec<Line>>();
        }

        let (ns_min, ns_max) = safe_range(ns_min, noise_scale);
        let (angle_min, angle_max) = safe_range(angle_min, angle_variation);
        let (curve_min, curve_max) = safe_range(curve_min, curvature);

        m.sand_lines = (0..N_LINES)
            .map(|i| {
                let index = i;
                let i = index as f32;
                let curve_i = exponential(
                    if curve_wtf { i / N_LINES as f32 } else { i },
                    curve_exp,
                );

                let ns = match noise_map_mode.as_str() {
                    "linear" => {
                        map_range(i, 0.0, N_LINES as f32, ns_min, ns_max)
                    }
                    "reversed" => {
                        map_range(i, 0.0, N_LINES as f32, ns_max, ns_min)
                    }
                    "triangle" => triangle_map(
                        i,
                        0.0,
                        N_LINES as f32 - 1.0,
                        ns_min,
                        ns_max,
                    ),
                    "triangle_reversed" => triangle_map(
                        i,
                        0.0,
                        N_LINES as f32 - 1.0,
                        ns_max,
                        ns_min,
                    ),
                    _ => noise_scale,
                };

                let angle_v = match angle_map_mode.as_str() {
                    "linear" => {
                        map_range(i, 0.0, N_LINES as f32, angle_min, angle_max)
                    }
                    "reversed" => {
                        map_range(i, 0.0, N_LINES as f32, angle_max, angle_min)
                    }
                    "triangle" => triangle_map(
                        i,
                        0.0,
                        N_LINES as f32 - 1.0,
                        angle_min,
                        angle_max,
                    ),
                    "triangle_reversed" => triangle_map(
                        i,
                        0.0,
                        N_LINES as f32 - 1.0,
                        angle_max,
                        angle_min,
                    ),
                    _ => angle_variation,
                };

                let curve = match curve_map_mode.as_str() {
                    "linear" => map_range(
                        curve_i,
                        0.0,
                        N_LINES as f32,
                        curve_min,
                        curve_max,
                    ),
                    "reversed" => map_range(
                        curve_i,
                        0.0,
                        N_LINES as f32,
                        curve_max,
                        curve_min,
                    ),
                    "triangle" => triangle_map(
                        curve_i,
                        0.0,
                        N_LINES as f32 - 1.0,
                        curve_min,
                        curve_max,
                    ),
                    "triangle_reversed" => triangle_map(
                        curve_i,
                        0.0,
                        N_LINES as f32 - 1.0,
                        curve_max,
                        curve_min,
                    ),
                    _ => curvature,
                };

                let sand_line = sand_line::SandLine::new(
                    match noise_strategy.as_str() {
                        "Octave" => Box::new(sand_line::OctaveNoise::new(
                            noise_octaves as u32,
                            noise_persistence,
                        )),
                        _ => Box::new(sand_line::GaussianNoise {}),
                    },
                    match distribution_strategy.as_str() {
                        "Curved" => {
                            Box::new(sand_line::CurvedDistribution::new(curve))
                        }
                        "TrigFn" => {
                            Box::new(sand_line::TrigFnDistribution::new(
                                curve,
                                *trig_fn_lookup()
                                    .get(trig_fn_a.as_str())
                                    .unwrap(),
                                *trig_fn_lookup()
                                    .get(trig_fn_b.as_str())
                                    .unwrap(),
                            ))
                        }
                        _ => Box::new(sand_line::PerpendicularDistribution),
                    },
                );

                sand_line.generate(
                    &m.ref_lines[index],
                    ns,
                    points_per_segment as usize,
                    angle_v,
                    passes as usize,
                )
            })
            .collect::<Vec<Line>>();

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

    let pad = m.wr.h_(48.0);
    let space = (m.wr.h() - (pad * 2.0)) / (N_LINES as f32 - 1.0);
    let y_off = m.wr.hh() - pad;

    for i in 0..N_LINES {
        let y = y_off - (space * i as f32);
        let draw = draw.translate(vec3(0.0, y, 0.0));

        if show_ref_line {
            draw.polyline()
                .weight(2.0)
                .points(m.ref_lines[i].iter().cloned())
                .color(rgba(0.33, 0.45, 0.9, 1.0));
        }

        if show_sand_line {
            for point in &m.sand_lines[i] {
                draw.rect()
                    .xy(*point)
                    .w_h(1.0, 1.0)
                    .color(hsla(0.0, 0.0, 0.0, alpha));
            }
        }
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
