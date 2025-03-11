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
    controls: ControlHub<Timing>,
    max_drops: usize,
    drops: Vec<(Drop, Hsl)>,
    droppers: Vec<Dropper>,
}

pub fn init(_app: &App, ctx: &LatticeContext) -> Drops {
    let controls = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .slider("center_min_radius", 2.0, (1.0, 50.0), 1.0, None)
        .slider("center_max_radius", 20.0, (1.0, 50.0), 1.0, None)
        .slider("trbl_min_radius", 2.0, (1.0, 50.0), 1.0, None)
        .slider("trbl_max_radius", 20.0, (1.0, 50.0), 1.0, None)
        .slider("corner_min_radius", 2.0, (1.0, 50.0), 1.0, None)
        .slider("corner_max_radius", 20.0, (1.0, 50.0), 1.0, None)
        .slider("center_bw_ratio", 0.5, (0.0, 1.0), 0.001, None)
        .slider("trbl_bw_ratio", 0.5, (0.0, 1.0), 0.001, None)
        .slider("corner_bw_ratio", 0.5, (0.0, 1.0), 0.001, None)
        .build();

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
            controls.animation.create_trigger(duration, 0.0),
            DropZone::new(vec2(0.0, 0.0)),
            drop_it,
            controls.get("center_min_radius"),
            controls.get("center_max_radius"),
            center_color,
        ),
        // Top
        Dropper::new(
            "trbl".to_string(),
            controls.animation.create_trigger(duration, delay),
            DropZone::new(vec2(0.0, rect.top())),
            drop_it,
            controls.get("trbl_min_radius"),
            controls.get("trbl_max_radius"),
            trbl_color,
        ),
        Dropper::new(
            "corner".to_string(),
            controls.animation.create_trigger(duration, delay * 2.0),
            DropZone::new(rect.top_right()),
            drop_it,
            controls.get("corner_min_radius"),
            controls.get("corner_max_radius"),
            corner_color,
        ),
        // Right
        Dropper::new(
            "trbl".to_string(),
            controls.animation.create_trigger(duration, delay * 3.0),
            DropZone::new(vec2(rect.top(), 0.0)),
            drop_it,
            controls.get("trbl_min_radius"),
            controls.get("trbl_max_radius"),
            trbl_color,
        ),
        Dropper::new(
            "corner".to_string(),
            controls.animation.create_trigger(duration, delay * 4.0),
            DropZone::new(rect.bottom_right()),
            drop_it,
            controls.get("corner_min_radius"),
            controls.get("corner_max_radius"),
            corner_color,
        ),
        // Bottom
        Dropper::new(
            "trbl".to_string(),
            controls.animation.create_trigger(duration, delay * 5.0),
            DropZone::new(vec2(0.0, rect.bottom())),
            drop_it,
            controls.get("trbl_min_radius"),
            controls.get("trbl_max_radius"),
            trbl_color,
        ),
        Dropper::new(
            "corner".to_string(),
            controls.animation.create_trigger(duration, delay * 6.0),
            DropZone::new(rect.bottom_left()),
            drop_it,
            controls.get("corner_min_radius"),
            controls.get("corner_max_radius"),
            corner_color,
        ),
        // Left
        Dropper::new(
            "trbl".to_string(),
            controls.animation.create_trigger(duration, delay * 7.0),
            DropZone::new(vec2(rect.bottom(), 0.0)),
            drop_it,
            controls.get("trbl_min_radius"),
            controls.get("trbl_max_radius"),
            trbl_color,
        ),
        Dropper::new(
            "corner".to_string(),
            controls.animation.create_trigger(duration, delay * 8.0),
            DropZone::new(rect.top_left()),
            drop_it,
            controls.get("corner_min_radius"),
            controls.get("corner_max_radius"),
            corner_color,
        ),
    ];

    Drops {
        controls,
        max_drops: MAX_DROPS,
        drops: Vec::new(),
        droppers,
    }
}

impl Sketch for Drops {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &LatticeContext) {
        self.controls.update();

        let offset = self.controls.animation.automate(
            &[
                Breakpoint::ramp(0.0, 1.0, Easing::Linear),
                Breakpoint::ramp(2.0, 2.0, Easing::Linear),
                Breakpoint::ramp(4.0, 3.0, Easing::Linear),
                Breakpoint::end(6.0, 0.0),
            ],
            Mode::Loop,
        );

        self.droppers.iter_mut().for_each(|dropper| {
            if self.controls.animation.should_trigger(&mut dropper.trigger) {
                match dropper.kind.as_str() {
                    "trbl" => {
                        dropper.min_radius =
                            self.controls.get("trbl_min_radius");
                        dropper.max_radius =
                            self.controls.get("trbl_max_radius");
                    }
                    "corner" => {
                        dropper.min_radius =
                            self.controls.get("corner_min_radius");
                        dropper.max_radius =
                            self.controls.get("corner_max_radius");
                    }
                    "center" => {
                        dropper.min_radius =
                            self.controls.get("center_min_radius");
                        dropper.max_radius =
                            self.controls.get("center_max_radius");
                    }
                    _ => {}
                }

                for _ in 0..3 {
                    (dropper.drop_fn)(
                        dropper,
                        dropper.zone.center * offset,
                        &mut self.drops,
                        self.max_drops,
                        (dropper.color_fn)(&self.controls.ui_controls),
                    );
                }
            }
        });
    }

    fn view(&self, app: &App, frame: Frame, _ctx: &LatticeContext) {
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
type ColorFn = fn(&UiControls) -> Hsl;

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

fn center_color(controls: &UiControls) -> Hsl {
    if random_f32() > controls.get("center_bw_ratio") {
        hsl(0.0, 0.0, 0.0)
    } else {
        hsl(0.0, 0.0, 1.0)
    }
}

fn trbl_color(controls: &UiControls) -> Hsl {
    if random_f32() > controls.get("trbl_bw_ratio") {
        hsl(0.0, 0.0, 0.0)
    } else {
        hsl(0.0, 0.0, 1.0)
    }
}

fn corner_color(controls: &UiControls) -> Hsl {
    if random_f32() > controls.get("corner_bw_ratio") {
        hsl(0.0, 0.0, 0.0)
    } else {
        hsl(0.0, 0.0, 1.0)
    }
}
