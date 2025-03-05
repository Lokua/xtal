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

#[derive(SketchComponents)]
pub struct Template {
    controls: Controls,
    grid: Vec<Vec2>,
    cell_size: f32,
}

pub fn init(_app: &App, ctx: &LatticeContext) -> Template {
    let (grid, cell_size) = create_grid(
        ctx.window_rect().w(),
        ctx.window_rect().h(),
        GRID_SIZE,
        vec2,
    );

    let controls = Controls::new(vec![
        Control::slider("x_offset", 1.0, (0.0, 20.0), 0.5),
        Control::slider("y_offset", 1.0, (0.0, 20.0), 0.5),
        Control::slider("size_mult", 0.5, (0.0125, 2.0), 0.0125),
        Control::slider("alpha", 0.5, (0.0, 1.0), 0.001),
    ]);

    Template {
        controls,
        grid,
        cell_size,
    }
}

impl Sketch for Template {
    fn update(&mut self, _app: &App, _update: Update, ctx: &LatticeContext) {
        if ctx.window_rect().changed() {
            (self.grid, self.cell_size) = create_grid(
                ctx.window_rect().w(),
                ctx.window_rect().h(),
                GRID_SIZE,
                vec2,
            );
            ctx.window_rect().mark_unchanged();
        }
    }

    fn view(&self, app: &App, frame: Frame, ctx: &LatticeContext) {
        let draw = app.draw();

        draw.rect()
            .x_y(0.0, 0.0)
            .w_h(ctx.window_rect().w(), ctx.window_rect().h())
            .hsla(0.0, 0.0, 1.0, 1.0);

        let cell_size = self.cell_size * self.controls.float("size_mult");
        let x_offset = self.controls.float("x_offset");
        let y_offset = self.controls.float("y_offset");
        let alpha = self.controls.float("alpha");

        for point in self.grid.iter() {
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
}
