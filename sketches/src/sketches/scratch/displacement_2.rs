use lattice::prelude::*;
use nannou::color::{Gradient, Mix};
use nannou::prelude::*;
use rayon::prelude::*;
use std::sync::Arc;

use crate::sketches::shared::displacer::*;
use crate::util::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "displacement_2",
    display_name: "Displacement 2",
    play_mode: PlayMode::Loop,
    fps: 30.0,
    bpm: 134.0,
    w: 1000,
    h: 1000,
};

const GRID_SIZE: usize = 128;

type TrigFnFns = Option<(fn(f32) -> f32, fn(f32) -> f32)>;

#[derive(SketchComponents)]
pub struct Displacement2 {
    grid: Vec<Vec2>,
    displacer_configs: Vec<DisplacerConfig>,
    controls: ControlHub<Timing>,
    cached_pattern: String,
    cached_trig_fns: TrigFnFns,
    gradient: Gradient<LinSrgb>,
    ellipses: Vec<(Vec2, f32, LinSrgb)>,
}

impl Displacement2 {
    fn update_trig_fns(&mut self) {
        let pattern = self.controls.string("pattern");
        let lookup = trig_fn_lookup();
        let parts: Vec<&str> = pattern.split(',').collect();

        self.cached_trig_fns = if parts.len() == 2 {
            match (lookup.get(parts[0]), lookup.get(parts[1])) {
                (Some(&f_a), Some(&f_b)) => {
                    self.cached_pattern = pattern;
                    Some((f_a, f_b))
                }
                _ => {
                    error!("Unknown function(s) in pattern: {}", pattern);
                    None
                }
            }
        } else if pattern == "Comp1" {
            self.cached_pattern = pattern;
            None
        } else {
            error!("Invalid pattern: {}", pattern);
            None
        };
    }

    fn weave_frequency(&self) -> f32 {
        let value = self.controls.get("weave_frequency");
        if self.controls.bool("animate_frequency") {
            map_range(self.controls.animation.tri(32.0), 0.0, 1.0, 0.01, value)
        } else {
            value
        }
    }
}

pub fn init(_app: &App, ctx: &Context) -> Displacement2 {
    let wr = ctx.window_rect();

    let controls = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .select("pattern", "cos,sin", &generate_pattern_options(), None)
        .checkbox("clamp_circle_radii", false, None)
        .checkbox("animate_frequency", false, None)
        .slider("gradient_spread", 0.99, (0.0, 1.0), 0.0001, None)
        .slider("circle_radius_min", 1.0, (0.1, 12.0), 0.1, None)
        .slider("circle_radius_max", 5.0, (0.1, 12.0), 0.1, None)
        .slider("displacer_radius", 0.001, (0.0001, 0.01), 0.0001, None)
        .slider("displacer_strength", 34.0, (0.5, 100.0), 0.5, None)
        .slider("weave_frequency", 0.01, (0.01, 0.2), 0.001, None)
        .slider("weave_scale", 0.05, (0.001, 0.1), 0.001, None)
        .slider("weave_amplitude", 0.001, (0.0001, 0.01), 0.0001, None)
        .checkbox("center", true, None)
        .checkbox("quad_1", false, None)
        .checkbox("quad_2", false, None)
        .checkbox("quad_3", false, None)
        .checkbox("quad_4", false, None)
        .build();

    let displacer_configs = vec![
        DisplacerConfig::new(
            "center",
            Displacer::new(vec2(0.0, 0.0), 20.0, 10.0, None),
            None,
            None,
        ),
        DisplacerConfig::new(
            "quad_1",
            Displacer::new(vec2(wr.w() / 4.0, wr.h() / 4.0), 20.0, 10.0, None),
            None,
            None,
        ),
        DisplacerConfig::new(
            "quad_2",
            Displacer::new(vec2(wr.w() / 4.0, -wr.h() / 4.0), 20.0, 10.0, None),
            None,
            None,
        ),
        DisplacerConfig::new(
            "quad_3",
            Displacer::new(
                vec2(-wr.w() / 4.0, -wr.h() / 4.0),
                20.0,
                10.0,
                None,
            ),
            None,
            None,
        ),
        DisplacerConfig::new(
            "quad_4",
            Displacer::new(vec2(-wr.w() / 4.0, wr.h() / 4.0), 20.0, 10.0, None),
            None,
            None,
        ),
    ];

    let pad = 80.0;
    let cached_pattern = controls.string("pattern");

    Displacement2 {
        grid: create_grid(wr.w() - pad, wr.h() - pad, GRID_SIZE, vec2).0,
        displacer_configs,
        controls,
        cached_pattern,
        cached_trig_fns: None,
        gradient: Gradient::new(vec![
            LIGHTCORAL.into_lin_srgb(),
            AZURE.into_lin_srgb(),
        ]),
        ellipses: Vec::with_capacity(GRID_SIZE),
    }
}

impl Sketch for Displacement2 {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &Context) {
        if self.cached_trig_fns.is_none()
            || (self.cached_pattern != self.controls.string("pattern"))
        {
            self.update_trig_fns();
        }

        let displacer_radius = self.controls.get("displacer_radius");
        let displacer_strength = self.controls.get("displacer_strength");
        let weave_scale = self.controls.get("weave_scale");
        let weave_amplitude = self.controls.get("weave_amplitude");
        let pattern = self.controls.string("pattern");
        let gradient_spread = self.controls.get("gradient_spread");
        let clamp_circle_radii = self.controls.bool("clamp_circle_radii");
        let circle_radius_min = self.controls.get("circle_radius_min");
        let circle_radius_max = self.controls.get("circle_radius_max");
        let animation = &self.controls.animation;
        let controls = &self.controls;
        let weave_frequency = self.weave_frequency();

        let cached_trig_fns = self.cached_trig_fns;
        let distance_fn: CustomDistanceFn =
            Some(Arc::new(move |grid_point, position| {
                weave(
                    grid_point.x,
                    grid_point.y,
                    position.x,
                    position.y,
                    weave_frequency,
                    weave_scale,
                    weave_amplitude,
                    pattern.clone(),
                    cached_trig_fns,
                )
            }));

        for config in self.displacer_configs.iter_mut() {
            config.update(animation, &controls.ui_controls);
            config.displacer.set_custom_distance_fn(distance_fn.clone());
            config.displacer.set_radius(displacer_radius);
            config.displacer.set_strength(displacer_strength);
        }

        let enabled_displacer_configs: Vec<&DisplacerConfig> = self
            .displacer_configs
            .iter()
            .filter(|x| self.controls.bool(x.kind))
            .collect();

        let max_mag = self.displacer_configs.len() as f32 * displacer_strength;
        let gradient = &self.gradient;

        self.ellipses = self
            .grid
            .par_iter()
            .map(|point| {
                let mut total_displacement = vec2(0.0, 0.0);
                let mut total_influence = 0.0;

                for config in &enabled_displacer_configs {
                    let displacement = config.displacer.influence(*point);
                    let influence = displacement.length();
                    total_displacement += displacement;
                    total_influence += influence;
                }

                let mut blended_color = gradient.get(0.0);
                let inv_total = 1.0 / total_influence.max(1.0);

                for config in &enabled_displacer_configs {
                    let displacement = config.displacer.influence(*point);
                    let influence = displacement.length();
                    let color_position = (influence
                        / config.displacer.strength)
                        .powf(gradient_spread)
                        .clamp(0.0, 1.0);
                    let color = gradient.get(color_position);
                    let weight = influence * inv_total;
                    blended_color = blended_color.mix(&color, weight);
                }

                let magnitude = if clamp_circle_radii {
                    total_displacement.length().clamp(0.0, max_mag)
                } else {
                    total_displacement.length()
                };
                let radius = map_range(
                    magnitude,
                    0.0,
                    max_mag,
                    circle_radius_min,
                    circle_radius_max,
                );

                (*point + total_displacement, radius, blended_color)
            })
            .collect();
    }

    fn view(&self, app: &App, frame: Frame, _ctx: &Context) {
        let draw = app.draw();

        frame.clear(BLACK);
        draw.background().color(rgb(0.1, 0.1, 0.1));

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
    kind: &'static str,
    displacer: Displacer,
    position_animation: AnimationFn<Vec2>,
    radius_animation: AnimationFn<f32>,
}

impl DisplacerConfig {
    pub fn new(
        kind: &'static str,
        displacer: Displacer,
        position_animation: AnimationFn<Vec2>,
        radius_animation: AnimationFn<f32>,
    ) -> Self {
        Self {
            kind,
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

#[allow(clippy::too_many_arguments)]
pub fn weave(
    grid_x: f32,
    grid_y: f32,
    center_x: f32,
    center_y: f32,
    frequency: f32,
    distance_scale: f32,
    amplitude: f32,
    pattern: String,
    trig_fns: TrigFnFns,
) -> f32 {
    let x = grid_x * frequency;
    let y = grid_y * frequency;

    let position_pattern = match pattern.as_str() {
        "Comp1" => (x.sin() * y.cos()) + (x - y).tanh() * (x + y).tan(),
        _ => match trig_fns {
            Some((f_a, f_b)) => f_a(x) + f_b(y),
            None => 0.0,
        },
    };

    let distance =
        ((center_x - grid_x).powi(2) + (center_y - grid_y).powi(2)).sqrt();

    let distance_pattern = (distance * distance_scale).sin();

    position_pattern * distance_pattern * amplitude
}

fn generate_pattern_options() -> Vec<String> {
    let functions = [
        "cos", "sin", "tan", "tanh", "sec", "csc", "cot", "sech", "csch",
        "coth",
    ];

    let mut options: Vec<String> = functions
        .iter()
        .flat_map(|a| functions.iter().map(move |b| format!("{},{}", a, b)))
        .collect();

    let custom_algs = vec!["Comp1".into()];

    options.extend(custom_algs);

    options
}
