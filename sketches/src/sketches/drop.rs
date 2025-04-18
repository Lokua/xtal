use xtal::prelude::*;
use nannou::prelude::*;

use super::shared::drop::*;
use crate::util::*;

// https://www.youtube.com/watch?v=p7IGZTjC008&t=613s
// https://people.csail.mit.edu/jaffer/Marbling/Dropping-Paint

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "drop",
    display_name: "Drop",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
};

#[derive(SketchComponents)]
pub struct Drops {
    hub: ControlHub<Timing>,
    drops: Vec<(Drop, Hsl)>,
    droppers: Vec<Dropper>,
}

pub fn init(_app: &App, ctx: &Context) -> Drops {
    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "drop.yaml"),
        Timing::new(ctx.bpm()),
    );

    Drops {
        hub,
        drops: Vec::new(),
        droppers: Vec::new(),
    }
}

impl Sketch for Drops {
    fn update(&mut self, _app: &App, _update: Update, ctx: &Context) {
        let max_drops = self.hub.get("max_drops");
        let duration: f32 = self.hub.string("duration").parse().unwrap();
        let spread_div = self.hub.get("spread_div");
        let offset = self.hub.get("offset");

        if ctx.should_clear() {
            self.drops.clear();
        }

        if ctx.window_rect().changed()
            || self.hub.any_changed_in(&["duration", "spread_div"])
        {
            let (w, h) = ctx.window_rect().wh();
            let rect =
                Rect::from_x_y_w_h(0.0, 0.0, w / spread_div, h / spread_div);
            self.droppers = create_droppers(&self.hub, rect, duration);
            ctx.window_rect().mark_unchanged();
            self.hub.mark_unchanged();
        }

        self.droppers.iter_mut().for_each(|dropper| {
            if self.hub.animation.should_trigger(&mut dropper.trigger) {
                match dropper.kind {
                    DropperKind::Trbl => {
                        dropper.min_radius = self.hub.get("trbl_min_radius");
                        dropper.max_radius = self.hub.get("trbl_max_radius");
                    }
                    DropperKind::Corner => {
                        dropper.min_radius = self.hub.get("corner_min_radius");
                        dropper.max_radius = self.hub.get("corner_max_radius");
                    }
                    DropperKind::Center => {
                        dropper.min_radius = self.hub.get("center_min_radius");
                        dropper.max_radius = self.hub.get("center_max_radius");
                    }
                }

                for _ in 0..3 {
                    (dropper.drop_fn)(
                        dropper,
                        dropper.zone.center * offset,
                        &mut self.drops,
                        max_drops as usize,
                        (dropper.color_fn)(&self.hub),
                    );
                }
            }
        });
    }

    fn view(&self, app: &App, frame: Frame, _ctx: &Context) {
        let draw = app.draw();

        draw.background()
            .color(hsl(0.0, 0.0, 1.0 - self.hub.get("bg_invert")));

        for (drop, color) in self.drops.iter() {
            draw.polygon()
                .color(*color)
                .points(drop.vertices().iter().cloned());
        }

        draw.to_frame(app, &frame).unwrap();
    }
}

fn create_droppers(
    hub: &ControlHub<Timing>,
    rect: Rect<f32>,
    duration: f32,
) -> Vec<Dropper> {
    let n_droppers = 9.0;
    let delay = duration / n_droppers;

    vec![
        Dropper {
            kind: DropperKind::Center,
            trigger: hub.animation.create_trigger(duration, 0.0),
            zone: DropZone::new(vec2(0.0, 0.0)),
            drop_fn: drop_it,
            min_radius: hub.get("center_min_radius"),
            max_radius: hub.get("center_max_radius"),
            color_fn: center_color,
        },
        // Top
        Dropper {
            kind: DropperKind::Trbl,
            trigger: hub.animation.create_trigger(duration, delay),
            zone: DropZone::new(vec2(0.0, rect.top())),
            drop_fn: drop_it,
            min_radius: hub.get("trbl_min_radius"),
            max_radius: hub.get("trbl_max_radius"),
            color_fn: trbl_color,
        },
        Dropper {
            kind: DropperKind::Corner,
            trigger: hub.animation.create_trigger(duration, delay * 2.0),
            zone: DropZone::new(rect.top_right()),
            drop_fn: drop_it,
            min_radius: hub.get("corner_min_radius"),
            max_radius: hub.get("corner_max_radius"),
            color_fn: corner_color,
        },
        // Right
        Dropper {
            kind: DropperKind::Trbl,
            trigger: hub.animation.create_trigger(duration, delay * 3.0),
            zone: DropZone::new(vec2(rect.top(), 0.0)),
            drop_fn: drop_it,
            min_radius: hub.get("trbl_min_radius"),
            max_radius: hub.get("trbl_max_radius"),
            color_fn: trbl_color,
        },
        Dropper {
            kind: DropperKind::Corner,
            trigger: hub.animation.create_trigger(duration, delay * 4.0),
            zone: DropZone::new(rect.bottom_right()),
            drop_fn: drop_it,
            min_radius: hub.get("corner_min_radius"),
            max_radius: hub.get("corner_max_radius"),
            color_fn: corner_color,
        },
        // Bottom
        Dropper {
            kind: DropperKind::Trbl,
            trigger: hub.animation.create_trigger(duration, delay * 5.0),
            zone: DropZone::new(vec2(0.0, rect.bottom())),
            drop_fn: drop_it,
            min_radius: hub.get("trbl_min_radius"),
            max_radius: hub.get("trbl_max_radius"),
            color_fn: trbl_color,
        },
        Dropper {
            kind: DropperKind::Corner,
            trigger: hub.animation.create_trigger(duration, delay * 6.0),
            zone: DropZone::new(rect.bottom_left()),
            drop_fn: drop_it,
            min_radius: hub.get("corner_min_radius"),
            max_radius: hub.get("corner_max_radius"),
            color_fn: corner_color,
        },
        // Left
        Dropper {
            kind: DropperKind::Trbl,
            trigger: hub.animation.create_trigger(duration, delay * 7.0),
            zone: DropZone::new(vec2(rect.bottom(), 0.0)),
            drop_fn: drop_it,
            min_radius: hub.get("trbl_min_radius"),
            max_radius: hub.get("trbl_max_radius"),
            color_fn: trbl_color,
        },
        Dropper {
            kind: DropperKind::Corner,
            trigger: hub.animation.create_trigger(duration, delay * 8.0),
            zone: DropZone::new(rect.top_left()),
            drop_fn: drop_it,
            min_radius: hub.get("corner_min_radius"),
            max_radius: hub.get("corner_max_radius"),
            color_fn: corner_color,
        },
    ]
}

type DropFn = fn(&Dropper, Vec2, &mut Vec<(Drop, Hsl)>, usize, Hsl);
type ColorFn = fn(&ControlHub<Timing>) -> Hsl;

enum DropperKind {
    Center,
    Corner,
    Trbl,
}

struct Dropper {
    kind: DropperKind,
    trigger: Trigger,
    zone: DropZone,
    drop_fn: DropFn,
    min_radius: f32,
    max_radius: f32,
    color_fn: ColorFn,
}

fn drop_it(
    dropper: &Dropper,
    point: Vec2,
    drops: &mut Vec<(Drop, Hsl)>,
    max_drops: usize,
    color: Hsl,
) {
    let point = nearby_point(point, 50.0);
    let (min, max) = safe_range(dropper.min_radius, dropper.max_radius);
    let drop = Drop::new(point, random_range(min, max), 64);

    for (other, _color) in drops.iter_mut() {
        drop.marble(other);
    }

    drops.push((drop, color));

    while drops.len() > max_drops {
        drops.remove(0);
    }
}

fn get_color(hub: &ControlHub<Timing>, ratio: &str) -> Hsl {
    let invert_l = random_f32() <= hub.get(ratio);

    if hub.bool("colorize") {
        let h = ternary!(hub.bool("randomize_h"), random(), hub.get("h"));
        let s = ternary!(hub.bool("randomize_s"), random(), hub.get("s"));
        let l = ternary!(hub.bool("randomize_l"), random(), hub.get("l"));
        let l = ternary!(invert_l, 1.0 - l, l);
        hsl(h, s, l)
    } else {
        let l = ternary!(invert_l, 1.0, 0.0);
        hsl(0.0, 0.0, l)
    }
}

fn center_color(hub: &ControlHub<Timing>) -> Hsl {
    get_color(hub, "center_bw_ratio")
}

fn trbl_color(hub: &ControlHub<Timing>) -> Hsl {
    get_color(hub, "trbl_bw_ratio")
}

fn corner_color(hub: &ControlHub<Timing>) -> Hsl {
    get_color(hub, "corner_bw_ratio")
}
