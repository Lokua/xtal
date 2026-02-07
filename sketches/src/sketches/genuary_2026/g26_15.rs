use nannou::prelude::*;
use nannou::text::Font;
use xtal::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "g26_15",
    display_name: "g26_15",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 111.0,
    w: 500,
    h: 500,
};

#[derive(SketchComponents)]
pub struct FuckICE {
    hub: ControlHub<Timing>,
    gpu: gpu::GpuState<gpu::BasicPositionVertex>,
    font: Font,
}

#[uniforms(banks = 12)]
struct ShaderParams {}

pub fn init(app: &App, ctx: &Context) -> FuckICE {
    let wr = ctx.window_rect();

    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "g26_15.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams::default();

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "g26_15.wgsl"),
        &params,
        0,
    );

    let font = Font::from_bytes(include_bytes!(
        "/Users/lokua/Library/Fonts/FiraCode-Bold.ttf"
    ))
    .unwrap();

    FuckICE { hub, gpu, font }
}

impl Sketch for FuckICE {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();
        let mut params = ShaderParams::from((&wr, &self.hub));
        params.set("a3", self.hub.animation.beats());
        self.gpu.update_params(app, wr.resolution_u32(), &params);
    }

    fn view(&self, app: &App, frame: Frame, ctx: &Context) {
        frame.clear(BLACK);
        self.gpu.render(&frame);

        let draw = app.draw();
        let font_size = self.hub.get("font_size") as u32;
        let wr = ctx.window_rect();
        let grid_cell_w = wr.w() / 3.0;
        let grid_cell_h = wr.h() / 3.0;
        let fade = self.hub.get("fade");

        let grid_keys = [
            ["grid_0_0", "grid_0_1", "grid_0_2"],
            ["grid_1_0", "grid_1_1", "grid_1_2"],
            ["grid_2_0", "grid_2_1", "grid_2_2"],
        ];

        (0..3).for_each(|row| {
            for col in 0..3 {
                let x = -wr.w() / 2.0
                    + grid_cell_w * col as f32
                    + grid_cell_w / 2.0;
                let y =
                    wr.h() / 2.0 - grid_cell_h * row as f32 - grid_cell_h / 2.0;

                let key = grid_keys[row][col];
                let grid_font_size = self.hub.get(key) as u32;

                draw.text("FUCK\nICE")
                    .color(hsla(
                        self.hub.get("bg_hue"),
                        self.hub.get("bg_saturation"),
                        self.hub.get("bg_lightness"),
                        fade,
                    ))
                    .font_size(grid_font_size)
                    .font(self.font.clone())
                    .no_line_wrap()
                    .center_justify()
                    .align_text_middle_y()
                    .x(x)
                    .y(y);
            }
        });

        let beats = self.hub.animation.beats();

        if beats > 8.0 {
            draw.text("FUCK\nICE")
                .color(hsl(
                    self.hub.get("hue"),
                    self.hub.get("saturation"),
                    self.hub.get("lightness"),
                ))
                .font_size(font_size)
                .font(self.font.clone())
                .no_line_wrap()
                .center_justify()
                .align_text_middle_y()
                .x(0.0)
                .y(0.0);

            draw.text("FUCK\nICE")
                .color(rgb(1.0, 1.0, 1.0))
                .font_size(font_size - self.hub.get("font_size_offset") as u32)
                .font(self.font.clone())
                .no_line_wrap()
                .center_justify()
                .align_text_middle_y()
                .x(0.0)
                .y(0.0);
        }

        draw.to_frame(app, &frame).unwrap();
    }
}
