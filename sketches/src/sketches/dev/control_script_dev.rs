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
    controls: ControlHub<Timing>,
}

pub fn init(_app: &App, ctx: &Context) -> ControlScriptDev {
    let controls = ControlHub::from_path(
        to_absolute_path(file!(), "control_script_dev.yaml"),
        Timing::new(ctx.bpm()),
    );

    ControlScriptDev { controls }
}

impl Sketch for ControlScriptDev {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &Context) {}

    fn view(&self, app: &App, frame: Frame, ctx: &Context) {
        let wr = ctx.window_rect();
        let draw = app.draw();

        ctx.background(
            &frame,
            &draw,
            hsla(0.0, 0.0, 0.02, self.controls.get("bg_alpha")),
        );

        if self.controls.bool("show_center_circle") {
            draw.ellipse()
                .color(hsl(self.controls.get("center_hue"), 0.5, 0.5))
                .radius(self.controls.get("center_radius"))
                .x_y(0.0, 0.0);
        }

        if self.controls.bool("show_white_circle") {
            draw.ellipse()
                .color(WHITE)
                .radius(self.controls.get("white_radius"))
                .x_y(
                    self.controls.get("white_pos_x") * wr.hw(),
                    self.controls.get("white_pos_y") * wr.hh(),
                );
        }

        if self.controls.bool("show_audio") {
            draw.rect()
                .color(CYAN)
                .x_y(
                    0.0,
                    map_range(
                        self.controls.get("audio_rect_y"),
                        0.0,
                        1.0,
                        -wr.hh(),
                        wr.hh(),
                    ),
                )
                .w_h(wr.w() - 100.0, 30.0);
        }

        if self.controls.bool("show_breakpoints") {
            draw.rect()
                .color(MAGENTA)
                .x_y(
                    0.0,
                    map_range(
                        self.controls.get("breakpoints_line"),
                        0.0,
                        1.0,
                        -wr.hh(),
                        wr.hh(),
                    ),
                )
                .w_h(wr.w(), 20.0);
        }

        if self.controls.bool("show_red_circle") {
            draw.ellipse()
                .color(RED)
                .radius(self.controls.get("red_circle_radius"))
                .x_y(
                    self.controls.get("red_circle_pos_x") * wr.hw(),
                    -wr.h() / 4.0,
                );
        }

        if self.controls.bool("show_midi_circle") {
            draw.ellipse()
                .color(YELLOW)
                .radius(self.controls.get("midi_radius"))
                .x_y(0.0, 0.0);
        }

        if self.controls.bool("show_random_section") {
            let size = self.controls.get("random_size");
            let size_slewed = self.controls.get("random_size_slewed");
            draw.rect().color(BLACK).w_h(size, size).x_y(-wr.qw(), 0.0);
            draw.rect()
                .color(BLACK)
                .w_h(size_slewed, size_slewed)
                .x_y(wr.qw(), 0.0);
        }

        if self.controls.bool("show_rm") {
            draw.rect()
                .color(LIMEGREEN)
                .x_y(
                    0.0,
                    map_range(
                        self.controls.get("rm_a"),
                        0.0,
                        1.0,
                        -wr.hh(),
                        wr.hh(),
                    ),
                )
                .w_h(wr.w() - 100.0, 30.0);
        }

        draw.to_frame(app, &frame).unwrap();
    }
}
