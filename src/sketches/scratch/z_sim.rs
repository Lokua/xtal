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

#[derive(LegacySketchComponents)]
pub struct Model {
    window_rect: WindowRect,
    controls: Controls,
    animation: Animation<FrameTiming>,
    grid: Vec<Vec2>,
    cell_size: f32,
}

pub fn init_model(_app: &App, window_rect: WindowRect) -> Model {
    let animation = Animation::new(FrameTiming::new(SKETCH_CONFIG.bpm));

    let (grid, cell_size) =
        create_grid(window_rect.w(), window_rect.h(), GRID_SIZE, vec2);

    let controls = Controls::new(vec![
        Control::slider("size_mult", 0.5, (0.0125, 2.0), 0.0125),
        Control::slider("alpha", 0.5, (0.0, 1.0), 0.001),
        Control::slider("depth_influence", 1.0, (0.0, 5.0), 0.1),
    ]);

    Model {
        window_rect,
        controls,
        animation,
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

    let hw = model.window_rect.w() / 2.0;
    let hh = model.window_rect.h() / 2.0;
    let cell_size = model.cell_size * model.controls.float("size_mult");
    let alpha = model.controls.float("alpha");
    let depth_influence = model.controls.float("depth_influence");
    let max_possible_dist = hw.max(hh);

    let center = vec2(
        model.animation.r_ramp(
            &[kfr((-hw, hw), 2.0)],
            0.0,
            1.0,
            Easing::Linear,
        ),
        model.animation.r_ramp(
            &[kfr((-hh, hh), 1.0)],
            0.0,
            0.5,
            Easing::Linear,
        ),
    );

    for point in model.grid.iter() {
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
