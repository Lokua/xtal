use bytemuck::{Pod, Zeroable};
use nannou::prelude::*;

use xtal::prelude::*;

// Live/2025.02.19 Blob
// Run with `osc` timing

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "blob",
    display_name: "Blob",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 1244,
};

#[derive(SketchComponents)]
pub struct Blob {
    hub: ControlHub<Timing>,
    shader: gpu::GpuState<gpu::BasicPositionVertex>,
    feedback_texture: Option<wgpu::TextureView>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct ShaderParams {
    // w, h, ..unused
    resolution: [f32; 4],

    // t1, t2, t3, t4
    a: [f32; 4],

    // invert, center_size, smoothness, color_mix
    b: [f32; 4],

    // t_long, center_y, outer_scale_animation_a, center_size
    c: [f32; 4],

    // UNUSED, outer_size, outer_scale_animation_mix, outer_scale_animation_b
    d: [f32; 4],

    e: [f32; 4],
    f: [f32; 4],
}

pub fn init(app: &App, ctx: &Context) -> Blob {
    let window_rect = ctx.window_rect();
    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "blob.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
        c: [0.0; 4],
        d: [0.0; 4],
        e: [0.0; 4],
        f: [0.0; 4],
    };

    let shader = gpu::GpuState::new_fullscreen(
        app,
        window_rect.resolution_u32(),
        to_absolute_path(file!(), "blob.wgsl"),
        &params,
        1,
    );

    Blob {
        hub,
        shader,
        feedback_texture: None,
    }
}

impl Sketch for Blob {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();

        let params = ShaderParams {
            resolution: [
                wr.w(),
                wr.h(),
                self.hub.get("edge_mix"),
                self.hub.get("edge_size"),
            ],
            a: [
                self.hub.get("t1"),
                self.hub.get("t2"),
                self.hub.get("t3"),
                self.hub.get("t4"),
            ],
            b: [
                self.hub.get("invert"),
                self.hub.get("smoothness"),
                self.hub.get("blur"),
                self.hub.get("color_mix"),
            ],
            c: [
                self.hub.get("t_long"),
                self.hub.get("center_y"),
                self.hub.get("c3"),
                self.hub.get("c4"),
            ],
            d: [
                self.hub.get("d1"),
                self.hub.get("d2"),
                self.hub.get("d3"),
                self.hub.get("d4"),
            ],
            e: [
                self.hub.get("e1"),
                self.hub.get("e2"),
                self.hub.get("e3"),
                self.hub.get("e4"),
            ],
            f: [
                self.hub.get("f1"),
                self.hub.get("f2"),
                self.hub.get("f3"),
                self.hub.get("f4"),
            ],
        };

        self.shader.update_params(app, wr.resolution_u32(), &params);

        if let Some(ref feedback_texture) = self.feedback_texture {
            self.shader.set_textures(app, &[feedback_texture]);
        }

        self.feedback_texture = Some(self.shader.render_to_texture(app));
    }

    fn view(&self, app: &App, frame: Frame, ctx: &Context) {
        let draw = app.draw();
        ctx.background(&frame, &draw, hsla(0.0, 0.0, 0.3, 0.02));
        self.shader.render(&frame);
    }
}
