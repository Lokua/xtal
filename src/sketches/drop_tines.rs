use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

use super::shared::drop::Drop;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "drop_tines",
    display_name: "DropTines",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(200),
};

#[derive(SketchComponents)]
pub struct DropTines {
    hub: ControlHub<Timing>,
    drops: Vec<(Drop, Srgb<u8>)>,
}

const COLORS: &[Srgb<u8>] = &[BLACK, MISTYROSE, AZURE];

pub fn init(_app: &App, ctx: &LatticeContext) -> DropTines {
    let hub = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .slider("circle_count", 10.0, (1.0, 30.0), 1.0, None)
        .slider("resolution", 8.0, (3.0, 128.0), 1.0, None)
        .slider("radius", 20.0, (3.0, 100.0), 1.0, None)
        .separator()
        .slider("tine_count", 10.0, (1.0, 30.0), 1.0, None)
        .slider("displacement", 10.0, (0.0, 200.0), 0.1, None)
        .slider("falloff", 2.0, (0.0, 10.0), 0.01, None)
        .build();

    DropTines { hub, drops: vec![] }
}

impl Sketch for DropTines {
    fn update(&mut self, _app: &App, _update: Update, ctx: &LatticeContext) {
        let (w, _) = ctx.window_rect().wh();
        let circle_count = self.hub.get("circle_count") as usize;
        let resolution = self.hub.get("resolution") as usize;
        let radius = self.hub.get("radius");
        let tine_count = self.hub.get("tine_count") as usize;
        let displacement = self.hub.get("displacement");
        let falloff = self.hub.get("falloff");

        if self.hub.any_changed_in(&["circle_count"]) {
            self.drops.clear();
            for i in 0..circle_count {
                let drop = Drop::new(vec2(0.0, 0.0), radius, resolution);
                let color = COLORS[i % COLORS.len()];
                self.drops.push((drop, color));
            }
            self.hub.mark_unchanged();
        }

        for (i, (drop, _)) in &mut self.drops.iter_mut().enumerate() {
            drop.update(
                vec2(0.0, 0.0),
                map_range(
                    (circle_count - i) as f32 * radius,
                    0.0,
                    radius,
                    0.0,
                    radius,
                ),
                resolution,
            );
            for j in 0..tine_count {
                let x = j as f32 * (w / tine_count as f32);
                drop.tine(
                    vec2(x, 0.0),
                    vec2(
                        self.hub.animation.triangle(5.0, (-1.0, 1.0), 0.0),
                        self.hub.animation.triangle(12.0, (1.0, 1.0), 0.0),
                    ),
                    displacement,
                    falloff,
                );
                drop.tine(
                    vec2(x, 0.0),
                    vec2(
                        self.hub.animation.triangle(7.0, (-1.0, 1.0), 0.0),
                        self.hub.animation.triangle(8.0, (-1.0, 1.0), 0.0),
                    ),
                    displacement,
                    falloff,
                );
            }
        }
    }

    fn view(&self, app: &App, frame: Frame, ctx: &LatticeContext) {
        let wr = ctx.window_rect();
        let draw = app.draw();

        draw.rect()
            .x_y(0.0, 0.0)
            .w_h(wr.w(), wr.h())
            .hsla(0.0, 0.0, 0.02, 0.9);

        for (drop, color) in self.drops.iter() {
            draw.polygon()
                .stroke(*color)
                .stroke_weight(1.0)
                .no_fill()
                .points(drop.vertices().iter().cloned());
        }

        draw.to_frame(app, &frame).unwrap();
    }
}
