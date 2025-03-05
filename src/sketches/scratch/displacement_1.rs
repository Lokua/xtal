use nannou::color::{Gradient, IntoLinSrgba, Mix};
use nannou::prelude::*;
use rayon::prelude::*;
use std::sync::Arc;

use crate::framework::prelude::*;
use crate::sketches::shared::displacer::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "displacement_1",
    display_name: "Displacement 1",
    fps: 60.0,
    bpm: 134.0,
    w: 1000,
    h: 1000,
    gui_w: None,
    gui_h: Some(220),
    play_mode: PlayMode::Loop,
};

const GRID_SIZE: usize = 128;

#[derive(SketchComponents)]
pub struct Displacement1 {
    grid: Vec<Vec2>,
    displacer_configs: Vec<DisplacerConfig>,
    animation: Animation<FrameTiming>,
    controls: Controls,
    gradient: Gradient<LinSrgb>,
    ellipses: Vec<(Vec2, f32, LinSrgb)>,
}

pub fn init(_app: &App, ctx: &LatticeContext) -> Displacement1 {
    let w = SKETCH_CONFIG.w;
    let h = SKETCH_CONFIG.h;
    let grid_w = w as f32 - 80.0;
    let grid_h = h as f32 - 80.0;
    let animation = Animation::new(FrameTiming::new(ctx.bpm()));

    let controls = Controls::new(vec![
        Control::slider("gradient_spread", 0.5, (0.0, 1.0), 0.0001),
        Control::slider("circle_radius_min", 0.1, (0.1, 4.0), 0.1),
        Control::slider("circle_radius_max", 2.5, (0.1, 4.0), 0.1),
        Control::slider("displacer_radius", 210.0, (0.0, 500.0), 1.0),
        Control::slider("displacer_strength", 95.0, (0.0, 500.0), 1.0),
    ]);

    let displacer_configs = vec![DisplacerConfig::new(
        Displacer::new(vec2(0.0, 0.0), 10.0, 50.0, None),
        None,
        None,
    )];

    Displacement1 {
        grid: create_grid(grid_w, grid_h, GRID_SIZE, vec2).0,
        displacer_configs,
        animation,
        controls,
        gradient: Gradient::new(vec![
            BEIGE.into_lin_srgb(),
            hsl(0.6, 0.8, 0.7).into_lin_srgba().into(),
        ]),
        ellipses: Vec::with_capacity(GRID_SIZE * GRID_SIZE),
    }
}

impl Sketch for Displacement1 {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &LatticeContext) {
        let circle_radius_min = self.controls.float("circle_radius_min");
        let circle_radius_max = self.controls.float("circle_radius_max");
        let radius = self.controls.float("displacer_radius");
        let strength = self.controls.float("displacer_strength");
        let gradient_spread = self.controls.float("gradient_spread");

        for config in &mut self.displacer_configs {
            config.update(&self.animation, &self.controls);
            config.displacer.set_strength(strength);
            config.displacer.set_radius(radius);
        }

        let max_mag = self.displacer_configs.len() as f32 * strength;
        let gradient = &self.gradient;

        self.ellipses = self
            .grid
            .par_iter()
            .map(|point| {
                // Initialize accumulators for total displacement and influence
                // These track the combined effect of all displacers on this point
                let mut total_displacement = vec2(0.0, 0.0);
                let mut total_influence = 0.0;

                // First pass: Calculate displacements and influences from all displacers
                // Store these for later use in color calculations
                let displacements: Vec<(Vec2, f32)> = self
                    .displacer_configs
                    .iter()
                    .map(|config| {
                        // Calculate how this displacer affects the current point
                        let displacement = config.displacer.influence(*point);
                        // Get the magnitude of the displacement as influence
                        let influence = displacement.length();

                        // Add to running totals
                        total_displacement += displacement; // Accumulate the vector displacement
                        total_influence += influence; // Accumulate the scalar influence

                        // Return both for use in color calculation
                        (displacement, influence)
                    })
                    .collect();

                // Initialize vector to store color contributions from each displacer
                let mut colors: Vec<(LinSrgb, f32)> = Vec::new();

                // Second pass: Calculate color contribution from each displacer
                for (index, config) in self.displacer_configs.iter().enumerate()
                {
                    let (_displacement, influence) = displacements[index];

                    // Calculate color position in gradient based on relative influence
                    // Higher influence = more intense color
                    let color_position = (influence
                        / config.displacer.strength)
                        .powf(gradient_spread) // Apply non-linear scaling from UI control
                        .clamp(0.0, 1.0); // Ensure we stay within gradient bounds

                    // Get the color from our gradient at this position
                    let color = gradient.get(color_position);

                    // Calculate how much this displacer's color should contribute
                    // based on its influence relative to total influence
                    let weight = influence / total_influence.max(1.0);

                    colors.push((color, weight));
                }

                // Blend all colors together based on their weights
                // Starting with the base gradient color (position 0.0)
                let blended_color = colors
                    .iter()
                    .fold(gradient.get(0.0), |acc, (color, weight)| {
                        acc.mix(color, *weight)
                    });

                let radius = map_range(
                    total_displacement.length(),
                    0.0,
                    max_mag,
                    circle_radius_min,
                    circle_radius_max,
                );

                (*point + total_displacement, radius, blended_color)
            })
            .collect();
    }

    fn view(&self, app: &App, frame: Frame, _ctx: &LatticeContext) {
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
    Arc<
        dyn Fn(&Displacer, &Animation<FrameTiming>, &Controls) -> R
            + Send
            + Sync,
    >,
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
        animation: &Animation<FrameTiming>,
        controls: &Controls,
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
