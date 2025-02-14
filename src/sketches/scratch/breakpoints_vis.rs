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

// Points array will length N_POINTS + 1
// to better visualize the loop
const N_POINTS: usize = 32;
const TOTAL_BEATS: f32 = 2.0;

#[derive(SketchComponents)]
pub struct Model {
    wr: WindowRect,
    points: Vec<(f32, f32)>,
}

pub fn init_model(_app: &App, wr: WindowRect) -> Model {
    let mut animation = Animation::new(ManualTiming::new(SKETCH_CONFIG.bpm));

    let mut points = vec![];

    let beat_step = TOTAL_BEATS / N_POINTS as f32;

    // We use =N_POINTS so there is 1 extra point to complete the loop
    for i in 0..=N_POINTS {
        let position = i as f32 * beat_step;
        animation.timing.set_beats(position);
        let value = animation.animate(
            &[
                Breakpoint::ramp(0.0, 0.0, Easing::Linear),
                Breakpoint::ramp(TOTAL_BEATS / 2.0, 1.0, Easing::Linear),
                Breakpoint::end(TOTAL_BEATS, 0.0),
            ],
            Mode::Once,
        );
        points.push((position, value));
    }

    Model { wr, points }
}

pub fn update(_app: &App, _m: &mut Model, _update: Update) {}

pub fn view(app: &App, m: &Model, frame: Frame) {
    let draw = app.draw();

    debug_once(format!("points: {:?}", m.points));

    // Background
    draw.rect()
        .x_y(0.0, 0.0)
        .w_h(m.wr.w(), m.wr.h())
        .color(gray(0.04));

    let track_height = 200.0;

    // Track background
    draw.rect()
        .x_y(0.0, 0.0)
        .w_h(m.wr.w(), track_height)
        .color(gray(0.2));

    for (position, value) in &m.points {
        draw.ellipse()
            .radius(4.0)
            .x_y(
                map_range(*position, 0.0, TOTAL_BEATS, -m.wr.hw(), m.wr.hw()),
                map_range(
                    *value,
                    0.0,
                    1.0,
                    -track_height / 2.0,
                    track_height / 2.0,
                ),
            )
            .color(rgb(1.0, 0.5, 0.0));
    }

    draw.to_frame(app, &frame).unwrap();
}
