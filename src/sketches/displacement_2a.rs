use geom::Ellipse;
use nannou::color::{Gradient, Mix};
use nannou::prelude::*;
use rayon::prelude::*;
use std::sync::Arc;

use super::shared::displacer::*;
use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "displacement_2a",
    display_name: "Displacement 2a",
    fps: 30.0,
    bpm: 135.0,
    w: 1000,
    h: 1000,
    gui_w: None,
    gui_h: Some(840),
    play_mode: PlayMode::Loop,
};

const DEBUG_QUADS: bool = false;
const GRID_SIZE: usize = 128;
const SAMPLE_RATE: usize = 48_000;
const N_BANDS: usize = 8;
const CIRCLE_RESOLUTION: f32 = 6.0;

#[derive(SketchComponents)]
pub struct Model {
    grid: Vec<Vec2>,
    displacer_configs: Vec<DisplacerConfig>,
    animation: Animation<Timing>,
    controls: Controls,
    cached_pattern: String,
    cached_trig_fns: Option<(fn(f32) -> f32, fn(f32) -> f32)>,
    palettes: Vec<Gradient<LinSrgb>>,
    ellipses: Vec<(Vec2, f32, LinSrgb)>,
    audio: Audio,
    fft_bands: Vec<f32>,
    last_position_animation: String,
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
        } else {
            self.cached_pattern = pattern;
            None
        };
    }
    fn weave_frequency(&self) -> f32 {
        let value = self.controls.float("weave_frequency");
        if self.controls.bool("animate_frequency") {
            map_range(
                self.animation
                    .lerp(&[kf(0.0, 16.0), kf(1.0, 16.0), kf(0.0, 0.0)], 0.0),
                0.0,
                1.0,
                0.01,
                value,
            )
        } else {
            value
        }
    }
}

pub fn init_model(_app: &App, _window_rect: WindowRect) -> Model {
    let w = SKETCH_CONFIG.w;
    let h = SKETCH_CONFIG.h;
    let animation = Animation::new(Timing::new(SKETCH_CONFIG.bpm));
    let audio = Audio::new(SAMPLE_RATE, SKETCH_CONFIG.fps);

    let controls = Controls::new(vec![
        Control::checkbox("audio_enabled", false),
        Control::slider("rise_rate", 0.96, (0.001, 1.0), 0.001),
        Control::slider("fall_rate", 0.9, (0.0, 1.0), 0.001),
        Control::Separator {},
        Control::select("pattern", "cos,sin", &generate_pattern_options()),
        Control::slider("scale", 1.0, (0.1, 4.0), 0.1),
        Control::checkbox("clamp_circle_radii", false),
        Control::checkbox("animate_frequency", false),
        Control::Separator {},
        Control::checkbox("quad_restraint", false),
        Control::slider("qr_lerp", 0.5, (0.0, 1.0), 0.0001),
        Control::slider("qr_divisor", 2.0, (0.5, 16.0), 0.125),
        Control::slider("qr_pos", 1.0, (0.125, 1.0), 0.125),
        Control::slider("qr_size", 1.0, (0.125, 1.0), 0.125),
        Control::select(
            "position_animation",
            "Counter Clockwise",
            &["None", "Counter Clockwise"],
        ),
        Control::select(
            "qr_shape",
            "Rectangle",
            &["Rectangle", "Circle", "Ripple", "Spiral"],
        ),
        Control::checkbox("quad_1", false),
        Control::checkbox("quad_2", false),
        Control::checkbox("quad_3", false),
        Control::checkbox("quad_4", false),
        Control::checkbox("quad_influence_or_attract", false),
        Control::Separator {},
        Control::checkbox("center", true),
        Control::checkbox("center_influence_or_attract", false),
        Control::Separator {},
        Control::select("palette", "lightcoral", &["lightcoral", "lightgreen"]),
        Control::slider("gradient_spread", 0.99, (0.0, 1.0), 0.0001),
        Control::checkbox("color_influence_or_attract", false),
        Control::Separator {},
        Control::slider("circle_radius_min", 1.0, (0.1, 12.0), 0.1),
        Control::slider("circle_radius_max", 5.0, (0.1, 12.0), 0.1),
        Control::slider("displacer_radius", 0.001, (0.0001, 0.01), 0.0001),
        Control::slider("displacer_strength", 34.0, (0.5, 100.0), 0.5),
        Control::slider("weave_frequency", 0.03, (0.01, 0.2), 0.001),
        Control::slider("weave_scale", 0.05, (0.001, 0.1), 0.001),
        Control::slider("weave_amplitude", 0.001, (0.0001, 0.01), 0.0001),
        Control::slider("scaling_power", 2.0, (0.25, 9.00), 0.25),
    ]);

    let mut displacer_configs = vec![
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

    let last_position_animation = controls.string("position_animation");
    let position_animations = animation_fns(&last_position_animation);
    for i in 0..displacer_configs.len() {
        displacer_configs[i].position_animation =
            position_animations[i].clone();
    }

    let pad = w as f32 * (1.0 / 3.0);
    let cached_pattern = controls.string("pattern");
    let palettes = vec![
        Gradient::new(vec![LIGHTCORAL.into_lin_srgb(), AZURE.into_lin_srgb()]),
        Gradient::new(vec![LIGHTGREEN.into_lin_srgb(), AZURE.into_lin_srgb()]),
    ];

    Model {
        grid: create_grid(w as f32 - pad, h as f32 - pad, GRID_SIZE, vec2).0,
        displacer_configs,
        animation,
        controls,
        cached_pattern,
        cached_trig_fns: None,
        ellipses: Vec::with_capacity(GRID_SIZE),
        audio,
        fft_bands: Vec::new(),
        last_position_animation,
        palettes,
    }
}

pub fn update(app: &App, model: &mut Model, _update: Update) {
    let audio_enabled = model.controls.bool("audio_enabled");
    let clamp_circle_radii = model.controls.bool("clamp_circle_radii");
    let quad_restraint = model.controls.bool("quad_restraint");
    let qr_shape = model.controls.string("qr_shape");
    let qr_lerp = model.controls.float("qr_lerp");
    let qr_divisor = model.controls.float("qr_divisor");
    let qr_pos = model.controls.float("qr_pos");
    let qr_size = model.controls.float("qr_size");
    let gradient_spread = model.controls.float("gradient_spread");
    let displacer_radius = model.controls.float("displacer_radius");
    let displacer_strength = model.controls.float("displacer_strength");
    let weave_scale = model.controls.float("weave_scale");
    let weave_amplitude = model.controls.float("weave_amplitude");
    let pattern = model.controls.string("pattern");
    let circle_radius_min = model.controls.float("circle_radius_min");
    let circle_radius_max = model.controls.float("circle_radius_max");
    let weave_frequency = model.weave_frequency();
    let scaling_power = model.controls.float("scaling_power");
    let quad_influence_or_attract =
        model.controls.bool("quad_influence_or_attract");
    let center_influence_or_attract =
        model.controls.bool("center_influence_or_attract");
    let color_influence_or_attract =
        model.controls.bool("color_influence_or_attract");

    if model.cached_trig_fns == None
        || (model.cached_pattern != model.controls.string("pattern"))
    {
        model.update_trig_fns();
    }

    if model.last_position_animation
        != model.controls.string("position_animation")
    {
        model.last_position_animation =
            model.controls.string("position_animation");
        let position_animations = animation_fns(&model.last_position_animation);
        for i in 0..model.displacer_configs.len() {
            model.displacer_configs[i].position_animation =
                position_animations[i].clone();
        }
    }

    model.fft_bands = model.audio.bands(
        N_BANDS,
        30.0,
        10_000.0,
        0.0,
        model.controls.float("rise_rate"),
        model.controls.float("fall_rate"),
    );

    let cached_trig_fns = model.cached_trig_fns.clone();
    let animation = &model.animation;
    let controls = &model.controls;
    let band_for_freq = model.fft_bands[0];

    let distance_fn: CustomDistanceFn =
        Some(Arc::new(move |grid_point, position| {
            weave(
                grid_point.x,
                grid_point.y,
                position.x,
                position.y,
                if audio_enabled {
                    map_range(band_for_freq, 0.0, 1.0, weave_frequency, 0.01)
                } else {
                    weave_frequency
                },
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
        config.displacer.set_strength(if audio_enabled {
            map_range(model.fft_bands[3], 0.0, 1.0, 0.5, displacer_strength)
        } else {
            displacer_strength
        });
    }

    let enabled_displacer_configs: Vec<&DisplacerConfig> = model
        .displacer_configs
        .iter()
        .filter(|x| model.controls.bool(x.kind))
        .collect();

    let max_mag = model.displacer_configs.len() as f32 * displacer_strength;
    let gradient =
        find_palette_by_name(&controls.string("palette"), &model.palettes);
    let time = app.time;

    model.ellipses = model
        .grid
        .par_iter()
        .map(|point| {
            let mut total_displacement = vec2(0.0, 0.0);
            let mut total_influence = 0.0;
            let mut quad_contains = false;
            let mut current_qr_kind = "center";

            for config in &enabled_displacer_configs {
                if quad_restraint {
                    if QuadShape::from_str(&qr_shape).contains_point(
                        config.displacer.position * qr_pos,
                        vec2(
                            SKETCH_CONFIG.w as f32 / 3.0,
                            SKETCH_CONFIG.h as f32 / 3.0,
                        ) * qr_size,
                        *point,
                        time,
                    ) {
                        current_qr_kind = config.kind;
                        quad_contains = true;
                    }
                }
                let displacement = if config.kind == "center" {
                    if center_influence_or_attract {
                        config.displacer.attract(*point, scaling_power)
                    } else {
                        config.displacer.core_influence(*point, scaling_power)
                    }
                } else {
                    if quad_influence_or_attract {
                        config.displacer.attract(*point, scaling_power)
                    } else {
                        config.displacer.core_influence(*point, scaling_power)
                    }
                };
                let influence = displacement.length();
                total_displacement += displacement;
                total_influence += influence;
            }

            let mut blended_color = gradient.get(0.0);
            let inv_total = 1.0 / total_influence.max(1.0);

            for config in &enabled_displacer_configs {
                let displacement = if color_influence_or_attract {
                    config.displacer.attract(*point, scaling_power)
                } else {
                    config.displacer.core_influence(*point, scaling_power)
                };
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

            if quad_restraint && quad_contains {
                let color = if DEBUG_QUADS {
                    match current_qr_kind {
                        "quad_1" => RED,
                        "quad_2" => GREEN,
                        "quad_3" => BLUE,
                        "quad_4" => YELLOW,
                        _ => WHITE,
                    }
                    .into_lin_srgb()
                } else {
                    blended_color
                };
                let displaced_point = if audio_enabled {
                    let bass_to_qr_divisor = map_range(
                        model.fft_bands[4],
                        0.0,
                        1.0,
                        qr_divisor,
                        0.0,
                    );
                    *point + (total_displacement / bass_to_qr_divisor)
                } else {
                    *point + (total_displacement / qr_divisor)
                };
                (
                    displaced_point,
                    radius,
                    gradient.get(0.0).mix(&color, qr_lerp),
                )
            } else {
                (*point + total_displacement, radius, blended_color)
            }
        })
        .collect();
}

pub fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();

    frame.clear(BLACK);
    draw.background().color(rgba(0.1, 0.1, 0.1, 0.01));

    let scaled_draw = draw.scale(model.controls.float("scale"));

    for (position, radius, color) in &model.ellipses {
        scaled_draw
            .ellipse()
            .no_fill()
            .stroke(*color)
            .stroke_weight(0.5)
            .radius(*radius)
            .resolution(CIRCLE_RESOLUTION)
            .xy(*position);
    }

    draw.to_frame(app, &frame).unwrap();
}

type AnimationFn<R> = Option<
    Arc<dyn Fn(&Displacer, &Animation<Timing>, &Controls) -> R + Send + Sync>,
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
        "Custo" => (x.sin() * y.cos()) + (x - y).tanh() * (x + y).tan(),
        "Sprial" => {
            (x * y).sin() * (x - y).cos() + ((x * x + y * y).sqrt()).tanh()
        }
        "Ripple" => (x * x + y * y).sin() * (x - y).cos() + (x * y).tanh(),
        "VortxFld" => {
            let radius = (x * x + y * y).sqrt();
            let angle = x.atan2(y);
            (angle * radius.sin()).cos() * radius.sin() + (x * y).cos()
        }
        "VortxFld2" => {
            let radius = (x * x + y * y).sqrt();
            let angle = (-x).atan2(-y);
            (angle * radius.sin()).cos() * radius.sin() + (x * y).cos()
        }
        "FracRipl" => (x * x - y * y).sin() + (2.0 * x * y).cos(),
        "Quantum" => x.sin() * y.cos() * (y.sin() * x.cos()).tanh(),
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
    let functions = vec![
        "cos", "sin", "tan", "tanh", "sec", "csc", "cot", "sech", "csch",
        "coth",
    ];

    functions
        .iter()
        .flat_map(|a| functions.iter().map(move |b| format!("{},{}", a, b)))
        .chain(str_vec![
            "Custo",
            "Sprial",
            "Ripple",
            "VortxFld",
            "VortxFld2",
            "FracRipl",
            "Quantum",
        ])
        .collect()
}

fn find_palette_by_name<'a>(
    name: &str,
    palettes: &'a Vec<Gradient<LinSrgb>>,
) -> &'a Gradient<LinSrgb> {
    match name {
        "lightcoral" => &palettes[0],
        "lightgreen" => &palettes[1],
        _ => panic!("No palette named {}", name),
    }
}

fn animation_fns(position_animations_kind: &str) -> Vec<AnimationFn<Vec2>> {
    match position_animations_kind {
        "Counter Clockwise" => animations_counter_clockwise(),
        _ => animations_none(),
    }
}

fn animations_none() -> Vec<AnimationFn<Vec2>> {
    let w = SKETCH_CONFIG.w as f32;
    let h = SKETCH_CONFIG.h as f32;

    vec![
        None,
        Some(Arc::new(move |_, _, _| vec2(w / 4.0, h / 4.0))),
        Some(Arc::new(move |_, _, _| vec2(w / 4.0, -h / 4.0))),
        Some(Arc::new(move |_, _, _| vec2(-w / 4.0, -h / 4.0))),
        Some(Arc::new(move |_, _, _| vec2(-w / 4.0, h / 4.0))),
    ]
}

fn animations_counter_clockwise() -> Vec<AnimationFn<Vec2>> {
    const BEATS: f32 = 8.0;
    vec![
        None,
        Some(Arc::new(move |_displacer, ax, _controls| {
            let w = SKETCH_CONFIG.w as f32;
            let h = SKETCH_CONFIG.h as f32;
            let xp = w / 4.0;
            let yp = h / 4.0;
            let x = ax.lerp(
                &[
                    kf(xp, BEATS),  // Start at right
                    kf(-xp, BEATS), // Move to left
                    kf(-xp, BEATS), // Stay at left
                    kf(xp, BEATS),  // Move to right
                    kf(xp, 0.0),    // Complete the cycle
                ],
                0.0,
            );
            let y = ax.lerp(
                &[
                    kf(yp, BEATS),  // Start at top
                    kf(yp, BEATS),  // Stay at top
                    kf(-yp, BEATS), // Move to bottom
                    kf(-yp, BEATS), // Stay at bottom
                    kf(yp, 0.0),    // Move back to top
                ],
                0.0,
            );
            vec2(x, y)
        })),
        Some(Arc::new(move |_displacer, ax, _controls| {
            let w = SKETCH_CONFIG.w as f32;
            let h = SKETCH_CONFIG.h as f32;
            let xp = w / 4.0;
            let yp = -h / 4.0;
            let x = ax.lerp(
                &[
                    kf(xp, BEATS),  // Start at right
                    kf(xp, BEATS),  // Stay at right
                    kf(-xp, BEATS), // Move to left
                    kf(-xp, BEATS), // Stay at left
                    kf(xp, 0.0),    // Complete the cycle
                ],
                0.0,
            );
            let y = ax.lerp(
                &[
                    kf(yp, BEATS),  // Start at bottom
                    kf(-yp, BEATS), // Move to top
                    kf(-yp, BEATS), // Stay at top
                    kf(yp, BEATS),  // Move at bottom
                    kf(yp, 0.0),    // Complete the cycle
                ],
                0.0,
            );
            vec2(x, y)
        })),
        Some(Arc::new(move |_displacer, ax, _controls| {
            let w = SKETCH_CONFIG.w as f32;
            let h = SKETCH_CONFIG.h as f32;
            let xp = -w / 4.0;
            let yp = -h / 4.0;
            let x = ax.lerp(
                &[
                    kf(xp, BEATS),  // Start at left
                    kf(-xp, BEATS), // Move to right
                    kf(-xp, BEATS), // Stay to right
                    kf(xp, BEATS),  // Move to left
                    kf(xp, 0.0),    // Complete the cycle
                ],
                0.0,
            );
            let y = ax.lerp(
                &[
                    kf(yp, BEATS),  // Start at bottom
                    kf(yp, BEATS),  // Move to top
                    kf(-yp, BEATS), // Stay at top
                    kf(-yp, BEATS), // Move to bottom
                    kf(yp, 0.0),    // Complete the cycle
                ],
                0.0,
            );
            vec2(x, y)
        })),
        Some(Arc::new(move |_displacer, ax, _controls| {
            let w = SKETCH_CONFIG.w as f32;
            let h = SKETCH_CONFIG.h as f32;
            let xp = -w / 4.0;
            let yp = h / 4.0;
            let x = ax.lerp(
                &[
                    kf(xp, BEATS),  // Start at left
                    kf(xp, BEATS),  // Stay at left
                    kf(-xp, BEATS), // Move to right
                    kf(-xp, BEATS), // Stay at right
                    kf(xp, 0.0),    // Complete the cycle
                ],
                0.0,
            );
            let y = ax.lerp(
                &[
                    kf(yp, BEATS),  // Start at top
                    kf(-yp, BEATS), // Move to bottom
                    kf(-yp, BEATS), // Stay at bottom
                    kf(yp, BEATS),  // Move to top
                    kf(yp, 0.0),    // Complete the cycle
                ],
                0.0,
            );
            vec2(x, y)
        })),
    ]
}

#[derive(Clone, Copy, PartialEq)]
pub enum QuadShape {
    Rectangle,
    Circle,
    Ripple,
    Spiral,
}

impl QuadShape {
    pub fn from_str(s: &str) -> Self {
        match s {
            "Rectangle" => QuadShape::Rectangle,
            "Circle" => QuadShape::Circle,
            "Ripple" => QuadShape::Ripple,
            "Spiral" => QuadShape::Spiral,
            _ => QuadShape::Rectangle,
        }
    }

    pub fn contains_point(
        &self,
        center: Vec2,
        size: Vec2,
        point: Vec2,
        time: f32,
    ) -> bool {
        match self {
            QuadShape::Rectangle => {
                let rect = Rect::from_xy_wh(center, size);
                rect_contains_point(&rect, &point)
            }
            QuadShape::Circle => {
                let rect = Rect::from_xy_wh(center, size);
                circle_contains_point(&Ellipse::new(rect, 24.0), &point)
            }
            QuadShape::Ripple => {
                let dx = (point.x - center.x) / size.x;
                let dy = (point.y - center.y) / size.y;
                let distance = (dx * dx + dy * dy).sqrt();
                let wave = (distance * 10.0 + time).sin() * 0.2;
                distance < 1.0 + wave
            }
            QuadShape::Spiral => {
                let dx = (point.x - center.x) / size.x;
                let dy = (point.y - center.y) / size.y;
                let distance = (dx * dx + dy * dy).sqrt();
                let angle = dy.atan2(dx);
                let spiral = (angle * 3.0 + time + distance * 4.0).sin() * 0.2;
                distance < 1.0 + spiral
            }
        }
    }
}
