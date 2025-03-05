use nannou::prelude::*;

use crate::framework::prelude::*;

// https://www.generativehut.com/post/how-to-make-generative-art-feel-natural

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "lines",
    display_name: "Lines",
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(230),
    play_mode: PlayMode::Loop,
};

const N_LINES: i32 = 4;
const STROKE_WEIGHT: f32 = 4.0;
const SPACING: f32 = 32.0;

#[derive(SketchComponents)]
pub struct Lines {
    controls: Controls,
    slant_points: Vec<(Vec2, Vec2)>,
    jerky_points: Vec<Vec<Vec2>>,
    chaikin_points: Vec<Vec<Vec2>>,
    kernel_points: Vec<Vec<Vec2>>,
    pad: f32,
}

pub fn init(_app: &App, ctx: &LatticeContext) -> Lines {
    let wr = ctx.window_rect();

    let controls = Controls::with_previous(vec![
        Control::slider("deviation", 5.0, (1.0, 10.0), 0.1),
        Control::slider("n_points", 16.0, (3.0, 64.0), 1.0),
        Control::slider("chaikin_passes", 4.0, (1.0, 16.0), 1.0),
        Control::slider("kernel_passes", 2.0, (1.0, 16.0), 1.0),
    ]);

    let pad = wr.w() / 20.0;

    Lines {
        controls,
        slant_points: vec![],
        jerky_points: vec![],
        chaikin_points: vec![],
        kernel_points: vec![],
        pad,
    }
}

impl Sketch for Lines {
    fn update(&mut self, _app: &App, _update: Update, ctx: &LatticeContext) {
        if self.controls.changed() {
            let deviation = self.controls.float("deviation");
            let n_points = self.controls.float("n_points") as usize;
            let chaikin_passes = self.controls.float("chaikin_passes") as usize;
            let kernel_passes = self.controls.float("kernel_passes") as usize;
            let wr = &ctx.window_rect();
            let params = &LineParams {
                pad: self.pad,
                deviation,
                n_points,
                chaikin_passes,
                kernel_passes,
            };
            self.slant_points = generate_slant_points(wr, params);
            self.jerky_points = generate_jerky_points(wr, params);
            self.chaikin_points =
                generate_points_using_chaikin_smoothing(wr, params);
            self.kernel_points =
                generate_points_using_kernel_smoothing(wr, params);

            self.controls.mark_unchanged();
        }
    }

    fn view(&self, app: &App, frame: Frame, ctx: &LatticeContext) {
        let draw = app.draw();
        let wr = &ctx.window_rect();

        draw.rect()
            .x_y(0.0, 0.0)
            .w_h(wr.w(), wr.h())
            .hsla(0.0, 0.0, 1.0, 1.0);

        let start_x = -wr.hw() + self.pad;
        let end_x = wr.hw() - self.pad;
        let n_demos = 5;

        for demo in 0..n_demos {
            let y = wr.top() - (wr.h() / n_demos as f32) * (demo as f32 + 0.5);
            let draw_shifted = draw.translate(vec3(0.0, y, 0.0));

            match demo {
                0 => {
                    draw_shifted
                        .line()
                        .start(vec2(start_x, 0.0))
                        .end(vec2(end_x, 0.0))
                        .color(BLACK)
                        .stroke_weight(STROKE_WEIGHT);
                }
                1 => {
                    for (start, end) in self.slant_points.iter() {
                        draw_shifted
                            .line()
                            .start(*start)
                            .end(*end)
                            .color(BLACK)
                            .stroke_weight(STROKE_WEIGHT);
                    }
                }
                2 => {
                    for line in self.jerky_points.iter() {
                        draw_shifted
                            .polyline()
                            .weight(STROKE_WEIGHT)
                            .points(line.iter().cloned())
                            .color(BLACK);
                    }
                }
                3 => {
                    for line in self.chaikin_points.iter() {
                        draw_shifted
                            .polyline()
                            .weight(STROKE_WEIGHT)
                            .points(line.iter().cloned())
                            .color(BLACK);
                    }
                }
                4 => {
                    for line in self.kernel_points.iter() {
                        draw_shifted
                            .polyline()
                            .weight(STROKE_WEIGHT)
                            .points(line.iter().cloned())
                            .color(BLACK);
                    }
                }
                _ => unreachable!(),
            }
        }

        draw.to_frame(app, &frame).unwrap();
    }
}

struct LineParams {
    pad: f32,
    deviation: f32,
    n_points: usize,
    chaikin_passes: usize,
    kernel_passes: usize,
}

fn generate_slant_points(
    wr: &WindowRect,
    params: &LineParams,
) -> Vec<(Vec2, Vec2)> {
    let start_x = -wr.hw() + params.pad;
    let end_x = wr.hw() - params.pad;
    let mut points = vec![];

    for i in 0..N_LINES {
        let base_y = i as f32 * wr.h() / SPACING;
        let offset_start_y = random_normal(params.deviation);
        let offset_end_y = random_normal(params.deviation);
        let start = vec2(start_x, base_y + offset_start_y);
        let end = vec2(end_x, base_y + offset_end_y);
        points.push((start, end));
    }

    points
}

fn generate_jerky_points(
    wr: &WindowRect,
    params: &LineParams,
) -> Vec<Vec<Vec2>> {
    let start_x = -wr.hw() + params.pad;
    let end_x = wr.hw() - params.pad;
    let mut lines = vec![];
    let segment_length = (end_x - start_x) / params.n_points as f32;

    for i in 0..N_LINES {
        let mut points = vec![];
        let base_y = i as f32 * wr.h() / SPACING;

        for j in 0..=params.n_points {
            let x = start_x + (j as f32 * segment_length);
            let offset_start_y = random_normal(params.deviation);
            let y = base_y + offset_start_y;
            points.push(pt2(x, y));
        }

        lines.push(points);
    }

    lines
}

fn generate_points_using_chaikin_smoothing(
    wr: &WindowRect,
    params: &LineParams,
) -> Vec<Vec<Vec2>> {
    let start_x = -wr.hw() + params.pad;
    let end_x = wr.hw() - params.pad;
    let mut lines = vec![];
    let segment_length = (end_x - start_x) / params.n_points as f32;

    for i in 0..N_LINES {
        let mut points = vec![];
        let base_y = i as f32 * wr.h() / SPACING;

        for j in 0..=params.n_points {
            let x = start_x + (j as f32 * segment_length);
            let offset_start_y = random_normal(params.deviation);
            let y = base_y + offset_start_y;
            points.push(pt2(x, y));
        }

        let smoothed = chaikin(points, params.chaikin_passes, false);
        lines.push(smoothed);
    }

    lines
}

fn generate_points_using_kernel_smoothing(
    wr: &WindowRect,
    params: &LineParams,
) -> Vec<Vec<Vec2>> {
    let start_x = -wr.hw() + params.pad;
    let end_x = wr.hw() - params.pad;
    let mut lines = vec![];
    let segment_length = (end_x - start_x) / params.n_points as f32;

    for i in 0..N_LINES {
        let mut points = vec![];
        let base_y = i as f32 * wr.h() / SPACING;

        for j in 0..=params.n_points {
            let x = start_x + (j as f32 * segment_length);
            let offset_start_y = random_normal(params.deviation);
            let y = base_y + offset_start_y;
            points.push(pt2(x, y));
        }

        points = average_neighbors(points, params.kernel_passes);

        lines.push(points);
    }

    lines
}
