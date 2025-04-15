use nannou::color::{Gradient, IntoLinSrgba};
use nannou::prelude::*;
use rayon::prelude::*;
use std::sync::Arc;

use crate::framework::prelude::*;
use crate::sketches::shared::displacer::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "displacement_1a",
    display_name: "Displacement 1a",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 1000,
    h: 1000,
};

const GRID_SIZE: usize = 128;

#[derive(SketchComponents)]
pub struct Displacement1a {
    grid: Vec<Vec2>,
    displacer_configs: Vec<DisplacerConfig>,
    controls: ControlHub<Timing>,
    gradient: Gradient<LinSrgb>,
    ellipses: Vec<(Vec2, f32, LinSrgb)>,
}

pub fn init(_app: &App, ctx: &Context) -> Displacement1a {
    let wr = ctx.window_rect();

    let grid_w = wr.w() - 80.0;
    let grid_h = wr.h() - 80.0;

    let controls = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .slider("gradient_spread", 0.5, (0.0, 1.0), 0.0001, None)
        .slider("circle_radius_min", 0.1, (0.1, 4.0), 0.1, None)
        .slider("circle_radius_max", 2.5, (0.1, 4.0), 0.1, None)
        .slider("displacer_radius", 210.0, (0.0, 500.0), 1.0, None)
        .slider("displacer_strength", 95.0, (0.0, 500.0), 1.0, None)
        .slider("scaling_power", 3.0, (0.5, 7.0), 0.25, None)
        .build();

    let displacer_configs = vec![DisplacerConfig::new(
        Displacer::new(vec2(0.0, 0.0), 10.0, 50.0, None),
        None,
        None,
    )];

    Displacement1a {
        grid: create_grid(grid_w, grid_h, GRID_SIZE, vec2).0,
        displacer_configs,
        controls,
        gradient: Gradient::new(vec![
            BEIGE.into_lin_srgb(),
            hsl(0.6, 0.8, 0.7).into_lin_srgba().into(),
        ]),
        ellipses: Vec::with_capacity(GRID_SIZE * GRID_SIZE),
    }
}

impl Sketch for Displacement1a {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &Context) {
        let circle_radius_min = self.controls.get("circle_radius_min");
        let circle_radius_max = self.controls.get("circle_radius_max");
        let radius = self.controls.get("displacer_radius");
        let strength = self.controls.get("displacer_strength");
        let gradient_spread = self.controls.get("gradient_spread");
        let scaling_power = self.controls.get("scaling_power");

        for config in &mut self.displacer_configs {
            config.update(&self.controls.animation, &self.controls.ui_controls);
            config.displacer.set_strength(strength);
            config.displacer.set_radius(radius);
        }

        let max_mag = self.displacer_configs.len() as f32 * strength;
        let gradient = &self.gradient;

        self.ellipses = self
            .grid
            .par_iter()
            .map(|point| {
                let total_displacement = self.displacer_configs.iter().fold(
                    vec2(0.0, 0.0),
                    |acc, config| {
                        acc + config.displacer.attract(*point, scaling_power)
                    },
                );

                let displacement_magnitude = total_displacement.length();

                let color = gradient.get(
                    1.0 - (displacement_magnitude / max_mag)
                        .powf(gradient_spread)
                        .clamp(0.0, 1.0),
                );

                let radius = map_clamp(
                    displacement_magnitude,
                    0.0,
                    max_mag,
                    circle_radius_min,
                    circle_radius_max,
                    |x| x,
                );

                (*point + total_displacement, radius, color)
            })
            .collect();
    }

    fn view(&self, app: &App, frame: Frame, _ctx: &Context) {
        let draw = app.draw();

        draw.background().color(hsl(0.0, 0.0, 0.02));

        for (position, radius, color) in &self.ellipses {
            draw.ellipse()
                .no_fill()
                .stroke(*color)
                .stroke_weight(0.5)
                .radius(*radius)
                .xy(*position);
        }

        draw.to_frame(app, &frame).unwrap();
    }
}

type AnimationFn<R> = Option<
    Arc<dyn Fn(&Displacer, &Animation<Timing>, &UiControls) -> R + Send + Sync>,
>;

struct DisplacerConfig {
    displacer: Displacer,
    position_animation: AnimationFn<Vec2>,
    radius_animation: AnimationFn<f32>,
}

impl DisplacerConfig {
    pub fn new(
        displacer: Displacer,
        position_animation: AnimationFn<Vec2>,
        radius_animation: AnimationFn<f32>,
    ) -> Self {
        Self {
            displacer,
            position_animation,
            radius_animation,
        }
    }

    pub fn update(
        &mut self,
        animation: &Animation<Timing>,
        controls: &UiControls,
    ) {
        if let Some(position_fn) = &self.position_animation {
            self.displacer.position =
                position_fn(&self.displacer, animation, controls);
        }
        if let Some(radius_fn) = &self.radius_animation {
            self.displacer.radius =
                radius_fn(&self.displacer, animation, controls);
        }
    }
}
