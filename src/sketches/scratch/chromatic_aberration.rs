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
    controls: ControlScript<Timing>,
    grid: Vec<Vec2>,
    cell_size: f32,
}

pub fn init(_app: &App, ctx: &LatticeContext) -> Template {
    let wr = ctx.window_rect();

    let (grid, cell_size) = create_grid(wr.w(), wr.h(), GRID_SIZE, vec2);

    let controls = ControlScriptBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .slider("x_offset", 1.0, (0.0, 20.0), 0.5, None)
        .slider("y_offset", 1.0, (0.0, 20.0), 0.5, None)
        .slider("size_mult", 0.5, (0.0125, 2.0), 0.0125, None)
        .slider("alpha", 0.5, (0.0, 1.0), 0.001, None)
        .build();

    Template {
        controls,
        grid,
        cell_size,
    }
}

impl Sketch for Template {
    fn update(&mut self, _app: &App, _update: Update, ctx: &LatticeContext) {
        let mut wr = ctx.window_rect();

        if wr.changed() {
            (self.grid, self.cell_size) =
                create_grid(wr.w(), wr.h(), GRID_SIZE, vec2);
            wr.mark_unchanged();
        }
    }

    fn view(&self, app: &App, frame: Frame, ctx: &LatticeContext) {
        let draw = app.draw();
        let wr = ctx.window_rect();

        // Background
        draw.rect()
            .x_y(0.0, 0.0)
            .w_h(wr.w(), wr.h())
            .hsla(0.0, 0.0, 1.0, 1.0);

        let cell_size = self.cell_size * self.controls.get("size_mult");
        let x_offset = self.controls.get("x_offset");
        let y_offset = self.controls.get("y_offset");
        let alpha = self.controls.get("alpha");

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
