use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "flow_field_basic",
    display_name: "Flow Field Basic",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(300),
};

#[derive(SketchComponents)]
#[sketch(clear_color = "hsla(1.0, 1.0, 1.0, 1.0)")]
pub struct FlowFieldBasic {
    #[allow(dead_code)]
    animation: Animation<Timing>,
    controls: Controls,
    agents: Vec<Agent>,
    noise: PerlinNoise,
}

pub fn init(_app: &App, ctx: LatticeContext) -> FlowFieldBasic {
    let animation = Animation::new(Timing::new(ctx.bpm));

    let controls = Controls::with_previous(vec![
        Control::select(
            "algorithm",
            "cos,sin",
            &[
                "cos,sin",
                "tanh,cosh",
                "exponential_drift",
                "lightning",
                "plasma",
                "static",
            ],
        ),
        Control::checkbox("randomize_point_size", false),
        Control::slider("agent_count", 1_000.0, (10.0, 10_000.0), 1.0),
        Control::slider("noise_scale", 100.0, (1.0, 1_000.0), 0.01),
        Control::slider("noise_strength", 10.0, (1.0, 20.0), 0.1),
        Control::slider("noise_vel", 0.01, (0.0, 0.02), 0.000_01),
        Control::slider("step_range", 5.0, (1.0, 40.0), 0.1),
        Control::slide("bg_alpha", 0.02),
    ]);

    FlowFieldBasic {
        animation,
        controls,
        agents: vec![],
        noise: PerlinNoise::new(512),
    }
}

impl Sketch for FlowFieldBasic {
    fn update(&mut self, _app: &App, _update: Update, ctx: &LatticeContext) {
        if self.controls.any_changed_in(&["agent_count"]) {
            let agent_count = self.controls.float("agent_count") as usize;
            let wr = ctx.window_rect();

            if self.agents.len() > agent_count {
                self.agents.truncate(agent_count);
            } else if self.agents.len() < agent_count {
                let new_agents = (self.agents.len()..agent_count)
                    .map(|_| Agent::new(random_point(&wr)));
                self.agents.extend(new_agents);
            }

            self.controls.mark_unchanged();
        }

        let noise_scale = self.controls.float("noise_scale");
        let noise_strength = self.controls.float("noise_strength");
        let noise_vel = self.controls.float("noise_vel");
        let step_range = self.controls.float("step_range");
        let algorithm = self.controls.string("algorithm");
        let wr = ctx.window_rect();

        self.agents.iter_mut().for_each(|agent| {
            agent.step_size = random_range(1.0, step_range + 0.001);
            agent.update(
                algorithm.as_str(),
                &self.noise,
                noise_scale,
                noise_strength,
                noise_vel,
            );
            agent.constrain(&wr);
        });
    }

    fn view(&self, app: &App, frame: Frame, ctx: &LatticeContext) {
        // let start = Instant::now();

        let draw = app.draw();
        let wr = ctx.window_rect();

        draw.rect().wh(wr.vec2()).color(hsla(
            1.0,
            1.0,
            1.0,
            self.controls.float("bg_alpha"),
        ));

        let randomize_point_size = self.controls.bool("randomize_point_size");

        self.agents.iter().for_each(|agent| {
            let radius = ternary!(randomize_point_size, random_f32(), 1.0);

            draw.ellipse()
                .radius(radius)
                .xy(agent.pos)
                .color(hsla(0.7, 0.2, 0.02, 1.0));
        });

        draw.to_frame(app, &frame).unwrap();

        // debug!("draw: {:?}", start.elapsed());
    }
}

fn random_point(wr: &WindowRect) -> Vec2 {
    vec2(
        random_range(-wr.hw(), wr.hw()),
        random_range(-wr.hh(), wr.hh()),
    )
}

#[derive(Clone)]
struct Agent {
    pub pos: Vec2,
    step_size: f32,
    angle: f32,
    noise_vel: f32,
}

impl Agent {
    pub fn new(initial_pos: Vec2) -> Self {
        Self {
            pos: initial_pos,
            step_size: random_range(1.0, 20.0),
            angle: 0.0,
            noise_vel: 0.0,
        }
    }

    pub fn update(
        &mut self,
        algorithm: &str,
        noise: &PerlinNoise,
        noise_scale: f32,
        noise_strength: f32,
        noise_vel: f32,
    ) {
        self.angle = noise.get([
            self.pos.x / noise_scale,
            self.pos.y / noise_scale,
            self.noise_vel,
        ]) * noise_strength;

        match algorithm {
            "cos,sin" => {
                self.pos.x += self.angle.cos() * self.step_size;
                self.pos.y += self.angle.sin() * self.step_size;
            }
            "tanh,cosh" => {
                self.pos.x += self.angle.tanh() * self.step_size;
                self.pos.y += self.angle.cosh() * self.step_size;
            }
            "exponential_drift" => {
                self.pos.x += self.angle.exp() * self.step_size * 0.1;
                self.pos.y += self.angle.powi(3) * self.step_size;
            }
            "lightning" => {
                self.pos.x += (1.0 / (1.0 + self.angle.abs())) * self.step_size;
                self.pos.y += self.angle.tan() * self.step_size;
            }
            "plasma" => {
                self.pos.x +=
                    (1.0 / (1.0 + self.angle.powi(2))) * self.step_size;
                self.pos.y +=
                    (self.angle.tan() * self.angle.cos()) * self.step_size;
            }
            "static" => {
                let r = 1.0 / (1.0 + self.angle.cos().powi(2));
                self.pos.x += r * self.step_size;
                self.pos.y += (self.angle.tan() / (1.0 + r)) * self.step_size;
            }
            _ => unreachable!(),
        }

        self.noise_vel += noise_vel;
    }

    pub fn constrain(&mut self, wr: &WindowRect) {
        if self.pos.x < -wr.hw() {
            self.pos.x = wr.hw();
        }
        if self.pos.x > wr.hw() {
            self.pos.x = -wr.hw();
        }
        if self.pos.y < -wr.hh() {
            self.pos.y = wr.hh();
        }
        if self.pos.y > wr.hh() {
            self.pos.y = -wr.hh();
        }
    }
}
