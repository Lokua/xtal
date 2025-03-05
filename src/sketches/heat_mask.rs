use nannou::color::Gradient;
use nannou::prelude::*;
use rayon::prelude::*;
use std::sync::Arc;

use super::shared::displacer::*;
use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "heat_mask",
    display_name: "Heat Mask",
    fps: 60.0,
    bpm: 134.0,
    w: 1000,
    h: 1000,
    gui_w: None,
    gui_h: Some(920),
    play_mode: PlayMode::Loop,
};

const GRID_SIZE: usize = 128;

#[derive(SketchComponents)]
#[sketch(clear_color = "hsl(0.0, 0.0, 0.03, 0.5)")]
pub struct HeatMask {
    grid: Vec<Vec2>,
    displacer_configs: Vec<DisplacerConfig>,
    animation: Animation<Timing>,
    controls: Controls,
    gradient: Gradient<LinSrgb>,
    objects: Vec<(Vec2, f32, f32, LinSrgb)>,
}

pub fn init(_app: &App, ctx: LatticeContext) -> HeatMask {
    let wr = ctx.window_rect();
    let grid_w = wr.w() - 80.0;
    let grid_h = wr.h() - 80.0;
    let animation = Animation::new(Timing::new(ctx.bpm));

    let modes = ["attract", "influence"];

    let disable_center_controls =
        |controls: &Controls| controls.bool("animate_center");
    let disable_corner_controls =
        |controls: &Controls| controls.bool("animate_corner");
    let disable_trbl_controls =
        |controls: &Controls| controls.bool("animate_trbl");

    let controls = Controls::new(vec![
        Control::checkbox("show_center", false),
        Control::checkbox("animate_center", false),
        Control::checkbox("center_use_grain", true),
        Control::select("center_mode", "attract", &modes),
        Control::slider_x(
            "center_radius",
            20.0,
            (1.0, 500.0),
            1.0,
            disable_center_controls,
        ),
        Control::slider_x(
            "center_strength",
            20.0,
            (1.0, 500.0),
            1.0,
            disable_center_controls,
        ),
        Control::Separator {},
        Control::checkbox("show_corner", true),
        Control::checkbox("animate_corner", false),
        Control::checkbox("corner_use_grain", true),
        Control::select("corner_mode", "attract", &modes),
        Control::slider_x(
            "corner_radius",
            20.0,
            (1.0, 500.0),
            1.0,
            disable_corner_controls,
        ),
        Control::slider_x(
            "corner_strength",
            20.0,
            (1.0, 500.0),
            1.0,
            disable_corner_controls,
        ),
        Control::Separator {},
        Control::checkbox("show_trbl", true),
        Control::checkbox("animate_trbl", false),
        Control::checkbox("trbl_use_grain", true),
        Control::select("trbl_mode", "attract", &modes),
        Control::slider_x(
            "trbl_radius",
            20.0,
            (1.0, 500.0),
            1.0,
            disable_trbl_controls,
        ),
        Control::slider_x(
            "trbl_strength",
            20.0,
            (1.0, 500.0),
            1.0,
            disable_trbl_controls,
        ),
        Control::Separator {},
        Control::slider("scale", 1.0, (0.1, 4.0), 0.1),
        Control::checkbox("flip", false),
        Control::select(
            "sort",
            "radius",
            &["luminance", "radius", "radius_reversed"],
        ),
        Control::checkbox("stroke", true),
        Control::slider_x(
            "stroke_weight",
            1.25,
            (0.25, 3.0),
            0.25,
            |controls| !controls.bool("stroke"),
        ),
        Control::Separator {},
        Control::checkbox("invert_colors", false),
        Control::slider("gradient_spread", 1.0, (0.0, 1.0), 0.0001),
        Control::slider("background_alpha", 1.0, (0.000, 1.0), 0.001),
        Control::slider("alpha", 1.0, (0.001, 1.0), 0.001),
        Control::Separator {},
        Control::slider("size_max", 7.3, (0.1, 20.0), 0.1),
        Control::slider("t_scale", 1.0, (1.0, 200.0), 1.0),
        Control::slider("scaling_power", 3.0, (0.5, 11.0), 0.25),
        Control::slider("mag_mult", 1.0, (1.0, 200.0), 1.0),
        Control::Separator {},
        Control::slider("grain_size", 101.0, (1.0, 200.0), 1.0),
        Control::slider("angle_mult", 48.0, (1.0, 200.0), 1.0),
        Control::slider("distance_strength", 0.5, (0.001, 1.0), 0.001),
        Control::slider("angle_frequency", 5.0, (5.0, 500.0), 5.0),
    ]);

    let mut displacer_configs = vec![
        DisplacerConfig::new_no_anim(
            DisplacerConfigKind::Center,
            Displacer::new_at_origin(),
        ),
        DisplacerConfig::new_no_anim(
            DisplacerConfigKind::Corner,
            Displacer::new_at_origin(),
        ),
        DisplacerConfig::new_no_anim(
            DisplacerConfigKind::Corner,
            Displacer::new_at_origin(),
        ),
        DisplacerConfig::new_no_anim(
            DisplacerConfigKind::Corner,
            Displacer::new_at_origin(),
        ),
        DisplacerConfig::new_no_anim(
            DisplacerConfigKind::Corner,
            Displacer::new_at_origin(),
        ),
        DisplacerConfig::new_no_anim(
            DisplacerConfigKind::Trbl,
            Displacer::new_at_origin(),
        ),
        DisplacerConfig::new_no_anim(
            DisplacerConfigKind::Trbl,
            Displacer::new_at_origin(),
        ),
        DisplacerConfig::new_no_anim(
            DisplacerConfigKind::Trbl,
            Displacer::new_at_origin(),
        ),
        DisplacerConfig::new_no_anim(
            DisplacerConfigKind::Trbl,
            Displacer::new_at_origin(),
        ),
    ];

    update_positions(&wr, &mut displacer_configs);

    HeatMask {
        grid: create_grid(grid_w, grid_h, GRID_SIZE, vec2).0,
        displacer_configs,
        animation,
        controls,
        gradient: Gradient::new(vec![
            CYAN.into_lin_srgb(),
            ORANGE.into_lin_srgb(),
            MAGENTA.into_lin_srgb(),
            GREEN.into_lin_srgb(),
        ]),
        objects: Vec::with_capacity(GRID_SIZE * GRID_SIZE),
    }
}

impl Sketch for HeatMask {
    fn update(&mut self, _app: &App, _update: Update, ctx: &LatticeContext) {
        let mut wr = ctx.window_rect();

        let show_center = self.controls.bool("show_center");
        let animate_center = self.controls.bool("animate_center");
        let center_use_grain = self.controls.bool("center_use_grain");
        let center_mode = self.controls.string("center_mode");
        let center_radius = self.controls.float("center_radius");
        let center_strength = self.controls.float("center_strength");
        // ---
        let show_corner = self.controls.bool("show_corner");
        let animate_corner = self.controls.bool("animate_corner");
        let corner_use_grain = self.controls.bool("corner_use_grain");
        let corner_mode = self.controls.string("corner_mode");
        let corner_radius = self.controls.float("corner_radius");
        let corner_strength = self.controls.float("corner_strength");
        // ---
        let show_trbl = self.controls.bool("show_trbl");
        let animate_trbl = self.controls.bool("animate_trbl");
        let trbl_use_grain = self.controls.bool("trbl_use_grain");
        let trbl_mode = self.controls.string("trbl_mode");
        let trbl_radius = self.controls.float("trbl_radius");
        let trbl_strength = self.controls.float("trbl_strength");
        // ---
        let sort = self.controls.string("sort");
        let size_max = self.controls.float("size_max");
        let invert_colors = self.controls.bool("invert_colors");
        let gradient_spread = self.controls.float("gradient_spread");
        let scaling_power = self.controls.float("scaling_power");
        let t_scale = self.controls.float("t_scale");
        let grain_size = self.controls.float("grain_size");
        let angle_mult = self.controls.float("angle_mult");
        let distance_strength = self.controls.float("distance_strength");
        let angle_frequency = self.controls.float("angle_frequency");

        let max_mag = self.displacer_configs.len() as f32
            * self.controls.float("mag_mult");
        let gradient = &self.gradient;

        if wr.changed() {
            (self.grid, _) = create_grid(wr.w(), wr.h(), GRID_SIZE, vec2);
            update_positions(&wr, &mut self.displacer_configs);
            wr.mark_unchanged();
        }

        let custom_distance_fn: CustomDistanceFn =
            Some(Arc::new(move |grid_point, position| {
                wood_grain_advanced(
                    grid_point.x,
                    grid_point.y,
                    position.x,
                    position.y,
                    grain_size,
                    angle_mult,
                    2.0,
                    distance_strength,
                    angle_frequency,
                )
            }));

        let configs: Vec<&mut DisplacerConfig> = self
            .displacer_configs
            .iter_mut()
            .filter(|config| match config.kind {
                DisplacerConfigKind::Center => show_center,
                DisplacerConfigKind::Corner => show_corner,
                DisplacerConfigKind::Trbl => show_trbl,
            })
            .map(|config| {
                config.update(&self.animation, &self.controls);
                match config.kind {
                    DisplacerConfigKind::Center => {
                        config.displacer.set_custom_distance_fn(
                            if center_use_grain {
                                custom_distance_fn.clone()
                            } else {
                                None
                            },
                        );
                        if animate_center {
                            config.displacer.set_strength(
                                self.animation.r_ramp(
                                    &[kfr((1.0, 500.0), 8.0)],
                                    0.0,
                                    4.0,
                                    Easing::Linear,
                                ),
                            );
                            config.displacer.set_radius(self.animation.r_ramp(
                                &[kfr((1.0, 500.0), 12.0)],
                                1.0,
                                3.0,
                                Easing::Linear,
                            ));
                        } else {
                            config.displacer.set_strength(center_radius);
                            config.displacer.set_radius(center_strength);
                        }
                    }
                    DisplacerConfigKind::Corner => {
                        config.displacer.set_custom_distance_fn(
                            if corner_use_grain {
                                custom_distance_fn.clone()
                            } else {
                                None
                            },
                        );
                        if animate_corner {
                            config.displacer.set_strength(
                                self.animation.r_ramp(
                                    &[kfr((1.0, 500.0), 4.0)],
                                    0.0,
                                    4.0,
                                    Easing::Linear,
                                ),
                            );
                            config.displacer.set_radius(self.animation.r_ramp(
                                &[kfr((1.0, 500.0), 8.0)],
                                1.0,
                                3.0,
                                Easing::Linear,
                            ));
                        } else {
                            config.displacer.set_strength(corner_radius);
                            config.displacer.set_radius(corner_strength);
                        }
                    }
                    DisplacerConfigKind::Trbl => {
                        config.displacer.set_custom_distance_fn(
                            if trbl_use_grain {
                                custom_distance_fn.clone()
                            } else {
                                None
                            },
                        );
                        if animate_trbl {
                            config.displacer.set_strength(
                                self.animation.r_ramp(
                                    &[kfr((1.0, 500.0), 16.0)],
                                    0.0,
                                    6.0,
                                    Easing::Linear,
                                ),
                            );
                            config.displacer.set_radius(self.animation.r_ramp(
                                &[kfr((1.0, 500.0), 24.0)],
                                2.0,
                                18.0,
                                Easing::Linear,
                            ));
                        } else {
                            config.displacer.set_strength(trbl_radius);
                            config.displacer.set_radius(trbl_strength);
                        }
                    }
                }
                config
            })
            .collect();

        self.objects = self
            .grid
            .par_iter()
            .map(|point| {
                let total_displacement =
                    configs.iter().fold(vec2(0.0, 0.0), |acc, config| {
                        let mode = match config.kind {
                            DisplacerConfigKind::Center => &center_mode,
                            DisplacerConfigKind::Corner => &corner_mode,
                            DisplacerConfigKind::Trbl => &trbl_mode,
                        };

                        acc + if mode == "attract" {
                            config.displacer.attract(*point, scaling_power)
                        } else {
                            config
                                .displacer
                                .core_influence(*point, scaling_power)
                        }
                    });

                let displacement_magnitude = total_displacement.length();

                let triangle_height = map_range(
                    displacement_magnitude,
                    0.0,
                    max_mag,
                    1.0,
                    t_scale,
                );

                let normalized = (displacement_magnitude / max_mag)
                    .powf(gradient_spread)
                    .clamp(0.0, 1.0);

                let color = gradient.get(if invert_colors {
                    normalized
                } else {
                    1.0 - normalized
                });

                assert!(
                    displacement_magnitude.is_finite(),
                    "displacement_magnitude is not finite: {:?}",
                    displacement_magnitude
                );

                let radius = map_clamp(
                    displacement_magnitude,
                    0.0,
                    max_mag,
                    0.1,
                    size_max,
                    ease_out_quad,
                );

                (*point + total_displacement, radius, triangle_height, color)
            })
            .collect();

        self.objects.sort_by(
            |(_position_a, radius_a, _triangle_height_a, color_a),
             (_position_b, radius_b, _triangle_height_b, color_b)| {
                match sort.as_str() {
                    "radius" => radius_a.partial_cmp(radius_b).unwrap(),
                    "radius_reversed" => {
                        radius_b.partial_cmp(radius_a).unwrap()
                    }
                    _ => luminance(color_a)
                        .partial_cmp(&luminance(color_b))
                        .unwrap(),
                }
            },
        );
    }

    fn view(&self, app: &App, frame: Frame, ctx: &LatticeContext) {
        let draw = app.draw();
        let wr = ctx.window_rect();

        draw.rect().w_h(wr.w(), wr.h()).color(hsla(
            0.0,
            0.0,
            0.03,
            self.controls.float("background_alpha"),
        ));

        let scaled_draw = draw.scale(self.controls.float("scale"));

        let alpha = self.controls.float("alpha");
        let stroke = self.controls.bool("stroke");
        let stroke_weight = self.controls.float("stroke_weight");
        let flip = self.controls.bool("flip");

        for (position, radius, triangle_height, color) in &self.objects {
            // Calculate outward direction from center to position
            let outward_dir = if position.length_squared() == 0.0 {
                vec2(1.0, 0.0)
            } else {
                (*position - vec2(0.0, 0.0)).normalize()
            };

            let radius = radius.max(0.01);

            // Distance from the center to the tip
            let height = radius;
            let base_width = radius;

            // Calculate vertices
            let tip = *position
                + outward_dir
                    * height
                    * *triangle_height
                    * if flip { -1.0 } else { 1.0 };

            // Perpendicular vector for the base
            let perpendicular = vec2(-outward_dir.y, outward_dir.x);

            let base_left = *position - perpendicular * (base_width / 2.0);
            let base_right = *position + perpendicular * (base_width / 2.0);

            scaled_draw
                .polygon()
                .stroke(if stroke {
                    hsla(0.0, 0.0, 0.0, 1.0)
                } else {
                    hsla(0.0, 0.0, 0.0, 0.0)
                })
                .stroke_weight(stroke_weight)
                .points(vec![tip, base_left, base_right])
                .color(lin_srgb_to_lin_srgba(*color, alpha));
        }

        draw.to_frame(app, &frame).unwrap();
    }
}

type AnimationFn<R> = Option<
    Arc<dyn Fn(&Displacer, &Animation<Timing>, &Controls) -> R + Send + Sync>,
>;

enum DisplacerConfigKind {
    Center,
    Corner,
    Trbl,
}

struct DisplacerConfig {
    #[allow(dead_code)]
    kind: DisplacerConfigKind,
    displacer: Displacer,
    position_animation: AnimationFn<Vec2>,
    radius_animation: AnimationFn<f32>,
}

impl DisplacerConfig {
    pub fn new(
        kind: DisplacerConfigKind,
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

    pub fn new_no_anim(
        kind: DisplacerConfigKind,
        displacer: Displacer,
    ) -> Self {
        Self::new(kind, displacer, None, None)
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

fn update_positions(
    wr: &WindowRect,
    displacer_configs: &mut Vec<DisplacerConfig>,
) {
    let w = wr.w() / 4.0;
    let h = wr.w() / 4.0;

    for (index, config) in displacer_configs.iter_mut().enumerate() {
        config.displacer.set_position(match index {
            1 => vec2(w, h),
            // Corner Q2
            2 => vec2(w, -h),
            // Corner Q3
            3 => vec2(-w, -h),
            // Corner Q4
            4 => vec2(-w, h),
            // Top
            5 => vec2(0.0, h),
            // Right
            6 => vec2(w, 0.0),
            // Bottom
            7 => vec2(0.0, -h),
            // Left
            8 => vec2(-w, 0.0),
            // Center
            _ => vec2(0.0, 0.0),
        });
    }
}
