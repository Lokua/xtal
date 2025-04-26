use nannou::prelude::*;

use xtal::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "fb",
    display_name: "fb",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
};

#[derive(SketchComponents)]
pub struct DynamicUniformsDev {
    hub: ControlHub<Timing>,
    shader: gpu::GpuState<gpu::BasicPositionVertex>,
    prev_texture: Option<wgpu::TextureView>,
    feedback_delay_trigger: Trigger,
}

#[uniforms(banks = 4)]
struct ShaderParams {}

pub fn init(app: &App, ctx: &Context) -> DynamicUniformsDev {
    let wr = ctx.window_rect();

    let hub = ControlHub::from_path(
        to_absolute_path(file!(), "fb.yaml"),
        Timing::new(ctx.bpm()),
    );

    let params = ShaderParams::default();

    let shader = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "fb.wgsl"),
        &params,
        1,
    );

    let feedback_delay_trigger = hub.animation.create_trigger(0.125, 0.0);

    DynamicUniformsDev {
        hub,
        shader,
        prev_texture: None,
        feedback_delay_trigger,
    }
}

impl Sketch for DynamicUniformsDev {
    fn update(&mut self, app: &App, _update: Update, ctx: &Context) {
        let wr = ctx.window_rect();

        let mut params = ShaderParams::from((&wr, &self.hub));

        params.set("a3", self.hub.animation.beats());

        self.shader.update_params(app, wr.resolution_u32(), &params);

        if let Some(ref prev_texture) = self.prev_texture {
            self.shader.set_textures(app, &[prev_texture]);
        }

        if self.hub.any_changed_in(&["delay"]) {
            self.feedback_delay_trigger = self
                .hub
                .animation
                .create_trigger(self.hub.get("delay"), 0.0);
            self.hub.mark_unchanged();
        }

        if self
            .hub
            .animation
            .should_trigger(&mut self.feedback_delay_trigger)
        {
            self.prev_texture = Some(self.shader.render_to_texture(app));
        }
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &Context) {
        self.shader.render(&frame);
    }
}
