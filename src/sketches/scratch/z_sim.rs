use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "z_sim",
    display_name: "Z Axis Simulation",
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(200),
    play_mode: PlayMode::Loop,
};

const GRID_SIZE: usize = 32;

#[derive(SketchComponents)]
pub struct ZSim {
    controls: ControlHub<Timing>,
    grid: Vec<Vec2>,
    cell_size: f32,
}

pub fn init(_app: &App, ctx: &LatticeContext) -> ZSim {
    let wr = ctx.window_rect();

    let (grid, cell_size) = create_grid(wr.w(), wr.h(), GRID_SIZE, vec2);

    let controls = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .slider("size_mult", 0.5, (0.0125, 2.0), 0.0125, None)
        .slider("alpha", 0.5, (0.0, 1.0), 0.001, None)
        .slider("depth_influence", 1.0, (0.0, 5.0), 0.1, None)
        .build();

    ZSim {
        controls,
        grid,
        cell_size,
    }
}

impl Sketch for ZSim {
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

        draw.rect()
            .x_y(0.0, 0.0)
            .w_h(wr.w(), wr.h())
            .hsla(0.0, 0.0, 1.0, 1.0);

        let hw = wr.w() / 2.0;
        let hh = wr.h() / 2.0;
        let cell_size = self.cell_size * self.controls.float("size_mult");
        let alpha = self.controls.float("alpha");
        let depth_influence = self.controls.float("depth_influence");
        let max_possible_dist = hw.max(hh);

        let center = vec2(
            self.controls.animation.random_slewed(
                2.0,
                (-hw, hw),
                0.8,
                0.0,
                946,
            ),
            self.controls.animation.random_slewed(
                1.0,
                (-hw, hw),
                0.6,
                0.0,
                765,
            ),
        );

        for point in self.grid.iter() {
            let dist_from_center = point.distance(center);
            let depth = 1.0
                - (dist_from_center / max_possible_dist).clamp(0.0, 1.0)
                    * depth_influence;

            // Modify size based on depth
            // Further objects are smaller
            let depth_adjusted_size = cell_size * (0.5 + depth);

            // Modify color based on depth
            // Further objects are darker
            let color_intensity = 0.3 + (depth * 0.7);
            let depth_color = rgba(
                255.0 * color_intensity,
                0.0,
                0.0,
                // Further objects more transparent
                alpha * (0.3 + depth * 0.7),
            );

            draw.rect()
                .xy(*point)
                .w_h(depth_adjusted_size, depth_adjusted_size)
                .color(depth_color);
        }

        draw.to_frame(app, &frame).unwrap();
    }
}
