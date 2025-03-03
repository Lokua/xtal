use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "chromatic_aberration",
    display_name: "Chromatic Aberration",
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(200),
    play_mode: PlayMode::Loop,
};

const GRID_SIZE: usize = 8;

#[derive(LegacySketchComponents)]
pub struct Model {
    window_rect: WindowRect,
    controls: Controls,
    grid: Vec<Vec2>,
    cell_size: f32,
}

pub fn init_model(_app: &App, window_rect: WindowRect) -> Model {
    let (grid, cell_size) =
        create_grid(window_rect.w(), window_rect.h(), GRID_SIZE, vec2);

    let controls = Controls::new(vec![
        Control::slider("x_offset", 1.0, (0.0, 20.0), 0.5),
        Control::slider("y_offset", 1.0, (0.0, 20.0), 0.5),
        Control::slider("size_mult", 0.5, (0.0125, 2.0), 0.0125),
        Control::slider("alpha", 0.5, (0.0, 1.0), 0.001),
    ]);

    Model {
        window_rect,
        controls,
        grid,
        cell_size,
    }
}

pub fn update(_app: &App, model: &mut Model, _update: Update) {
    if model.window_rect.changed() {
        (model.grid, model.cell_size) = create_grid(
            model.window_rect.w(),
            model.window_rect.h(),
            GRID_SIZE,
            vec2,
        );
        model.window_rect.mark_unchanged();
    }
}

pub fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();

    draw.rect()
        .x_y(0.0, 0.0)
        .w_h(model.window_rect.w(), model.window_rect.h())
        .hsla(0.0, 0.0, 1.0, 1.0);

    let cell_size = model.cell_size * model.controls.float("size_mult");
    let x_offset = model.controls.float("x_offset");
    let y_offset = model.controls.float("y_offset");
    let alpha = model.controls.float("alpha");

    for point in model.grid.iter() {
        draw.rect()
            .xy(*point + vec2(x_offset, y_offset))
            .w_h(cell_size, cell_size)
            .color(rgba(255.0, 0.0, 0.0, alpha));

        draw.rect()
            .xy(*point - vec2(x_offset, y_offset))
            .w_h(cell_size, cell_size)
            .color(rgba(0.0, 255.0, 0.0, alpha));

        draw.rect()
            .xy(*point)
            .w_h(cell_size, cell_size)
            .color(rgba(0.0, 0.0, 255.0, alpha));
    }

    draw.to_frame(app, &frame).unwrap();
}
