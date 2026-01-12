use nannou::color::*;
use nannou::prelude::*;
use nannou::text::Font;

use xtal::prelude::*;

const SCROLL_RATE: f32 = 0.5;
const LINE_RATE: f32 = 4.0;
const SATURATION_RATE: f32 = 4.0;
const LIGHTNESS_RATE: f32 = 8.0;
const CHAR_WIDTH: f32 = 8.0;
const MIN_SPACE: f32 = 8.0;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "g26_11_quine",
    display_name: "Genuary 2026:11: Quine",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 127.0,
    w: 700,
    h: 1244,
};

#[derive(SketchComponents)]
pub struct Quine {
    hub: ControlHub<Timing>,
    lines: Vec<String>,
    font: Font,
    scroll_offset: f32,
    // Store smoothed values for horizontal and space animations
    smoothed_x_offsets: Vec<f32>,
    smoothed_space_counts: Vec<f32>,
}

pub fn init(_app: &App, ctx: &Context) -> Quine {
    let hub = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .build();

    // Get absolute path to this source file
    let path = to_absolute_path(file!(), "g26_11_quine.rs");
    // Read and filter non-empty lines from source file
    let lines: Vec<String> = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| String::from("Could not read file"))
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|s| s.to_string())
        .collect();

    // Load font from bytes at compile time
    let font = Font::from_bytes(include_bytes!(
        "/Users/lokua/Library/Fonts/FiraCode-Regular.ttf"
    ))
    .unwrap();

    // Precompute initial smoothed values to avoid lunge
    let max_spaces = [1, 2, 3];
    let phase_offsets = [0.25, 0.5, 0.75];
    let num_lines = lines.len();
    let mut smoothed_x_offsets = vec![0.0; num_lines];
    let mut smoothed_space_counts = vec![0.0; num_lines];
    for idx in 0..num_lines {
        let line = &lines[idx];
        let trimmed = line.trim();
        let word_count = trimmed.split_whitespace().count();
        let has_word_spaces = word_count > 1;
        let max_offset = max_spaces[idx % max_spaces.len()];
        let phase_offset = phase_offsets[idx % phase_offsets.len()];
        if has_word_spaces {
            let target_space_count = hub.animation.triangle(
                LINE_RATE,
                (1.0, max_offset as f32),
                phase_offset,
            );
            smoothed_space_counts[idx] = target_space_count;
        } else {
            let target_x_offset = hub.animation.triangle(
                LINE_RATE,
                (0.0, max_offset as f32 * 8.0),
                phase_offset,
            );
            smoothed_x_offsets[idx] = target_x_offset;
        }
    }
    Quine {
        hub,
        lines,
        font,
        scroll_offset: 0.0,
        smoothed_x_offsets,
        smoothed_space_counts,
    }
}

impl Sketch for Quine {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &Context) {
        self.hub.update();

        if SCROLL_RATE > 0.0 {
            // Animate scroll offset based on beats and scroll rate
            let current_offset = self.hub.animation.beats() / SCROLL_RATE;
            let max_offset = self.lines.len() as f32;
            self.scroll_offset = current_offset % max_offset;
        }
        // Smoothing for horizontal and space animations
        let max_spaces = [2, 3, 4, 5];
        let phase_offsets = [0.0, 0.3, 0.6, 0.9, 0.2, 0.5, 0.8, 0.1, 0.4, 0.7];
        let num_lines = self.lines.len();
        // Ensure vectors are correct length
        if self.smoothed_x_offsets.len() != num_lines {
            self.smoothed_x_offsets = vec![0.0; num_lines];
        }
        if self.smoothed_space_counts.len() != num_lines {
            self.smoothed_space_counts = vec![0.0; num_lines];
        }
        // Smoothing factor (0.2 = fast, 0.05 = slow)
        let alpha = 0.15;
        for idx in 0..num_lines {
            let line = &self.lines[idx];
            let trimmed = line.trim();
            let word_count = trimmed.split_whitespace().count();
            let has_word_spaces = word_count > 1;
            let max_offset = max_spaces[idx % max_spaces.len()];
            let phase_offset = phase_offsets[idx % phase_offsets.len()];
            if has_word_spaces {
                let target_space_count = self.hub.animation.triangle(
                    LINE_RATE,
                    (1.0, max_offset as f32),
                    phase_offset,
                );
                self.smoothed_space_counts[idx] =
                    self.smoothed_space_counts[idx] * (1.0 - alpha)
                        + target_space_count * alpha;
                self.smoothed_x_offsets[idx] = 0.0;
            } else {
                let target_x_offset = self.hub.animation.triangle(
                    LINE_RATE,
                    (0.0, max_offset as f32 * 8.0),
                    phase_offset,
                );
                self.smoothed_x_offsets[idx] = self.smoothed_x_offsets[idx]
                    * (1.0 - alpha)
                    + target_x_offset * alpha;
                self.smoothed_space_counts[idx] = 0.0;
            }
        }
    }

    fn view(&self, app: &App, frame: Frame, ctx: &Context) {
        let wr = ctx.window_rect();
        let draw = app.draw();

        draw.rect()
            .x_y(0.0, 0.0)
            .w_h(wr.w(), wr.h())
            .color(hsl(0.0, 0.0, 0.02));

        let start_x = wr.left() + 110.0;
        let start_y = wr.top() - 20.0;
        let line_height = 16.0;

        let palette = [
            hsl(90.0 / 360.0, 0.25, 0.65),
            hsl(120.0 / 360.0, 0.22, 0.60),
            hsl(60.0 / 360.0, 0.20, 0.70),
            hsl(210.0 / 360.0, 0.18, 0.65),
            hsl(30.0 / 360.0, 0.20, 0.68),
            hsl(150.0 / 360.0, 0.20, 0.60),
            hsl(270.0 / 360.0, 0.15, 0.60),
        ];

        let num_lines = self.lines.len();
        let visible_lines =
            ((wr.h() / line_height).ceil() as usize).min(num_lines);
        for i in 0..visible_lines {
            // Wrap line index for scrolling
            let line_index =
                ((i as f32 + self.scroll_offset).floor() as usize) % num_lines;
            let line = &self.lines[line_index];
            // Animate vertical position with fractional scroll
            let y = start_y
                - ((i as f32 - (self.scroll_offset.fract())) * line_height);

            let trimmed = line.trim();
            let word_count = trimmed.split_whitespace().count();
            let has_word_spaces = word_count > 1;
            let is_comment = trimmed.starts_with("//");

            let base_color = palette[line_index % palette.len()];

            let Hsl {
                hue,
                saturation,
                lightness,
                ..
            } = base_color;
            // Convert hue to [0,1] for hsl()
            let (h, mut s, mut l) =
                (hue.to_degrees() / 360.0, saturation, lightness);

            if !is_comment {
                // Animate saturation
                let sat_anim = if SATURATION_RATE > 0.0 {
                    self.hub.animation.triangle(
                        SATURATION_RATE,
                        (-0.08, 0.08),
                        i as f32 / self.lines.len() as f32 * 0.5,
                    )
                } else {
                    0.0
                };
                // Animate lightness
                let light_anim = if LIGHTNESS_RATE > 0.0 {
                    self.hub.animation.triangle(
                        LIGHTNESS_RATE,
                        (-0.10, 0.10),
                        i as f32 / self.lines.len() as f32 * 0.7,
                    )
                } else {
                    0.0
                };
                s = (s + sat_anim).clamp(0.0, 1.0);
                l = (l + light_anim).clamp(0.0, 1.0);
            }

            let color = if is_comment {
                rgb(0.3, 0.3, 0.3).into()
            } else {
                hsl(h, s, l).into()
            };

            if has_word_spaces && !is_comment {
                let space_width =
                    self.smoothed_space_counts[line_index] * CHAR_WIDTH;
                draw_words_with_variable_space(
                    &draw,
                    &self.font,
                    start_x,
                    y,
                    line,
                    space_width.max(MIN_SPACE),
                    color,
                );
                continue;
            }

            let x_pos = if !is_comment {
                let x_offset = self.smoothed_x_offsets[line_index].floor();
                start_x - x_offset
            } else {
                start_x
            };

            draw.text(line)
                .color(color)
                .font_size(12)
                .font(self.font.clone())
                .no_line_wrap()
                .left_justify()
                .x(x_pos)
                .y(y);
        }

        draw.to_frame(app, &frame).unwrap();
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_words_with_variable_space(
    draw: &nannou::draw::Draw,
    font: &Font,
    x: f32,
    y: f32,
    line: &str,
    space_width: f32,
    color: nannou::color::Srgba,
) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        draw.text(line)
            .color(color)
            .font_size(12)
            .font(font.clone())
            .no_line_wrap()
            .left_justify()
            .x(x)
            .y(y);
        return;
    }

    let leading_spaces = line.len() - line.trim_start().len();
    let leading = &line[..leading_spaces];
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    let mut cursor_x = x;
    if !leading.is_empty() {
        draw.text(leading)
            .color(color)
            .font_size(12)
            .font(font.clone())
            .no_line_wrap()
            .left_justify()
            .x(cursor_x)
            .y(y);
        cursor_x += leading.len() as f32 * CHAR_WIDTH;
    }
    for (i, word) in words.iter().enumerate() {
        if i > 0 {
            cursor_x += space_width;
        }
        draw.text(word)
            .color(color)
            .font_size(12)
            .font(font.clone())
            .no_line_wrap()
            .left_justify()
            .x(cursor_x)
            .y(y);
        cursor_x += word.len() as f32 * CHAR_WIDTH;
    }
}
