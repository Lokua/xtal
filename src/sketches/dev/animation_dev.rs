use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "animation_dev",
    display_name: "Animation Test",
    fps: 60.0,
    bpm: 134.0,
    // fps: 24.0,
    // bpm: 360.0,
    w: 500,
    h: 500,
    gui_w: None,
    gui_h: Some(150),
    play_mode: PlayMode::Loop,
};

#[derive(SketchComponents)]
pub struct Model {
    animation: Animation<Timing>,
    lerp: f32,
    ramp: f32,
    r_ramp: f32,
}

pub fn init_model(_app: &App, _window_rect: WindowRect) -> Model {
    let animation = Animation::new(Timing::new(SKETCH_CONFIG.bpm));

    Model {
        animation,
        lerp: 0.0,
        ramp: 0.0,
        r_ramp: 0.0,
    }
}

pub fn update(_app: &App, model: &mut Model, _update: Update) {
    model.lerp = model
        .animation
        .lerp(&[kf(0.0, 2.0), kf(1.0, 2.0), kf(0.0, KF::END)], 0.0);

    model.ramp =
        model
            .animation
            .ramp(&[kf(0.0, 4.0), kf(1.0, 4.0)], 0.0, 1.0, linear);

    model.r_ramp =
        model
            .animation
            .r_ramp(&[kfr((0.0, 1.0), 4.0)], 0.0, 1.0, linear);
}

pub fn view(app: &App, model: &Model, frame: Frame) {
    let window_rect = app
        .window(frame.window_id())
        .expect("Unable to get window")
        .rect();

    let draw = app.draw();

    draw.rect()
        .x_y(0.0, 0.0)
        .w_h(window_rect.w(), window_rect.h())
        .color(BEIGE);

    let hw = window_rect.w() / 2.0;
    let hh = window_rect.h() / 2.0;
    let radius = hh / 5.0;
    let edge = hw - radius;
    let component_value = PHI_F32 - 1.0;

    draw.ellipse()
        .x_y(map_range(model.lerp, 0.0, 1.0, -edge, edge), hh / 2.0)
        .radius(radius)
        .color(rgb(component_value, 0.0, 0.0));

    draw.ellipse()
        .x_y(map_range(model.ramp, 0.0, 1.0, -edge, edge), 0.0)
        .radius(radius)
        .color(rgb(0.0, component_value, 0.0));

    draw.ellipse()
        .x_y(map_range(model.r_ramp, 0.0, 1.0, -edge, edge), -hh / 2.0)
        .radius(radius)
        .color(rgb(0.0, 0.0, component_value));

    draw.to_frame(app, &frame).unwrap();
}
