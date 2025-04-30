use nannou::color::*;
use nannou::prelude::*;

use xtal::prelude::*;

// Live/2025/Xtal - ControlScript Test

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "control_script_dev",
    display_name: "ControlScript Test",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 500,
    h: 500,
};

#[derive(SketchComponents)]
pub struct ControlScriptDev {
    hub: ControlHub<Timing>,
}

pub fn init(_app: &App, ctx: &Context) -> ControlScriptDev {
    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "control_script_dev.yaml"),
        Timing::new(ctx.bpm()),
    );

    ControlScriptDev { hub }
}

impl Sketch for ControlScriptDev {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &Context) {}

    fn view(&self, app: &App, frame: Frame, ctx: &Context) {
        let wr = ctx.window_rect();
        let draw = app.draw();

        ctx.background(
            &frame,
            &draw,
            hsla(0.0, 0.0, 0.02, self.hub.get("bg_alpha")),
        );

        if self.hub.bool("show_center_circle") {
            draw.ellipse()
                .color(hsl(self.hub.get("center_hue"), 0.5, 0.5))
                .radius(self.hub.get("center_radius"))
                .x_y(0.0, 0.0);
        }

        if self.hub.bool("show_white_circle") {
            draw.ellipse()
                .color(WHITE)
                .radius(self.hub.get("white_radius"))
                .x_y(
                    self.hub.get("white_pos_x") * wr.hw(),
                    self.hub.get("white_pos_y") * wr.hh(),
                );
        }

        if self.hub.bool("show_audio") {
            draw.rect()
                .color(CYAN)
                .x_y(
                    0.0,
                    map_range(
                        self.hub.get("audio_rect_y"),
                        0.0,
                        1.0,
                        -wr.hh(),
                        wr.hh(),
                    ),
                )
                .w_h(wr.w() - 100.0, 30.0);
        }

        if self.hub.bool("show_breakpoints") {
            draw.rect()
                .color(MAGENTA)
                .x_y(
                    0.0,
                    map_range(
                        self.hub.get("breakpoints_line"),
                        0.0,
                        1.0,
                        -wr.hh(),
                        wr.hh(),
                    ),
                )
                .w_h(wr.w(), 20.0);
        }

        if self.hub.bool("show_red_circle") {
            draw.ellipse()
                .color(RED)
                .radius(self.hub.get("red_circle_radius"))
                .x_y(self.hub.get("red_circle_pos_x") * wr.hw(), -wr.h() / 4.0);
        }

        if self.hub.bool("show_midi_circle") {
            draw.ellipse()
                .color(YELLOW)
                .radius(self.hub.get("midi_radius"))
                .x_y(0.0, 0.0);
        }

        if self.hub.bool("show_random_section") {
            let size = self.hub.get("random_size");
            let size_slewed = self.hub.get("random_size_slewed");
            draw.rect().color(BLACK).w_h(size, size).x_y(-wr.qw(), 0.0);
            draw.rect()
                .color(BLACK)
                .w_h(size_slewed, size_slewed)
                .x_y(wr.qw(), 0.0);
        }

        if self.hub.bool("show_rm") {
            draw.rect()
                .color(LIMEGREEN)
                .x_y(
                    0.0,
                    map_range(
                        self.hub.get("rm_a"),
                        0.0,
                        1.0,
                        -wr.hh(),
                        wr.hh(),
                    ),
                )
                .w_h(wr.w() - 100.0, 30.0);
        }

        if self.hub.bool("show_ramp") {
            draw.rect()
                .color(ORANGE)
                .x_y(
                    -wr.hw(),
                    map_range(
                        self.hub.get("ramp"),
                        0.0,
                        1.0,
                        -wr.hh(),
                        wr.hh(),
                    ),
                )
                .w_h(100.0, 100.0);
        }

        draw.to_frame(app, &frame).unwrap();
    }
}
