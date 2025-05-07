use std::str::FromStr;

use nannou::color::*;
use nannou::prelude::*;

use xtal::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "easing_vis",
    display_name: "Easing",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
};

#[derive(SketchComponents)]
pub struct EasingVis {
    hub: ControlHub<Timing>,
    easing: Easing,
}

pub fn init(_app: &App, ctx: &Context) -> EasingVis {
    let hub = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .select(
            "easing",
            "linear",
            &Easing::FUNCTION_NAMES
                .iter()
                .copied()
                .filter(|s| *s != "custom")
                .collect::<Vec<&str>>(),
            None,
        )
        .slider(
            "exponent",
            2.0,
            (0.1, 20.0),
            0.1,
            Some(Box::new(|controls| {
                controls.string("easing") != "exponential"
            })),
        )
        .slider(
            "curve",
            0.2,
            (-1.0, 1.0),
            0.01,
            Some(Box::new(|controls| controls.string("easing") != "curve")),
        )
        .slider(
            "sigmoid_steepness",
            5.0,
            (0.01, 10.0),
            0.1,
            Some(Box::new(|controls| controls.string("easing") != "sigmoid")),
        )
        .separator()
        .slider_n("up_hue", 0.2937)
        .slider_n("down_hue", 0.1329)
        .build();

    let easing =
        Easing::from_str(&hub.string("easing")).unwrap_or(Easing::Linear);

    let easing = match easing {
        Easing::Exponential(_) => Easing::Exponential(hub.get("exponent")),
        Easing::Curve(..) => {
            Easing::Curve(hub.get("curve"), SUGGESTED_CURVE_MAX_EXPONENT)
        }
        Easing::Sigmoid(_) => Easing::Sigmoid(hub.get("sigmoid_steepness")),
        _ => easing,
    };

    EasingVis { hub, easing }
}

impl Sketch for EasingVis {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &Context) {
        if self.hub.changed() {
            let easing = Easing::from_str(&self.hub.string("easing"))
                .unwrap_or(Easing::Linear);

            self.easing = match easing {
                Easing::Exponential(_) => {
                    Easing::Exponential(self.hub.get("exponent"))
                }
                Easing::Curve(..) => Easing::Curve(
                    self.hub.get("curve"),
                    SUGGESTED_CURVE_MAX_EXPONENT,
                ),
                Easing::Sigmoid(_) => {
                    Easing::Sigmoid(self.hub.get("sigmoid_steepness"))
                }
                _ => easing,
            };

            self.hub.mark_unchanged();
        }
    }

    fn view(&self, app: &App, frame: Frame, ctx: &Context) {
        let draw = app.draw();
        ctx.background(&frame, &draw, hsl(0.0, 0.0, 0.02));

        let n_points = 100;
        let line_weight = 2.0;
        let wr = ctx.window_rect();
        let w = wr.w();
        let h = wr.h();

        let origin_x = -w / 2.0;
        let origin_y = -h / 2.0;

        let points: Vec<Point2> = (0..=n_points)
            .map(|i| {
                let t = i as f32 / n_points as f32;
                let eased_t = self.easing.apply(t);
                let x = origin_x + t * w;
                let y = origin_y + eased_t * h;
                vec2(x, y)
            })
            .collect();

        let points_down: Vec<Point2> = (0..=n_points)
            .map(|i| {
                let t = i as f32 / n_points as f32;
                let eased_t = self.easing.apply(t);
                let x = origin_x + t * w;
                let y = origin_y + h - eased_t * h;
                vec2(x, y)
            })
            .collect();

        draw.polyline()
            .weight(line_weight)
            .points(points)
            .color(hsl(self.hub.get("up_hue"), 0.5, 0.5));

        draw.polyline()
            .weight(line_weight)
            .points(points_down)
            .color(hsl(self.hub.get("down_hue"), 0.5, 0.5));

        draw.to_frame(app, &frame).unwrap();
    }
}
