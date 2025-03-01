use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "responsive_dev",
    display_name: "Responsive Test",
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(150),
    play_mode: PlayMode::Loop,
};

#[derive(LegacySketchComponents)]
pub struct Model {
    window_rect: WindowRect,
    grid: Vec<Vec2>,
    cell_size: f32,
}

pub fn init_model(_app: &App, window_rect: WindowRect) -> Model {
    let (grid, cell_size) =
        create_grid(window_rect.w(), window_rect.h(), 64, vec2);

    Model {
        window_rect,
        grid,
        cell_size,
    }
}

pub fn update(_app: &App, model: &mut Model, _update: Update) {
    if model.window_rect.changed() {
        (model.grid, model.cell_size) =
            create_grid(model.window_rect.w(), model.window_rect.h(), 64, vec2);
        model.window_rect.mark_unchanged();
    }
}

pub fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();

    draw.rect()
        .x_y(0.0, 0.0)
        .w_h(model.window_rect.w(), model.window_rect.h())
        .hsla(0.0, 0.0, 0.02, 0.1);

    for point in model.grid.iter() {
        draw.rect()
            .xy(*point)
            .w_h(model.cell_size, model.cell_size)
            .color(ORANGE)
            .stroke_weight(2.0)
            .stroke(BLACK);
    }

    draw.to_frame(app, &frame).unwrap();
}
