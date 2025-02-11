use nannou::color::{Gradient, Mix};
use nannou::prelude::*;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

use crate::framework::prelude::*;
use crate::sketches::shared::displacer::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "displacement_2",
    display_name: "Displacement 2",
    fps: 30.0,
    bpm: 134.0,
    w: 1000,
    h: 1000,
    gui_w: None,
    gui_h: Some(450),
    play_mode: PlayMode::Loop,
};

const GRID_SIZE: usize = 128;

#[derive(SketchComponents)]
pub struct Model {
    grid: Vec<Vec2>,
    displacer_configs: Vec<DisplacerConfig>,
    animation: Animation<FrameTiming>,
    controls: Controls,
    cached_pattern: String,
    cached_trig_fns: Option<(fn(f32) -> f32, fn(f32) -> f32)>,
    gradient: Gradient<LinSrgb>,
    ellipses: Vec<(Vec2, f32, LinSrgb)>,
}

impl Model {
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
        let value = self.controls.float("weave_frequency");
        if self.controls.bool("animate_frequency") {
            map_range(self.animation.tri(32.0), 0.0, 1.0, 0.01, value)
        } else {
            value
        }
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

pub fn init_model(_app: &App, _window_rect: WindowRect) -> Model {
    let w = SKETCH_CONFIG.w;
    let h = SKETCH_CONFIG.h;
    let animation = Animation::new(FrameTiming::new(SKETCH_CONFIG.bpm));

    let controls = Controls::new(vec![
        Control::Select {
            name: "pattern".into(),
            value: "cos,sin".into(),
            options: generate_pattern_options(),
            disabled: None,
        },
        Control::Checkbox {
            name: "clamp_circle_radii".into(),
            value: false,
            disabled: None,
        },
        Control::Checkbox {
            name: "animate_frequency".into(),
            value: false,
            disabled: None,
        },
        Control::Slider {
            name: "gradient_spread".into(),
            value: 0.99,
            min: 0.0,
            max: 1.0,
            step: 0.0001,
            disabled: None,
        },
        Control::Slider {
            name: "circle_radius_min".into(),
            value: 1.0,
            min: 0.1,
            max: 12.0,
            step: 0.1,
            disabled: None,
        },
        Control::Slider {
            name: "circle_radius_max".into(),
            value: 5.0,
            min: 0.1,
            max: 12.0,
            step: 0.1,
            disabled: None,
        },
        Control::Slider {
            name: "displacer_radius".into(),
            value: 0.001,
            min: 0.0001,
            max: 0.01,
            step: 0.0001,
            disabled: None,
        },
        Control::Slider {
            name: "displacer_strength".into(),
            value: 34.0,
            min: 0.5,
            max: 100.0,
            step: 0.5,
            disabled: None,
        },
        Control::Slider {
            name: "weave_frequency".into(),
            value: 0.01,
            min: 0.01,
            max: 0.2,
            step: 0.001,
            disabled: None,
        },
        Control::Slider {
            name: "weave_scale".into(),
            value: 0.05,
            min: 0.001,
            max: 0.1,
            step: 0.001,
            disabled: None,
        },
        Control::Slider {
            name: "weave_amplitude".into(),
            value: 0.001,
            min: 0.0001,
            max: 0.01,
            step: 0.0001,
            disabled: None,
        },
        Control::Checkbox {
            name: "center".into(),
            value: true,
            disabled: None,
        },
        Control::Checkbox {
            name: "quad_1".into(),
            value: false,
            disabled: None,
        },
        Control::Checkbox {
            name: "quad_2".into(),
            value: false,
            disabled: None,
        },
        Control::Checkbox {
            name: "quad_3".into(),
            value: false,
            disabled: None,
        },
        Control::Checkbox {
            name: "quad_4".into(),
            value: false,
            disabled: None,
        },
    ]);

    let displacer_configs = vec![
        DisplacerConfig::new(
            "center",
            Displacer::new(vec2(0.0, 0.0), 20.0, 10.0, None),
            None,
            None,
        ),
        DisplacerConfig::new(
            "quad_1",
            Displacer::new(
                vec2(w as f32 / 4.0, h as f32 / 4.0),
                20.0,
                10.0,
                None,
            ),
            None,
            None,
        ),
        DisplacerConfig::new(
            "quad_2",
            Displacer::new(
                vec2(w as f32 / 4.0, -h as f32 / 4.0),
                20.0,
                10.0,
                None,
            ),
            None,
            None,
        ),
        DisplacerConfig::new(
            "quad_3",
            Displacer::new(
                vec2(-w as f32 / 4.0, -h as f32 / 4.0),
                20.0,
                10.0,
                None,
            ),
            None,
            None,
        ),
        DisplacerConfig::new(
            "quad_4",
            Displacer::new(
                vec2(-w as f32 / 4.0, h as f32 / 4.0),
                20.0,
                10.0,
                None,
            ),
            None,
            None,
        ),
    ];

    let pad = 80.0;
    let cached_pattern = controls.string("pattern");

    Model {
        grid: create_grid(w as f32 - pad, h as f32 - pad, GRID_SIZE, vec2).0,
        displacer_configs,
        animation,
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

pub fn update(_app: &App, model: &mut Model, _update: Update) {
    if model.cached_trig_fns == None
        || (model.cached_pattern != model.controls.string("pattern"))
    {
        model.update_trig_fns();
    }

    let displacer_radius = model.controls.float("displacer_radius");
    let displacer_strength = model.controls.float("displacer_strength");
    let weave_scale = model.controls.float("weave_scale");
    let weave_amplitude = model.controls.float("weave_amplitude");
    let pattern = model.controls.string("pattern");
    let gradient_spread = model.controls.float("gradient_spread");
    let clamp_circle_radii = model.controls.bool("clamp_circle_radii");
    let circle_radius_min = model.controls.float("circle_radius_min");
    let circle_radius_max = model.controls.float("circle_radius_max");
    let animation = &model.animation;
    let controls = &model.controls;
    let weave_frequency = model.weave_frequency();

    let cached_trig_fns = model.cached_trig_fns.clone();
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

    for config in model.displacer_configs.iter_mut() {
        config.update(animation, controls);
        config.displacer.set_custom_distance_fn(distance_fn.clone());
        config.displacer.set_radius(displacer_radius);
        config.displacer.set_strength(displacer_strength);
    }

    let enabled_displacer_configs: Vec<&DisplacerConfig> = model
        .displacer_configs
        .iter()
        .filter(|x| model.controls.bool(x.kind))
        .collect();

    let max_mag = model.displacer_configs.len() as f32 * displacer_strength;
    let gradient = &model.gradient;

    model.ellipses = model
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
                let color_position = (influence / config.displacer.strength)
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

pub fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();

    frame.clear(BLACK);
    draw.background().color(rgb(0.1, 0.1, 0.1));

    for (position, radius, color) in &model.ellipses {
        draw.ellipse()
            .no_fill()
            .stroke(*color)
            .stroke_weight(0.5)
            .radius(*radius)
            .xy(*position);
    }

    draw.to_frame(app, &frame).unwrap();
}

pub fn weave(
    grid_x: f32,
    grid_y: f32,
    center_x: f32,
    center_y: f32,
    frequency: f32,
    distance_scale: f32,
    amplitude: f32,
    pattern: String,
    trig_fns: Option<(fn(f32) -> f32, fn(f32) -> f32)>,
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

fn trig_fn_lookup() -> HashMap<&'static str, fn(f32) -> f32> {
    let mut map = HashMap::new();
    map.insert("cos", f32::cos as fn(f32) -> f32);
    map.insert("sin", f32::sin as fn(f32) -> f32);
    map.insert("tan", f32::tan as fn(f32) -> f32);
    map.insert("tanh", f32::tanh as fn(f32) -> f32);
    map.insert("sec", f32::sec as fn(f32) -> f32);
    map.insert("csc", f32::csc as fn(f32) -> f32);
    map.insert("cot", f32::cot as fn(f32) -> f32);
    map.insert("sech", f32::sech as fn(f32) -> f32);
    map.insert("csch", f32::csch as fn(f32) -> f32);
    map.insert("coth", f32::coth as fn(f32) -> f32);
    map
}

fn generate_pattern_options() -> Vec<String> {
    let functions = vec![
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
