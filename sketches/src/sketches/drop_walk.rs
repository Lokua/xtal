use nannou::prelude::*;
use xtal::prelude::*;

use super::shared::drop::*;
use crate::util::*;

// https://www.youtube.com/watch?v=p7IGZTjC008&t=613s
// https://people.csail.mit.edu/jaffer/Marbling/Dropping-Paint

// ~/Documents/Live/2024/2024.02.19 Dumb Out Project/04 - Dumb Out - Video Promo Edit.als

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "drop_walk",
    display_name: "Drop Walk",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
};

const MAX_DROPS: usize = 5000;

#[derive(SketchComponents)]
pub struct Drops {
    controls: ControlHub<Timing>,
    max_drops: usize,
    drops: Vec<(Drop, Hsl)>,
    droppers: Vec<Dropper>,
    palettes: Vec<(ColorFn, ColorFn, ColorFn)>,
}

pub fn init(_app: &App, ctx: &Context) -> Drops {
    let w = ctx.window_rect().w();
    let h = ctx.window_rect().h();

    let controls = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .checkbox("debug_walker", false, None)
        .slider("n_drops", 1.0, (1.0, 20.0), 1.0, None)
        .slider("step_size", 2.0, (1.0, 200.0), 1.0, None)
        .slider("splatter_radius", 2.0, (1.0, 200.0), 1.0, None)
        .slider("drop_max_radius", 20.0, (1.0, 50.0), 1.0, None)
        .separator()
        .select("palette", "millennial", &["millennial", "gen_x"], None)
        .slider_n("color_ratio", 0.5)
        .slider_n("lightness_min", 0.0)
        .slider_n("lightness_max", 1.0)
        .build();

    let step_size = controls.get("step_size");
    let drop_max_radius = controls.get("drop_max_radius");

    let droppers = vec![
        Dropper::new(
            "center".to_string(),
            controls.animation.create_trigger(0.5, 0.0),
            Dropper::drop_it,
            1.0,
            drop_max_radius,
            Walker {
                w,
                h,
                step_size,
                ..Walker::default()
            },
        ),
        Dropper::new(
            "center".to_string(),
            controls.animation.create_trigger(1.0, 0.0),
            Dropper::drop_it,
            1.0,
            drop_max_radius,
            Walker {
                w,
                h,
                step_size,
                ..Walker::default()
            },
        ),
        Dropper::new(
            "center".to_string(),
            controls.animation.create_trigger(1.5, 0.0),
            Dropper::drop_it,
            1.0,
            drop_max_radius,
            Walker {
                w,
                h,
                step_size,
                ..Walker::default()
            },
        ),
    ];

    Drops {
        controls,
        max_drops: MAX_DROPS,
        drops: Vec::new(),
        droppers,
        palettes: vec![
            (
                |controls: &UiControls| gen_color(controls, (0.38, 0.47)),
                |controls: &UiControls| gen_color(controls, (0.8, 0.9)),
                |controls: &UiControls| gen_color(controls, (0.6, 0.65)),
            ),
            (
                |controls: &UiControls| gen_color(controls, (0.0, 0.05)),
                |controls: &UiControls| gen_color(controls, (0.28, 0.33)),
                |controls: &UiControls| gen_color(controls, (0.63, 0.68)),
            ),
        ],
    }
}

impl Sketch for Drops {
    fn update(&mut self, _app: &App, _update: Update, ctx: &Context) {
        let step_size = self.controls.get("step_size");
        let drop_max_radius = self.controls.get("drop_max_radius");
        let splatter_radius = self.controls.get("splatter_radius");
        let n_drops = self.controls.get("n_drops");
        let palette_name = self.controls.string("palette");
        let palette = palette_by_name(&self.palettes, &palette_name);
        let wr = ctx.window_rect();

        self.droppers
            .iter_mut()
            .enumerate()
            .for_each(|(index, dropper)| {
                if self.controls.animation.should_trigger(&mut dropper.trigger)
                {
                    dropper.walker.step_size = step_size;
                    dropper.walker.w = wr.w();
                    dropper.walker.h = wr.h();

                    let color_fn = match index % 3 {
                        0 => palette.0,
                        1 => palette.1,
                        2 => palette.2,
                        _ => unreachable!(),
                    };

                    dropper.walker.step();

                    if dropper.kind.as_str() == "center" {
                        dropper.min_radius = 0.1;
                        dropper.max_radius = drop_max_radius;
                    }

                    for _ in 0..n_drops as i32 {
                        (dropper.drop_fn)(
                            dropper,
                            dropper.walker.to_vec2(),
                            &mut self.drops,
                            self.max_drops,
                            (color_fn)(&self.controls.ui_controls),
                            splatter_radius,
                        );
                    }
                }
            });
    }

    fn view(&self, app: &App, frame: Frame, _ctx: &Context) {
        let draw = app.draw();

        draw.background().color(hsl(0.0, 0.0, 1.0));

        for (drop, color) in self.drops.iter() {
            draw.polygon()
                .color(*color)
                .points(drop.vertices().iter().cloned());
        }

        if self.controls.bool("debug_walker") {
            for (index, dropper) in self.droppers.iter().enumerate() {
                draw.ellipse()
                    .color(match index {
                        0 => RED,
                        1 => GREEN,
                        _ => BLUE,
                    })
                    .stroke(BLACK)
                    .stroke_weight(4.0)
                    .xy(dropper.walker.to_vec2())
                    .radius(20.0);
            }
        }

        draw.to_frame(app, &frame).unwrap();
    }
}

type DropFn = fn(&Dropper, Vec2, &mut Vec<(Drop, Hsl)>, usize, Hsl, f32);
type ColorFn = fn(&UiControls) -> Hsl;

struct Dropper {
    kind: String,
    trigger: Trigger,
    drop_fn: DropFn,
    min_radius: f32,
    max_radius: f32,
    walker: Walker,
}

impl Dropper {
    pub fn new(
        kind: String,
        trigger: Trigger,
        drop_fn: DropFn,
        min_radius: f32,
        max_radius: f32,
        walker: Walker,
    ) -> Self {
        Self {
            kind,
            trigger,
            drop_fn,
            min_radius,
            max_radius,
            walker,
        }
    }

    pub fn drop_it(
        dropper: &Dropper,
        point: Vec2,
        drops: &mut Vec<(Drop, Hsl)>,
        max_drops: usize,
        color: Hsl,
        splatter_radius: f32,
    ) {
        let point = nearby_point(point, splatter_radius);
        let drop = Drop::new(
            point,
            random_range(dropper.min_radius, dropper.max_radius),
            64,
        );

        for (other, _color) in drops.iter_mut() {
            drop.marble(other);
        }

        drops.push((drop, color));

        while drops.len() > max_drops {
            drops.remove(0);
        }
    }
}

struct Walker {
    w: f32,
    h: f32,
    x: f32,
    y: f32,
    step_size: f32,
}

impl Walker {
    pub fn step(&mut self) {
        match random_range(0.0, 4.0).floor() as u32 {
            0 => {
                self.x += self.step_size;
            }
            1 => {
                self.x -= self.step_size;
            }
            2 => {
                self.y += self.step_size;
            }
            _ => {
                self.y -= self.step_size;
            }
        }

        self.constrain();
    }

    fn constrain(&mut self) {
        let half_w = self.w / 2.0;
        let half_h = self.h / 2.0;

        if self.x < -half_w {
            self.x += (self.step_size * 2.0).min(half_w);
        }
        if self.x > half_w {
            self.x -= (self.step_size * 2.0).min(half_w);
        }
        if self.y < -half_h {
            self.y += (self.step_size * 2.0).min(half_h);
        }
        if self.y > half_h {
            self.y -= (self.step_size * 2.0).min(half_h);
        }
    }

    pub fn to_vec2(&self) -> Vec2 {
        vec2(self.x, self.y)
    }
}

impl Default for Walker {
    fn default() -> Self {
        Self {
            w: 0.0,
            h: 0.0,
            x: 0.0,
            y: 0.0,
            step_size: 1.0,
        }
    }
}

fn palette_by_name(
    palettes: &[(ColorFn, ColorFn, ColorFn)],
    name: &str,
) -> (ColorFn, ColorFn, ColorFn) {
    let index = match name {
        "millennial" => 0,
        "gen_x" => 1,
        _ => 0,
    };
    palettes[index]
}

fn gen_color(controls: &UiControls, hue_range: (f32, f32)) -> Hsl {
    let (min, max) = safe_range(
        controls.get("lightness_min"),
        controls.get("lightness_max"),
    );
    if random_f32() > controls.get("color_ratio") {
        hsl(
            random_range(hue_range.0, hue_range.1),
            0.9,
            random_range(min, max),
        )
    } else {
        hsl(0.0, 0.0, 1.0)
    }
}
