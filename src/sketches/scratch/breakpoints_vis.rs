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
const N_POINTS: usize = 32;
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
                .color(rgb(1.0, 0.5, 0.0));
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

    // We use =N_POINTS so there is 1 extra point to complete the loop
    for i in 0..=N_POINTS {
        let position = i as f32 * beat_step;
        animation.timing.set_beats(position);
        let value = animation.animate(
            &[
                Breakpoint::wave(0.0, 0.0, Shape::Triangle, 0.25, 0.25),
                Breakpoint::wave(
                    TOTAL_BEATS / 2.0,
                    1.0,
                    Shape::Triangle,
                    0.25,
                    0.25,
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
