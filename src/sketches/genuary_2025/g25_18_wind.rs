use bevy_reflect::Reflect;
use nannou::prelude::*;

use crate::framework::prelude::*;

// Bitwig/2025/Lattice - Flow Field w 312

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "g25_18_wind",
    display_name: "Genuary 18: What does wind look like?",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 1244,
    gui_w: None,
    gui_h: Some(400),
};

const MAX_COUNT: usize = 100_000;
const VERTEX_TYPE_BG: f32 = 0.0;
const VERTEX_TYPE_AGENT: f32 = 1.0;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Reflect)]
struct Vertex {
    position: [f32; 2],
    vertex_type: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderParams {
    resolution: [f32; 4],
    // bg_alpha, bg_anim, displace, slice_glitch
    a: [f32; 4],
    // lightning, ...unused
    b: [f32; 4],
}

#[derive(SketchComponents)]
#[sketch(clear_color = "hsla(1.0, 1.0, 1.0, 1.0)")]
pub struct G25_18Wind {
    #[allow(dead_code)]
    animation: Animation<MidiSongTiming>,
    controls: Controls,
    midi: MidiControls,
    agents: Vec<Agent>,
    noise: PerlinNoise,
    gpu: gpu::GpuState<Vertex>,
}

pub fn init(app: &App, ctx: &LatticeContext) -> G25_18Wind {
    let animation = Animation::new(MidiSongTiming::new(ctx.bpm()));

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
        // NOTE: this control is broken; needs to be maxed out
        Control::slider_x(
            "agent_count",
            MAX_COUNT as f32,
            (10.0, MAX_COUNT as f32),
            1.0,
            |_controls: &Controls| true,
        ),
        Control::slider("agent_size", 0.002, (0.001, 0.01), 0.0001),
        Control::slider("step_range", 5.0, (1.0, 40.0), 0.1),
        Control::slide("bg_alpha", 0.02),
        Control::Separator {},
        Control::slide("displace", 0.00),
        Control::slide("slice_glitch", 0.00),
        Control::slide("b2", 0.00),
        Control::slide("b3", 0.00),
        Control::slide("b4", 0.00),
    ]);

    let midi = MidiControlBuilder::new()
        .control_mapped("noise_strength", (0, 1), (0.0, 20.0), 0.0)
        .control_mapped("noise_vel", (0, 2), (0.0, 0.02), 0.0)
        .control_mapped("noise_scale", (0, 3), (1.0, 1_000.0), 100.0)
        .control("displace", (0, 4), 0.0)
        .control("slice_glitch", (0, 5), 1.0)
        .control_mapped("alg", (0, 6), (0.0, 5.0), 0.0)
        .control("lightning", (0, 7), 1.0)
        .build();

    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
    };

    let initial_vertices: Vec<Vertex> = vec![
        Vertex {
            position: [0.0, 0.0],
            vertex_type: VERTEX_TYPE_AGENT,
        };
        (MAX_COUNT * 6) + 6
    ];

    let gpu = gpu::GpuState::new(
        app,
        ctx.window_rect().resolution_u32(),
        to_absolute_path(file!(), "g25_18_wind.wgsl"),
        &params,
        Some(&initial_vertices),
        wgpu::PrimitiveTopology::TriangleList,
        Some(wgpu::BlendState::ALPHA_BLENDING),
        false,
        true,
    );

    G25_18Wind {
        animation,
        controls,
        midi,
        agents: vec![],
        noise: PerlinNoise::new(512),
        gpu,
    }
}

impl Sketch for G25_18Wind {
    fn update(&mut self, app: &App, _update: Update, ctx: &LatticeContext) {
        let wr = ctx.window_rect();

        if self.controls.any_changed_in(&["agent_count"]) {
            let agent_count = MAX_COUNT;

            if self.agents.len() > agent_count {
                self.agents.truncate(agent_count);
            } else if self.agents.len() < agent_count {
                let new_agents = (self.agents.len()..agent_count)
                    .map(|_| Agent::new(random_point(&wr)));
                self.agents.extend(new_agents);
            }

            self.controls.mark_unchanged();
        }

        let algorithm = match self.midi.get("alg").floor() as u32 {
            0 => "cos,sin",
            1 => "tanh,cosh",
            2 => "exponential_drift",
            3 => "lightning",
            4 => "plasma",
            5 => "static",
            _ => panic!("Unsupported algorithm"),
        };

        let agent_size = self.controls.float("agent_size");
        let noise_scale = self.midi.get("noise_scale");
        let noise_strength = self.midi.get("noise_strength");
        let noise_vel = self.midi.get("noise_vel");
        let step_range = self.controls.float("step_range");
        let bg_alpha = self.controls.float("bg_alpha");

        self.agents.iter_mut().for_each(|agent| {
            agent.step_size = random_range(1.0, step_range + 0.001);
            agent.update(
                algorithm,
                &self.noise,
                noise_scale,
                noise_strength,
                noise_vel,
            );
            agent.constrain(&wr);
        });

        let params = ShaderParams {
            resolution: [wr.w(), wr.h(), 0.0, 0.0],
            a: [
                bg_alpha,
                self.animation.lrp(&[kf(40.0, 4.0), kf(70.0, 4.0)], 0.0),
                self.midi.get("displace"),
                self.midi.get("slice_glitch"),
            ],
            b: [
                self.midi.get("lightning"),
                self.animation.tri(6.0),
                self.controls.float("b3"),
                self.controls.float("b4"),
            ],
        };

        let randomize_point_size = self.controls.bool("randomize_point_size");
        let (size_min, _) = self.controls.slider_range("agent_size");
        let size_range = safe_range(size_min - 0.000_1, agent_size);

        let mut vertices =
            generate_quad_vertices(vec2(0.0, 0.0), 1.0, VERTEX_TYPE_BG);
        vertices.reserve(self.agents.len() * 6);

        for agent in &self.agents {
            let size = if randomize_point_size {
                random_range(size_range.0, size_range.1)
            } else {
                agent_size
            };
            vertices.extend(generate_quad_vertices(
                vec2(agent.pos.x / wr.hw(), agent.pos.y / wr.hh()),
                size,
                VERTEX_TYPE_AGENT,
            ));
        }

        self.gpu.update(
            app,
            ctx.window_rect().resolution_u32(),
            &params,
            &vertices,
        );
    }

    fn view(&self, _app: &App, frame: Frame, _ctx: &LatticeContext) {
        self.gpu.render(&frame);
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

fn generate_quad_vertices(p: Vec2, size: f32, vertex_type: f32) -> Vec<Vertex> {
    vec![
        Vertex {
            position: [p.x - size, p.y - size],
            vertex_type,
        },
        Vertex {
            position: [p.x + size, p.y - size],
            vertex_type,
        },
        Vertex {
            position: [p.x + size, p.y + size],
            vertex_type,
        },
        Vertex {
            position: [p.x - size, p.y - size],
            vertex_type,
        },
        Vertex {
            position: [p.x + size, p.y + size],
            vertex_type,
        },
        Vertex {
            position: [p.x - size, p.y + size],
            vertex_type,
        },
    ]
}
