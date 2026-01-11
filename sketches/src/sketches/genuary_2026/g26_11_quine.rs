use nannou::color::*;
use nannou::prelude::*;
use nannou::text::Font;

use xtal::prelude::*;

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
    offset: usize,
    trigger: Trigger,
}

fn replace_word_spaces(line: &str, space_count: usize) -> String {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return line.to_string();
    }

    // Find leading whitespace
    let leading_spaces = line.len() - line.trim_start().len();
    let leading = &line[..leading_spaces];

    // Replace spaces between words with variable spacing
    let replacement = " ".repeat(space_count);
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    let result = words.join(&replacement);

    format!("{}{}", leading, result)
}

pub fn init(_app: &App, ctx: &Context) -> Quine {
    let hub = ControlHubBuilder::new()
        .timing(Timing::new(ctx.bpm()))
        .slider("radius", 100.0, (10.0, 300.0), 1.0, None)
        .build();

    let path = to_absolute_path(file!(), "g26_11_quine.rs");
    let mut lines: Vec<String> = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| String::from("Could not read file"))
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|s| s.to_string())
        .collect();

    lines.push("-".repeat(70).to_string());

    let font = Font::from_bytes(include_bytes!(
        "/Users/lokua/Library/Fonts/FiraCode-Regular.ttf"
    ))
    .unwrap();

    let trigger = hub.animation.create_trigger(0.5, 0.0);

    Quine {
        hub,
        lines,
        font,
        offset: 0,
        trigger,
    }
}

impl Sketch for Quine {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &Context) {
        if self.hub.animation.should_trigger(&mut self.trigger) {
            self.offset = (self.offset + 1) % self.lines.len();
        }
    }

    fn view(&self, app: &App, frame: Frame, ctx: &Context) {
        let wr = ctx.window_rect();
        let draw = app.draw();

        draw.rect().x_y(0.0, 0.0).w_h(wr.w(), wr.h()).color(BLACK);

        let start_x = wr.left() + 150.0;
        let start_y = wr.top() - 20.0;
        let line_height = 16.0;

        let max_spaces = [2, 5, 8, 11];
        let phase_offsets = [0.0, 0.3, 0.6, 0.9, 0.2, 0.5, 0.8, 0.1, 0.4, 0.7];

        for i in 0..self.lines.len() {
            let line_index = (i + self.offset) % self.lines.len();
            let line = &self.lines[line_index];
            let y = start_y - (i as f32 * line_height);

            // Get max space range for this line
            let max_space = max_spaces[line_index % max_spaces.len()];

            // Get phase offset for this line (consistent per line)
            let phase_offset = phase_offsets[line_index % phase_offsets.len()];

            // Use triangle wave to animate space count
            let space_count = self
                .hub
                .animation
                .triangle(2.0, (1.0, max_space as f32), phase_offset)
                .floor() as usize;

            // Replace spaces between words
            let animated_line = replace_word_spaces(line, space_count);

            draw.text(&animated_line)
                .color(WHITE)
                .font_size(12)
                .font(self.font.clone())
                .no_line_wrap()
                .left_justify()
                .x(start_x)
                .y(y);
        }

        draw.to_frame(app, &frame).unwrap();
    }
}
