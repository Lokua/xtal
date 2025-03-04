use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

// Live/2025/Lattice - ControlScript Test

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "control_script_dev",
    display_name: "ControlScript Test",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 500,
    h: 500,
    gui_w: None,
    gui_h: Some(500),
};

#[derive(LegacySketchComponents)]
pub struct Model {
    controls: ControlScript<Timing>,
    wr: WindowRect,
}

pub fn init_model(_app: &App, wr: WindowRect) -> Model {
    let controls = ControlScript::from_path(
        to_absolute_path(file!(), "control_script_dev.yaml"),
        Timing::new(Bpm::new(SKETCH_CONFIG.bpm)),
    );

    Model { controls, wr }
}

pub fn update(_app: &App, m: &mut Model, _update: Update) {
    m.controls.update();
}

pub fn view(app: &App, m: &Model, frame: Frame) {
    let draw = app.draw();

    // background
    draw.rect().x_y(0.0, 0.0).w_h(m.wr.w(), m.wr.h()).hsla(
        0.0,
        0.0,
        0.02,
        m.controls.get("bg_alpha"),
    );

    if m.controls.bool("show_center_circle") {
        draw.ellipse()
            .color(hsl(m.controls.get("center_hue"), 0.5, 0.5))
            .radius(m.controls.get("center_radius"))
            .x_y(0.0, 0.0);
    }

    if m.controls.bool("show_white_circle") {
        draw.ellipse()
            .color(WHITE)
            .radius(m.controls.get("white_radius"))
            .x_y(
                m.controls.get("white_pos_x") * m.wr.hw(),
                m.controls.get("white_pos_y") * m.wr.hh(),
            );
    }

    if m.controls.bool("show_audio") {
        draw.rect()
            .color(CYAN)
            .x_y(
                0.0,
                map_range(
                    m.controls.get("audio_rect_y"),
                    0.0,
                    1.0,
                    -m.wr.hh(),
                    m.wr.hh(),
                ),
            )
            .w_h(m.wr.w() - 100.0, 30.0);
    }

    if m.controls.bool("show_breakpoints") {
        draw.rect()
            .color(MAGENTA)
            .x_y(
                0.0,
                map_range(
                    m.controls.get("breakpoints_line"),
                    0.0,
                    1.0,
                    -m.wr.hh(),
                    m.wr.hh(),
                ),
            )
            .w_h(m.wr.w(), 20.0);
    }

    if m.controls.bool("show_red_circle") {
        draw.ellipse()
            .color(RED)
            .radius(m.controls.get("red_circle_radius"))
            .x_y(
                m.controls.get("red_circle_pos_x") * m.wr.hw(),
                -m.wr.h() / 4.0,
            );
    }

    if m.controls.bool("show_midi_circle") {
        draw.ellipse()
            .color(YELLOW)
            .radius(m.controls.get("midi_radius"))
            .x_y(0.0, 0.0);
    }

    draw.to_frame(app, &frame).unwrap();
}
