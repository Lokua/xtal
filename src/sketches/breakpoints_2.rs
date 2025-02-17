use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "breakpoints_2",
    display_name: "Breakpoints 2",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(300),
};

#[derive(SketchComponents)]
pub struct Model {
    animation: Animation<ManualTiming>,
    controls: ControlScript<ManualTiming>,
    wr: WindowRect,
    breakpoints: Vec<Breakpoint>,
    points: Vec<[f32; 2]>,
    slew_limiter: SlewLimiter,
}

pub fn init_model(_app: &App, wr: WindowRect) -> Model {
    let timing = ManualTiming::new(SKETCH_CONFIG.bpm);
    let animation = Animation::new(timing.clone());
    let controls = ControlScript::new(
        to_absolute_path(file!(), "breakpoints_2.yaml"),
        timing,
    );

    let slew_limiter = SlewLimiter::default();

    Model {
        animation,
        controls,
        wr,
        breakpoints: vec![],
        points: vec![],
        slew_limiter,
    }
}

pub fn update(_app: &App, m: &mut Model, _update: Update) {
    m.controls.update();

    if m.controls.changed() {
        debug!("changed");
        m.breakpoints = m.controls.breakpoints("points");
        let slew = m.controls.bool("slew");
        let rise = m.controls.get("rise");
        let fall = m.controls.get("fall");

        m.slew_limiter.set_rates(rise, fall);

        m.points = create_points(
            &mut m.animation,
            &m.breakpoints,
            1024 * 2,
            ternary!(slew, Some(&mut m.slew_limiter), None),
        );

        m.controls.mark_unchanged();
    }
}

pub fn view(app: &App, m: &Model, frame: Frame) {
    let draw = app.draw();

    draw.rect()
        .x_y(0.0, 0.0)
        .w_h(m.wr.w(), m.wr.h())
        .color(gray(0.1));

    let track_height = m.wr.h() / 4.0;
    let track_h_padding = 18.0;
    let track_v_padding = 6.0;

    draw.rect()
        .x_y(0.0, 0.0)
        .w_h(m.wr.w() - track_h_padding, track_height)
        .color(gray(0.2));

    for point in &m.points {
        draw.ellipse()
            .x_y(
                map_range(
                    point[0],
                    0.0,
                    m.breakpoints.last().unwrap().position,
                    -m.wr.hw() + track_h_padding,
                    m.wr.hw() - track_h_padding,
                ),
                map_range(
                    point[1],
                    0.0,
                    1.0,
                    -(track_height / 2.0) + track_v_padding,
                    (track_height / 2.0) - track_v_padding,
                ),
            )
            .radius(1.0)
            .color(PALETURQUOISE);
    }

    draw.to_frame(app, &frame).unwrap();
}

fn create_points(
    animation: &mut Animation<ManualTiming>,
    breakpoints: &[Breakpoint],
    n_points: usize,
    mut slew_limiter: Option<&mut SlewLimiter>,
) -> Vec<[f32; 2]> {
    let mut points: Vec<[f32; 2]> = vec![];
    let total_beats = breakpoints.last().unwrap().position;
    let step = total_beats / n_points as f32;
    for t in 0..n_points {
        animation.timing.set_beats(t as f32 * step);
        let anim = animation.automate(breakpoints, Mode::Once);
        let processed = post_pipeline(anim, &mut slew_limiter);
        points.push([animation.beats(), processed]);
    }
    points
}

fn post_pipeline(
    value: f32,
    slew_limiter: &mut Option<&mut SlewLimiter>,
) -> f32 {
    if let Some(slew) = slew_limiter {
        return slew.slew(value);
    }
    value
}
