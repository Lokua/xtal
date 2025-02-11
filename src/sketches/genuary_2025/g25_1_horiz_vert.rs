use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

// ~/Documents/Live/2024/2016 Begins

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "g25_1_horiz_vert",
    display_name: "Genuary 1: Vertical or horizontal lines only",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 127.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(220),
};

const N_LINES: u32 = 64;
const GRID_SIZE: u32 = 16;

#[derive(SketchComponents)]
#[sketch(clear_color = "hsla(0.0, 0.0, 0.0, 1.0)")]
pub struct Model {
    #[allow(dead_code)]
    animation: Animation<Timing>,
    controls: Controls,
    wr: WindowRect,
    lines: Vec<Vec2>,
}

pub fn init_model(_app: &App, wr: WindowRect) -> Model {
    let animation = Animation::new(Timing::new(SKETCH_CONFIG.bpm));

    let controls = Controls::new(vec![
        Control::checkbox("invert", false),
        Control::slider_norm("a", 0.5),
        Control::slider_norm("b", 0.5),
        Control::slider("aberration", 0.5, (0.0, 100.0), 1.0),
        Control::slider_norm("background_alpha", 1.0),
    ]);

    Model {
        animation,
        controls,
        wr,
        lines: vec![],
    }
}

pub fn update(_app: &App, m: &mut Model, _update: Update) {
    m.lines.clear();
    let spacing = m.wr.w() / (N_LINES as f32 + 1.0);
    let b = m.controls.float("b") * 100.0;
    let line_length = (m.wr.h() / GRID_SIZE as f32) + b;
    let base_time = m.animation.tri(4.0);
    let lrp_time = 4.0;

    for i in 0..N_LINES {
        let n_lines = N_LINES as f32;
        let i_f32 = i as f32;
        let x = -m.wr.hw() + spacing * (i as f32 + 1.0);
        let start = vec2(x, 0.0);

        let position_offset = i_f32 / n_lines * TAU * base_time * 2.0;

        let wave = (m.animation.lrp(
            &[
                (0.0, lrp_time),
                (1.0, lrp_time),
                (0.0, lrp_time),
                (-1.0, lrp_time),
            ],
            0.0,
        ))
        .sin();

        let envelope =
            (i_f32 / n_lines * TAU + base_time * 0.5 + position_offset * 0.5)
                .cos();

        let end = vec2(x, line_length * wave * envelope);
        m.lines.push(start);
        m.lines.push(end);
    }
}

pub fn view(app: &App, m: &Model, frame: Frame) {
    let draw = app.draw();
    let invert = m.controls.bool("invert");

    draw.rect().wh(m.wr.vec2()).color(hsla(
        0.0,
        0.0,
        if invert { 1.0 } else { 0.0 },
        m.controls.float("background_alpha"),
    ));

    let space = m.wr.h() / GRID_SIZE as f32;

    let mut draw2 = draw.translate(vec3(0.0, -m.wr.hh(), 0.0));
    for i in 0..GRID_SIZE {
        let j = i as f32;
        draw_lines(
            &draw2,
            m,
            (j + 1.0) * (1.0 / GRID_SIZE as f32),
            hsla(0.0, 0.0, if invert { 0.0 } else { 1.0 }, 1.0),
        );
        draw2 = draw2.translate(vec3(0.0, space, 0.0));
    }

    let mut draw3 = draw.rotate(PI / 2.0);
    draw3 = draw3.translate(vec3(0.0, -m.wr.hh(), 0.0));
    for i in 0..GRID_SIZE {
        let j = i as f32;
        draw_lines(
            &draw3,
            m,
            (j + 1.0) * (1.0 / GRID_SIZE as f32),
            hsla(0.0, 0.0, if invert { 0.0 } else { 1.0 }, 1.0),
        );
        draw3 = draw3.translate(vec3(0.0, space, 0.0));
    }

    draw.to_frame(app, &frame).unwrap();
}

pub fn draw_lines(draw: &Draw, m: &Model, anim_delay: f32, _color: Hsla) {
    let time = 6.0;
    let aberration = m.controls.float("aberration");

    for chunk in m.lines.chunks(2) {
        if let [start, end] = chunk {
            let range = m.animation.tri(32.0) * m.wr.hw();

            let animated_center = m
                .animation
                .lrp(&[(-range, time), (range, time)], anim_delay);

            let distance_from_center =
                ((start.x - animated_center) / m.wr.hw()).abs();

            let min_weight = 0.1;
            let max_weight = 4.0;
            let weight =
                max_weight - (max_weight - min_weight) * distance_from_center;

            // Red channel (shifted right)
            draw.line()
                .start(vec2(start.x + aberration, start.y))
                .end(vec2(end.x + aberration, end.y))
                .weight(weight)
                .color(rgba(1.0, 0.0, 0.0, 0.8));

            // Green channel (no shift)
            draw.line()
                .start(*start)
                .end(*end)
                .weight(weight)
                .color(rgba(0.0, 1.0, 0.0, 0.8));

            // Blue channel (shifted left)
            draw.line()
                .start(vec2(start.x - aberration, start.y))
                .end(vec2(end.x - aberration, end.y))
                .weight(weight)
                .color(rgba(0.0, 0.0, 1.0, 0.8));
        }
    }
}
