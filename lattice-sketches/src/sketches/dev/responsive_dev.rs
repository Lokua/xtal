use nannou::color::*;
use nannou::prelude::*;

use lattice::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "responsive_dev",
    display_name: "Responsive Test",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
};

#[derive(SketchComponents)]
pub struct ResponsiveDev {
    grid: Vec<Vec2>,
    cell_size: f32,
}

pub fn init(_app: &App, ctx: &Context) -> ResponsiveDev {
    let wr = ctx.window_rect();
    let (grid, cell_size) = create_grid(wr.w(), wr.h(), 64, vec2);

    ResponsiveDev { grid, cell_size }
}

impl Sketch for ResponsiveDev {
    fn update(&mut self, _app: &App, _update: Update, ctx: &Context) {
        let mut wr = ctx.window_rect();

        if wr.changed() {
            debug!("changed w: {}, h: {}", wr.w(), wr.h());
            (self.grid, self.cell_size) = create_grid(wr.w(), wr.h(), 64, vec2);
            wr.mark_unchanged();
        }
    }

    fn view(&self, app: &App, frame: Frame, ctx: &Context) {
        let wr = ctx.window_rect();
        let draw = app.draw();

        draw.rect()
            .x_y(0.0, 0.0)
            .w_h(wr.w(), wr.h())
            .hsla(0.0, 0.0, 0.02, 0.1);

        for point in self.grid.iter() {
            draw.rect()
                .xy(*point)
                .w_h(self.cell_size, self.cell_size)
                .color(ORANGE)
                .stroke_weight(2.0)
                .stroke(BLACK);
        }

        draw.to_frame(app, &frame).unwrap();
    }
}
