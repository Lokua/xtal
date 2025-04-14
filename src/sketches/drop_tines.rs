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
#[sketch(clear_color = "hsla(0.0, 0.0, 0.02, 1.0)")]
pub struct DropTines {
    hub: ControlHub<Timing>,
    drops: Vec<(Drop, Srgb<u8>)>,
}

const COLORS: &[Srgb<u8>] = &[BLACK, MISTYROSE, AZURE];

pub fn init(_app: &App, ctx: &Context) -> DropTines {
    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "drop_tines.yaml"),
        Timing::new(ctx.bpm()),
    );

    DropTines { hub, drops: vec![] }
}

impl Sketch for DropTines {
    fn update(&mut self, _app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();
        let circle_count = self.hub.get("circle_count") as usize;
        let resolution = self.hub.get("resolution") as usize;
        let radius = self.hub.get("radius");
        let tine_count = self.hub.select(
            "animate_tine_count",
            "tine_count_animation",
            "tine_count",
        ) as usize;
        let displacement = self.hub.get("displacement");
        let falloff = self.hub.get("falloff");
        let dir_x =
            self.hub.select("animate_dir_x", "dir_x_animation", "dir_x");
        let dir_y =
            self.hub.select("animate_dir_y", "dir_y_animation", "dir_y");

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
                let x = map_range(
                    j as f32,
                    0.0,
                    tine_count as f32 - 1.0,
                    -wr.hw(),
                    wr.hw(),
                );
                drop.tine(
                    vec2(x, 0.0),
                    vec2(dir_x, dir_y),
                    displacement,
                    falloff,
                );
            }
        }
    }

    fn view(&self, app: &App, frame: Frame, ctx: &Context) {
        let wr = ctx.window_rect();
        let draw = app.draw();

        draw.rect().x_y(0.0, 0.0).w_h(wr.w(), wr.h()).hsla(
            0.0,
            0.0,
            0.02,
            self.hub.get("bg_alpha"),
        );

        let draws = &[
            draw.translate(vec3(0.0, 0.0, 0.0)),
            // draw.translate(vec3(-wr.h() / 4.0, 0.0, 0.0)),
            // draw.translate(vec3(wr.h() / 4.0, 0.0, 0.0)),
        ];

        for draw in draws {
            for (drop, color) in self.drops.iter() {
                draw.polygon()
                    .stroke(*color)
                    .stroke_weight(self.hub.get("stroke_weight"))
                    .no_fill()
                    .points(drop.vertices().iter().cloned());
            }
        }

        draw.to_frame(app, &frame).unwrap();
    }
}
