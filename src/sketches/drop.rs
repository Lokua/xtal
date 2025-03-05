use nannou::prelude::*;

use super::shared::drop::*;
use crate::framework::prelude::*;

// https://www.youtube.com/watch?v=p7IGZTjC008&t=613s
// https://people.csail.mit.edu/jaffer/Marbling/Dropping-Paint

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "drop",
    display_name: "Drop",
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(400),
    play_mode: PlayMode::Loop,
};

const MAX_DROPS: usize = 2500;

#[derive(SketchComponents)]
pub struct Drops {
    animation: Animation<Timing>,
    controls: Controls,
    max_drops: usize,
    drops: Vec<(Drop, Hsl)>,
    droppers: Vec<Dropper>,
}

pub fn init(_app: &App, ctx: &LatticeContext) -> Drops {
    let animation = Animation::new(Timing::new(ctx.bpm()));
    let controls = Controls::new(vec![
        Control::slider("center_min_radius", 2.0, (1.0, 50.0), 1.0),
        Control::slider("center_max_radius", 20.0, (1.0, 50.0), 1.0),
        Control::slider("trbl_min_radius", 2.0, (1.0, 50.0), 1.0),
        Control::slider("trbl_max_radius", 20.0, (1.0, 50.0), 1.0),
        Control::slider("corner_min_radius", 2.0, (1.0, 50.0), 1.0),
        Control::slider("corner_max_radius", 20.0, (1.0, 50.0), 1.0),
        Control::slider("center_bw_ratio", 0.5, (0.0, 1.0), 0.001),
        Control::slider("trbl_bw_ratio", 0.5, (0.0, 1.0), 0.001),
        Control::slider("corner_bw_ratio", 0.5, (0.0, 1.0), 0.001),
    ]);

    let rect: Rect<f32> = Rect::from_x_y_w_h(
        0.0,
        0.0,
        SKETCH_CONFIG.w as f32 / 4.0,
        SKETCH_CONFIG.h as f32 / 4.0,
    );

    let duration = 1.0;
    let delay = duration / 9.0;

    let droppers = vec![
        Dropper::new(
            "center".to_string(),
            animation.create_trigger(duration, 0.0),
            DropZone::new(vec2(0.0, 0.0)),
            drop_it,
            controls.float("center_min_radius"),
            controls.float("center_max_radius"),
            center_color,
        ),
        // Top
        Dropper::new(
            "trbl".to_string(),
            animation.create_trigger(duration, delay),
            DropZone::new(vec2(0.0, rect.top())),
            drop_it,
            controls.float("trbl_min_radius"),
            controls.float("trbl_max_radius"),
            trbl_color,
        ),
        Dropper::new(
            "corner".to_string(),
            animation.create_trigger(duration, delay * 2.0),
            DropZone::new(rect.top_right()),
            drop_it,
            controls.float("corner_min_radius"),
            controls.float("corner_max_radius"),
            corner_color,
        ),
        // Right
        Dropper::new(
            "trbl".to_string(),
            animation.create_trigger(duration, delay * 3.0),
            DropZone::new(vec2(rect.top(), 0.0)),
            drop_it,
            controls.float("trbl_min_radius"),
            controls.float("trbl_max_radius"),
            trbl_color,
        ),
        Dropper::new(
            "corner".to_string(),
            animation.create_trigger(duration, delay * 4.0),
            DropZone::new(rect.bottom_right()),
            drop_it,
            controls.float("corner_min_radius"),
            controls.float("corner_max_radius"),
            corner_color,
        ),
        // Bottom
        Dropper::new(
            "trbl".to_string(),
            animation.create_trigger(duration, delay * 5.0),
            DropZone::new(vec2(0.0, rect.bottom())),
            drop_it,
            controls.float("trbl_min_radius"),
            controls.float("trbl_max_radius"),
            trbl_color,
        ),
        Dropper::new(
            "corner".to_string(),
            animation.create_trigger(duration, delay * 6.0),
            DropZone::new(rect.bottom_left()),
            drop_it,
            controls.float("corner_min_radius"),
            controls.float("corner_max_radius"),
            corner_color,
        ),
        // Left
        Dropper::new(
            "trbl".to_string(),
            animation.create_trigger(duration, delay * 7.0),
            DropZone::new(vec2(rect.bottom(), 0.0)),
            drop_it,
            controls.float("trbl_min_radius"),
            controls.float("trbl_max_radius"),
            trbl_color,
        ),
        Dropper::new(
            "corner".to_string(),
            animation.create_trigger(duration, delay * 8.0),
            DropZone::new(rect.top_left()),
            drop_it,
            controls.float("corner_min_radius"),
            controls.float("corner_max_radius"),
            corner_color,
        ),
    ];

    Drops {
        animation,
        controls,
        max_drops: MAX_DROPS,
        drops: Vec::new(),
        droppers,
    }
}

impl Sketch for Drops {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &LatticeContext) {
        let offset = self
            .animation
            .lerp(&[kf(1.0, 2.0), kf(2.0, 2.0), kf(3.0, 2.0)], 0.0);

        self.droppers.iter_mut().for_each(|dropper| {
            if self.animation.should_trigger(&mut dropper.trigger) {
                match dropper.kind.as_str() {
                    "trbl" => {
                        dropper.min_radius =
                            self.controls.float("trbl_min_radius");
                        dropper.max_radius =
                            self.controls.float("trbl_max_radius");
                    }
                    "corner" => {
                        dropper.min_radius =
                            self.controls.float("corner_min_radius");
                        dropper.max_radius =
                            self.controls.float("corner_max_radius");
                    }
                    "center" => {
                        dropper.min_radius =
                            self.controls.float("center_min_radius");
                        dropper.max_radius =
                            self.controls.float("center_max_radius");
                    }
                    _ => {}
                }

                for _ in 0..3 {
                    (dropper.drop_fn)(
                        dropper,
                        dropper.zone.center * offset,
                        &mut self.drops,
                        self.max_drops,
                        (dropper.color_fn)(&self.controls),
                    );
                }
            }
        });
    }

    fn view(&self, app: &App, frame: Frame, _ctx: &LatticeContext) {
        let _window_rect = app
            .window(frame.window_id())
            .expect("Unable to get window")
            .rect();

        let draw = app.draw();

        draw.background().color(hsl(0.0, 0.0, 1.0));

        for (drop, color) in self.drops.iter() {
            draw.polygon()
                .color(*color)
                .points(drop.vertices().iter().cloned());
        }

        draw.to_frame(app, &frame).unwrap();
    }
}

type DropFn = fn(&Dropper, Vec2, &mut Vec<(Drop, Hsl)>, usize, Hsl);
type ColorFn = fn(&Controls) -> Hsl;

struct Dropper {
    kind: String,
    trigger: Trigger,
    zone: DropZone,
    drop_fn: DropFn,
    min_radius: f32,
    max_radius: f32,
    color_fn: ColorFn,
}

impl Dropper {
    pub fn new(
        kind: String,
        trigger: Trigger,
        zone: DropZone,
        drop_fn: DropFn,
        min_radius: f32,
        max_radius: f32,
        color_fn: ColorFn,
    ) -> Self {
        Self {
            kind,
            trigger,
            zone,
            drop_fn,
            min_radius,
            max_radius,
            color_fn,
        }
    }
}

fn drop_it(
    dropper: &Dropper,
    point: Vec2,
    drops: &mut Vec<(Drop, Hsl)>,
    max_drops: usize,
    color: Hsl,
) {
    let point = nearby_point(point, 50.0);
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

fn center_color(controls: &Controls) -> Hsl {
    if random_f32() > controls.float("center_bw_ratio") {
        hsl(0.0, 0.0, 0.0)
    } else {
        hsl(0.0, 0.0, 1.0)
    }
}

fn trbl_color(controls: &Controls) -> Hsl {
    if random_f32() > controls.float("trbl_bw_ratio") {
        hsl(0.0, 0.0, 0.0)
    } else {
        hsl(0.0, 0.0, 1.0)
    }
}

fn corner_color(controls: &Controls) -> Hsl {
    if random_f32() > controls.float("corner_bw_ratio") {
        hsl(0.0, 0.0, 0.0)
    } else {
        hsl(0.0, 0.0, 1.0)
    }
}
