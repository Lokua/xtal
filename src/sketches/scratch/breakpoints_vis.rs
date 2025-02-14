use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "breakpoints_vis",
    display_name: "Breakpoints Visualization",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(150),
};

// Points array will length N_POINTS + 1 to better visualize the loop
const N_POINTS: usize = 64;
const TOTAL_BEATS: f32 = 2.0;

type Lane = Vec<(f32, f32)>;

#[derive(SketchComponents)]
pub struct Model {
    wr: WindowRect,
    lanes: Vec<Lane>,
}

pub fn init_model(_app: &App, wr: WindowRect) -> Model {
    let mut animation = Animation::new(ManualTiming::new(SKETCH_CONFIG.bpm));

    let lanes = vec![
        create_ramp_lane(&mut animation),
        create_wave_lane(&mut animation),
        create_step_lane(&mut animation),
    ];

    Model { wr, lanes }
}

pub fn update(_app: &App, _m: &mut Model, _update: Update) {}

pub fn view(app: &App, m: &Model, frame: Frame) {
    let draw = app.draw();

    // Background
    draw.rect()
        .x_y(0.0, 0.0)
        .w_h(m.wr.w(), m.wr.h())
        .color(gray(0.06));

    let track_height = 200.0;
    let track_padding = 20.0;
    let lane_padding = 8.0;
    let vertical_offset = track_height + track_padding;
    let point_color = rgb(1.0, 0.4, 0.1);

    for (lane_index, lane) in m.lanes.iter().enumerate() {
        debug_once(format!("lane: {:?}", lane));

        let y_offset = m.wr.top()
            - (track_height / 2.0)
            - track_padding
            - (lane_index as f32 * vertical_offset);

        // Track background
        draw.rect()
            .x_y(0.0, y_offset)
            .w_h(m.wr.w() - track_padding * 2.0, track_height)
            .color(gray(0.2));

        // Connecting lines
        for points in lane.windows(2) {
            if let [p1, p2] = points {
                let x1 = map_range(
                    p1.0,
                    0.0,
                    TOTAL_BEATS,
                    -m.wr.hw() + track_padding + lane_padding,
                    m.wr.hw() - track_padding - lane_padding,
                );
                let y1 = y_offset
                    + map_range(
                        p1.1,
                        0.0,
                        1.0,
                        -track_height / 2.0 + lane_padding,
                        track_height / 2.0 - lane_padding,
                    );
                let x2 = map_range(
                    p2.0,
                    0.0,
                    TOTAL_BEATS,
                    -m.wr.hw() + track_padding + lane_padding,
                    m.wr.hw() - track_padding - lane_padding,
                );
                let y2 = y_offset
                    + map_range(
                        p2.1,
                        0.0,
                        1.0,
                        -track_height / 2.0 + lane_padding,
                        track_height / 2.0 - lane_padding,
                    );

                draw.line()
                    .start(pt2(x1, y1))
                    .end(pt2(x2, y2))
                    .color(point_color)
                    .weight(1.0);
            }
        }

        // Points
        for (position, value) in lane {
            draw.ellipse()
                .radius(4.0)
                .x_y(
                    map_range(
                        *position,
                        0.0,
                        TOTAL_BEATS,
                        -m.wr.hw() + track_padding + lane_padding,
                        m.wr.hw() - track_padding - lane_padding,
                    ),
                    y_offset
                        + map_range(
                            *value,
                            0.0,
                            1.0,
                            -track_height / 2.0 + lane_padding,
                            track_height / 2.0 - lane_padding,
                        ),
                )
                .color(point_color);
        }
    }

    draw.to_frame(app, &frame).unwrap();
}

fn create_ramp_lane(animation: &mut Animation<ManualTiming>) -> Lane {
    let mut points = vec![];

    let beat_step = TOTAL_BEATS / N_POINTS as f32;

    // We use =N_POINTS so there is 1 extra point to complete the loop
    for i in 0..=N_POINTS {
        let position = i as f32 * beat_step;
        animation.timing.set_beats(position);
        let value = animation.animate(
            &[
                Breakpoint::ramp(0.0, 0.0, Easing::EaseInQuad),
                Breakpoint::ramp(TOTAL_BEATS / 2.0, 1.0, Easing::EaseInQuad),
                Breakpoint::end(TOTAL_BEATS, 0.0),
            ],
            Mode::Once,
        );
        points.push((position, value));
    }

    points
}

fn create_wave_lane(animation: &mut Animation<ManualTiming>) -> Lane {
    let mut points = vec![];

    let beat_step = TOTAL_BEATS / N_POINTS as f32;
    let frequency = 0.125;
    let amplitude = 0.125;

    // We use =N_POINTS so there is 1 extra point to complete the loop
    for i in 0..=N_POINTS {
        let position = i as f32 * beat_step;
        animation.timing.set_beats(position);
        let value = animation.animate(
            &[
                Breakpoint::wave(
                    0.0,
                    0.0,
                    Shape::Triangle,
                    frequency,
                    amplitude,
                ),
                Breakpoint::wave(
                    TOTAL_BEATS / 2.0,
                    1.0,
                    Shape::Triangle,
                    frequency,
                    amplitude,
                ),
                Breakpoint::end(TOTAL_BEATS, 0.0),
            ],
            Mode::Once,
        );
        let value = clamp(value, 0.0, 1.0);
        points.push((position, value));
    }

    points
}

fn create_step_lane(_animation: &mut Animation<ManualTiming>) -> Lane {
    let mut points = vec![];
    let breakpoints = [
        Breakpoint::step(0.0, 0.0),
        Breakpoint::step(TOTAL_BEATS / 4.0, 0.5),
        Breakpoint::step(TOTAL_BEATS / 2.0, 1.0),
        Breakpoint::end(TOTAL_BEATS, 0.0),
    ];

    // For each breakpoint (except the last), create points for the horizontal line
    // and the vertical transition to the next value
    for i in 0..breakpoints.len() - 1 {
        let current = &breakpoints[i];
        let next = &breakpoints[i + 1];

        // Start of current step
        points.push((current.position, current.value));
        // End of current step (just before transition)
        points.push((next.position, current.value));
        // Start of next step (after vertical transition)
        points.push((next.position, next.value));
    }

    points
}
