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
    h: 700,
    gui_w: None,
    gui_h: Some(150),
};

const TOTAL_BEATS: f32 = 2.0;
const CURVE_RESOLUTION: usize = 128;
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
    wr: WindowRect,
    lanes: Vec<Vec<Breakpoint>>,
    segments: Vec<Vec<LaneSegment>>,
}

pub fn init_model(_app: &App, wr: WindowRect) -> Model {
    let mut animation = Animation::new(ManualTiming::new(SKETCH_CONFIG.bpm));

    let lanes = vec![
        create_ramp_lane(),
        create_wave_lane(),
        create_step_lane(),
        kitchen_sink(),
    ];

    let segments = compute_all_segments(&lanes, &mut animation, &wr);

    Model {
        wr,
        lanes,
        segments,
    }
}

fn compute_all_segments(
    lanes: &[Vec<Breakpoint>],
    animation: &mut Animation<ManualTiming>,
    wr: &WindowRect,
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

            create_lane_segments(breakpoints, animation, y_offset, wr.hw())
        })
        .collect()
}

pub fn update(_app: &App, _m: &mut Model, _update: Update) {}

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

fn create_lane_segments(
    breakpoints: &[Breakpoint],
    animation: &mut Animation<ManualTiming>,
    y_offset: f32,
    half_width: f32,
) -> Vec<LaneSegment> {
    let mut segments = Vec::new();

    for window in breakpoints.windows(2) {
        if let [current, next] = window {
            let segment_duration = next.position - current.position;
            let step = segment_duration / CURVE_RESOLUTION as f32;

            let points = match current.kind {
                Transition::Step => {
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
                        // let current_pos = current.position + (i as f32 * step);
                        let current_pos =
                            current.position + (i as f32 * step) + (step / 2.0);
                        animation.timing.set_beats(current_pos);
                        let value = animation.animate(breakpoints, Mode::Once);

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
                is_step: matches!(current.kind, Transition::Step),
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
            value.clamp(0.0, 1.0),
            0.0,
            1.0,
            -TRACK_HEIGHT / 2.0 + LANE_PADDING,
            TRACK_HEIGHT / 2.0 - LANE_PADDING,
        );
    pt2(x, y)
}

fn create_ramp_lane() -> Vec<Breakpoint> {
    vec![
        Breakpoint::ramp(0.0, 0.0, Easing::EaseInQuint),
        Breakpoint::ramp(TOTAL_BEATS / 2.0, 1.0, Easing::EaseOutBounce),
        Breakpoint::end(TOTAL_BEATS, 0.0),
    ]
}

fn create_wave_lane() -> Vec<Breakpoint> {
    let frequency = 0.125;
    let amplitude = 0.125;
    vec![
        Breakpoint::wave(0.0, 0.0, Shape::Triangle, frequency, amplitude),
        Breakpoint::wave(
            TOTAL_BEATS / 2.0,
            1.0,
            Shape::Triangle,
            frequency,
            amplitude,
        ),
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

fn kitchen_sink() -> Vec<Breakpoint> {
    vec![
        Breakpoint::step(0.0, 0.0),
        Breakpoint::ramp(0.25, 0.0, Easing::Linear),
        Breakpoint::step(0.5, 1.0),
        Breakpoint::ramp(1.0, 0.5, Easing::EaseIn),
        Breakpoint::wave(1.5, 1.0, Shape::Triangle, 0.125, 0.125),
        Breakpoint::end(2.0, 0.0),
    ]
}
