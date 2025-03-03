use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "lin_alg",
    display_name: "Lin Alg",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(150),
};

#[derive(LegacySketchComponents)]
pub struct Model {
    #[allow(dead_code)]
    animation: Animation<FrameTiming>,
    controls: Controls,
    wr: WindowRect,
    vectors: Vec<Vec2>,
}

pub fn init_model(_app: &App, wr: WindowRect) -> Model {
    let animation =
        Animation::new(FrameTiming::new(Bpm::new(SKETCH_CONFIG.bpm)));

    let controls = Controls::new(vec![]);

    let a = vec2(1.0, 2.0);
    let b = vec2(3.0, -1.0);
    // [1 + 3 = 4,, 2 + -1 = 1] = [4.0, 1.0]
    let c = a + b;

    Model {
        animation,
        controls,
        wr,
        vectors: vec![a, b, c],
    }
}

pub fn update(_app: &App, _m: &mut Model, _update: Update) {}

pub fn view(app: &App, m: &Model, frame: Frame) {
    let draw = app.draw();

    draw.rect()
        .x_y(0.0, 0.0)
        .w_h(m.wr.w(), m.wr.h())
        .hsla(0.0, 0.0, 0.02, 0.1);

    let origin = vec2(0.0, 0.0);
    let unit_mult = 40.0;

    draw.line()
        .start(origin)
        .end(m.vectors[0] * unit_mult)
        .stroke_weight(2.0)
        .color(YELLOW);

    draw.line()
        .start(origin)
        .end(m.vectors[1] * unit_mult)
        .stroke_weight(2.0)
        .color(BLUE);

    draw.line()
        .start(origin)
        .end(m.vectors[2] * unit_mult)
        .stroke_weight(2.0)
        .color(GREEN);

    draw.to_frame(app, &frame).unwrap();
}
