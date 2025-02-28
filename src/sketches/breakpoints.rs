use std::str::FromStr;

use crate::framework::prelude::*;
use nannou::color::*;
use nannou::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "breakpoints",
    display_name: "Breakpoints Visualization",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 1244,
    gui_w: None,
    gui_h: Some(400),
};

const TOTAL_BEATS: f32 = 2.0;
const CURVE_RESOLUTION: usize = 512;
const TRACK_HEIGHT: f32 = 150.0;
const TRACK_PADDING: f32 = 20.0;
const LANE_PADDING: f32 = 8.0;
const POINT_RADIUS: f32 = 4.0;
const POINT_COLOR: Rgb<f32> = Rgb {
    red: 1.0,
    green: 0.2,
    blue: 0.4,
    standard: std::marker::PhantomData,
};

struct LaneSegment {
    points: Vec<Point2>,
    is_step: bool,
}

#[derive(SketchComponents)]
pub struct Model {
    animation: Animation<ManualTiming>,
    controls: Controls,
    lanes: Vec<Vec<Breakpoint>>,
    segments: Vec<Vec<LaneSegment>>,
    slew_limiter: SlewLimiter,
    wr: WindowRect,
}

pub fn init_model(_app: &App, wr: WindowRect) -> Model {
    let animation = Animation::new(ManualTiming::new(SKETCH_CONFIG.bpm));

    let controls = Controls::with_previous(vec![
        Control::select("easing", "linear", &Easing::unary_function_names()),
        Control::select(
            "wave_easing",
            "linear",
            &Easing::unary_function_names(),
        ),
        Control::select("wave_shape", "sine", &["sine", "triangle", "square"]),
        Control::slider("wave_amplitude", 0.125, (-1.0, 1.0), 0.001),
        Control::slider("wave_frequency", 0.25, (0.0, 1.0), 0.0125),
        Control::slider("wave_width", 0.5, (0.0, 1.0), 0.01),
        Control::select(
            "wave_clamp_method",
            "clamp",
            &["none", "clamp", "fold", "wrap"],
        ),
        Control::Separator {},
        Control::slide("slew_rise", 0.0),
        Control::slide("slew_fall", 0.0),
    ]);

    Model {
        animation,
        controls,
        lanes: vec![],
        segments: vec![],
        wr,
        slew_limiter: SlewLimiter::default(),
    }
}

pub fn update(_app: &App, m: &mut Model, _update: Update) {
    if m.controls.changed() {
        let easing = Easing::from_str(&m.controls.string("easing")).unwrap();
        let wave_easing =
            Easing::from_str(&m.controls.string("wave_easing")).unwrap();
        let width = m.controls.float("wave_width");
        let shape = Shape::from_str(&m.controls.string("wave_shape")).unwrap();
        let clamp_method = m.controls.string("wave_clamp_method");
        let constrain =
            Constrain::try_from((clamp_method.as_str(), 0.0, 1.0)).unwrap();
        let amplitude = m.controls.float("wave_amplitude");
        let frequency = m.controls.float("wave_frequency");
        let rise = m.controls.float("slew_rise");
        let fall = m.controls.float("slew_fall");

        m.slew_limiter.set_rates(rise, fall);

        let lanes = vec![
            create_ramp_lane(easing.clone()),
            create_step_lane(),
            create_wave_lane(
                shape.clone(),
                wave_easing.clone(),
                width,
                constrain.clone(),
                amplitude,
                frequency,
            ),
            create_random_lane(
                wave_easing.clone(),
                constrain.clone(),
                amplitude,
                frequency,
            ),
            kitchen_sink(
                easing,
                shape,
                wave_easing,
                width,
                constrain,
                amplitude,
                frequency,
            ),
        ];

        m.segments = create_segments(
            &lanes,
            &mut m.animation,
            &m.wr,
            &mut m.slew_limiter,
        );
        m.lanes = lanes;

        m.controls.mark_unchanged();
    }
}

pub fn view(app: &App, m: &Model, frame: Frame) {
    let draw = app.draw();

    draw.rect()
        .x_y(0.0, 0.0)
        .w_h(m.wr.w(), m.wr.h())
        .color(gray(0.06));

    let vertical_offset = TRACK_HEIGHT + TRACK_PADDING;

    for (lane_index, (breakpoints, segments)) in
        m.lanes.iter().zip(m.segments.iter()).enumerate()
    {
        let y_offset = m.wr.top()
            - (TRACK_HEIGHT / 2.0)
            - TRACK_PADDING
            - (lane_index as f32 * vertical_offset);

        draw.rect()
            .x_y(0.0, y_offset)
            .w_h(m.wr.w() - TRACK_PADDING * 2.0, TRACK_HEIGHT)
            .color(gray(0.2));

        for segment in segments {
            if segment.is_step {
                for points in segment.points.windows(2) {
                    if let [p1, p2] = points {
                        draw.line()
                            .start(*p1)
                            .end(*p2)
                            .color(POINT_COLOR)
                            .weight(1.0);
                    }
                }
            } else {
                draw.polyline()
                    .weight(1.0)
                    .points(segment.points.iter().cloned())
                    .color(POINT_COLOR);
            }
        }

        for breakpoint in breakpoints {
            let point = map_to_track(
                breakpoint.position,
                breakpoint.value,
                y_offset,
                m.wr.hw(),
            );

            draw.ellipse()
                .radius(POINT_RADIUS)
                .xy(point)
                .color(POINT_COLOR);
        }
    }

    draw.to_frame(app, &frame).unwrap();
}

fn create_segments(
    lanes: &[Vec<Breakpoint>],
    animation: &mut Animation<ManualTiming>,
    wr: &WindowRect,
    mut slew_limiter: &mut SlewLimiter,
) -> Vec<Vec<LaneSegment>> {
    let vertical_offset = TRACK_HEIGHT + TRACK_PADDING;

    lanes
        .iter()
        .enumerate()
        .map(|(lane_index, breakpoints)| {
            let y_offset = wr.top()
                - (TRACK_HEIGHT / 2.0)
                - TRACK_PADDING
                - (lane_index as f32 * vertical_offset);

            create_lanes(
                breakpoints,
                animation,
                y_offset,
                wr.hw(),
                &mut slew_limiter,
            )
        })
        .collect()
}

fn create_lanes(
    breakpoints: &[Breakpoint],
    animation: &mut Animation<ManualTiming>,
    y_offset: f32,
    half_width: f32,
    slew_limiter: &mut SlewLimiter,
) -> Vec<LaneSegment> {
    let mut segments = Vec::new();

    for window in breakpoints.windows(2) {
        if let [current, next] = window {
            let segment_duration = next.position - current.position;
            let step = segment_duration / CURVE_RESOLUTION as f32;

            let points = match current.kind {
                Kind::Step => {
                    let start = map_to_track(
                        current.position,
                        current.value,
                        y_offset,
                        half_width,
                    );
                    let end = map_to_track(
                        next.position,
                        next.value,
                        y_offset,
                        half_width,
                    );

                    let mid = pt2(end.x, start.y);
                    vec![start, mid, end]
                }
                _ => {
                    let mut points = Vec::new();

                    for i in 0..=CURVE_RESOLUTION {
                        let current_pos =
                            current.position + (i as f32 * step) + (step / 2.0);
                        animation.timing.set_beats(current_pos);
                        let value = animation.automate(breakpoints, Mode::Once);
                        let value = slew_limiter.slew(value);

                        points.push(map_to_track(
                            current_pos,
                            value,
                            y_offset,
                            half_width,
                        ));
                    }
                    points
                }
            };

            segments.push(LaneSegment {
                points,
                is_step: matches!(current.kind, Kind::Step),
            });
        }
    }

    segments
}

fn map_to_track(
    position: f32,
    value: f32,
    track_offset: f32,
    half_width: f32,
) -> Point2 {
    let track_half_width = half_width - TRACK_PADDING;
    let x = map_range(
        position,
        0.0,
        TOTAL_BEATS,
        -track_half_width + LANE_PADDING,
        track_half_width - LANE_PADDING,
    );
    let y = track_offset
        + map_range(
            value,
            0.0,
            1.0,
            -TRACK_HEIGHT / 2.0 + LANE_PADDING,
            TRACK_HEIGHT / 2.0 - LANE_PADDING,
        );
    pt2(x, y)
}

fn create_ramp_lane(easing: Easing) -> Vec<Breakpoint> {
    vec![
        Breakpoint::ramp(0.0, 0.0, easing.clone()),
        Breakpoint::ramp(TOTAL_BEATS / 2.0, 1.0, easing),
        Breakpoint::end(TOTAL_BEATS, 0.0),
    ]
}

fn create_step_lane() -> Vec<Breakpoint> {
    vec![
        Breakpoint::step(0.0, 0.0),
        Breakpoint::step(TOTAL_BEATS / 4.0, 0.5),
        Breakpoint::step(TOTAL_BEATS / 2.0, 1.0),
        Breakpoint::end(TOTAL_BEATS, 0.0),
    ]
}

fn create_wave_lane(
    shape: Shape,
    easing: Easing,
    width: f32,
    constrain: Constrain,
    amplitude: f32,
    frequency: f32,
) -> Vec<Breakpoint> {
    vec![
        Breakpoint::wave(
            0.0,
            0.0,
            shape.clone(),
            frequency,
            width,
            amplitude,
            easing.clone(),
            constrain.clone(),
        ),
        Breakpoint::wave(
            TOTAL_BEATS / 2.0,
            1.0,
            shape,
            frequency,
            width,
            amplitude,
            easing,
            constrain,
        ),
        Breakpoint::end(TOTAL_BEATS, 0.0),
    ]
}

fn create_random_lane(
    easing: Easing,
    constrain: Constrain,
    amplitude: f32,
    frequency: f32,
) -> Vec<Breakpoint> {
    vec![
        Breakpoint::random_smooth(
            0.0,
            0.0,
            frequency,
            amplitude,
            easing.clone(),
            constrain.clone(),
        ),
        Breakpoint::random_smooth(
            TOTAL_BEATS / 2.0,
            1.0,
            frequency,
            amplitude,
            easing.clone(),
            constrain.clone(),
        ),
        Breakpoint::end(TOTAL_BEATS, 0.0),
    ]
}

fn kitchen_sink(
    easing: Easing,
    shape: Shape,
    wave_easing: Easing,
    width: f32,
    constrain: Constrain,
    amplitude: f32,
    frequency: f32,
) -> Vec<Breakpoint> {
    vec![
        Breakpoint::step(0.0, 0.0),
        Breakpoint::ramp(0.25, 0.0, easing.clone()),
        Breakpoint::step(0.5, 1.0),
        Breakpoint::random_smooth(
            0.75,
            0.0,
            frequency,
            amplitude,
            easing.clone(),
            constrain.clone(),
        ),
        Breakpoint::ramp(1.0, 0.5, easing),
        Breakpoint::wave(
            1.5,
            1.0,
            shape,
            frequency,
            width,
            amplitude,
            wave_easing,
            constrain,
        ),
        Breakpoint::end(TOTAL_BEATS, 0.0),
    ]
}
