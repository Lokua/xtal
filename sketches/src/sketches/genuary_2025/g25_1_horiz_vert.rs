use nannou::color::*;
use nannou::prelude::*;

use xtal::prelude::*;

// ~/Documents/Live/2024/2016 Begins

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "g25_1_horiz_vert",
    display_name: "Genuary 1: Vertical or horizontal lines only",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 127.0,
    w: 700,
    h: 700,
};

const N_LINES: u32 = 64;
const GRID_SIZE: u32 = 16;

#[derive(SketchComponents)]
#[sketch(clear_color = "hsla(0.0, 0.0, 0.0, 1.0)")]
pub struct HorizVert {
    controls: ControlHub<Timing>,
    lines: Vec<Vec2>,
}

pub fn init(_app: &App, ctx: &Context) -> HorizVert {
    let controls = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .checkbox("invert", false, None)
        .slider_n("a", 0.5)
        .slider_n("b", 0.5)
        .slider("aberration", 0.5, (0.0, 100.0), 1.0, None)
        .slider_n("background_alpha", 1.0)
        .build();

    HorizVert {
        controls,
        lines: vec![],
    }
}

impl Sketch for HorizVert {
    fn update(&mut self, _app: &App, _update: Update, ctx: &Context) {
        self.lines.clear();

        let window_rect = ctx.window_rect();
        let spacing = window_rect.w() / (N_LINES as f32 + 1.0);
        let b = self.controls.get("b") * 100.0;
        let line_length = (window_rect.h() / GRID_SIZE as f32) + b;
        let base_time = self.controls.animation.tri(4.0);
        let lrp_time = 4.0;

        for i in 0..N_LINES {
            let n_lines = N_LINES as f32;
            let i_f32 = i as f32;
            let x = -window_rect.hw() + spacing * (i as f32 + 1.0);
            let start = vec2(x, 0.0);

            let position_offset = i_f32 / n_lines * TAU * base_time * 2.0;

            let wave = self
                .controls
                .animation
                .triangle(lrp_time * 4.0, (-1.0, 1.0), 0.5)
                .sin();

            let envelope = (i_f32 / n_lines * TAU
                + base_time * 0.5
                + position_offset * 0.5)
                .cos();

            let end = vec2(x, line_length * wave * envelope);
            self.lines.push(start);
            self.lines.push(end);
        }
    }

    fn view(&self, app: &App, frame: Frame, ctx: &Context) {
        let wr = ctx.window_rect();
        let draw = app.draw();
        let window_rect = ctx.window_rect();
        let invert = self.controls.bool("invert");

        draw.rect().wh(window_rect.vec2()).color(hsla(
            0.0,
            0.0,
            if invert { 1.0 } else { 0.0 },
            self.controls.get("background_alpha"),
        ));

        let space = window_rect.h() / GRID_SIZE as f32;

        let mut draw2 = draw.translate(vec3(0.0, -wr.hh(), 0.0));
        for i in 0..GRID_SIZE {
            let j = i as f32;
            draw_lines(
                &draw2,
                self,
                wr.clone(),
                (j + 1.0) * (1.0 / GRID_SIZE as f32),
                hsla(0.0, 0.0, if invert { 0.0 } else { 1.0 }, 1.0),
            );
            draw2 = draw2.translate(vec3(0.0, space, 0.0));
        }

        let mut draw3 = draw.rotate(PI / 2.0);
        draw3 = draw3.translate(vec3(0.0, -window_rect.hh(), 0.0));
        for i in 0..GRID_SIZE {
            let j = i as f32;
            draw_lines(
                &draw3,
                self,
                wr.clone(),
                (j + 1.0) * (1.0 / GRID_SIZE as f32),
                hsla(0.0, 0.0, if invert { 0.0 } else { 1.0 }, 1.0),
            );
            draw3 = draw3.translate(vec3(0.0, space, 0.0));
        }

        draw.to_frame(app, &frame).unwrap();
    }
}

pub fn draw_lines(
    draw: &Draw,
    template: &HorizVert,
    wr: WindowRect,
    anim_delay: f32,
    _color: Hsla,
) {
    let time = 6.0;
    let aberration = template.controls.get("aberration");

    for chunk in template.lines.chunks(2) {
        if let [start, end] = chunk {
            let range = template.controls.animation.tri(32.0) * wr.hw();

            let animated_center = template.controls.animation.triangle(
                time * 2.0,
                (-range, range),
                anim_delay,
            );

            let distance_from_center =
                ((start.x - animated_center) / wr.hw()).abs();

            let min_weight = 0.1;
            let max_weight = 4.0;
            let weight =
                max_weight - (max_weight - min_weight) * distance_from_center;

            // Red channel (shifted right)
            draw.line()
                .start(vec2(start.x + aberration, start.y))
                .end(vec2(end.x + aberration, end.y))
                .weight(weight)
                .color(rgba(1.0, 0.0, 0.0, 0.8));

            // Green channel (no shift)
            draw.line()
                .start(*start)
                .end(*end)
                .weight(weight)
                .color(rgba(0.0, 1.0, 0.0, 0.8));

            // Blue channel (shifted left)
            draw.line()
                .start(vec2(start.x - aberration, start.y))
                .end(vec2(end.x - aberration, end.y))
                .weight(weight)
                .color(rgba(0.0, 0.0, 1.0, 0.8));
        }
    }
}
