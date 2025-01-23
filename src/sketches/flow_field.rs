use bevy_reflect::Reflect;
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "flow_field",
    display_name: "Flow Field",
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

    // lightning, ...unsued
    b: [f32; 4],
}

#[derive(SketchComponents)]
#[sketch(clear_color = "hsla(1.0, 1.0, 1.0, 1.0)")]
pub struct Model {
    #[allow(dead_code)]
    animation: Animation<MidiSongTiming>,
    controls: Controls,
    midi: MidiControls,
    wr: WindowRect,
    agents: Vec<Agent>,
    noise: PerlinNoise,
    gpu: gpu::GpuState<Vertex>,
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let animation = Animation::new(MidiSongTiming::new(SKETCH_CONFIG.bpm));

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
        Control::slider("agent_count", 1_000.0, (10.0, MAX_COUNT as f32), 1.0),
        Control::slider("agent_size", 0.002, (0.001, 0.01), 0.0001),
        Control::slider("step_range", 5.0, (1.0, 40.0), 0.1),
        Control::slider_norm("bg_alpha", 0.02),
        Control::Separator {},
        Control::slider_norm("displace", 0.00),
        Control::slider_norm("slice_glitch", 0.00),
        Control::slider_norm("b2", 0.00),
        Control::slider_norm("b3", 0.00),
        Control::slider_norm("b4", 0.00),
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
        to_absolute_path(file!(), "./flow_field.wgsl"),
        &params,
        Some(&initial_vertices),
        wgpu::PrimitiveTopology::TriangleList,
        Some(wgpu::BlendState::ALPHA_BLENDING),
        true,
    );

    Model {
        animation,
        controls,
        midi,
        wr,
        agents: vec![],
        noise: PerlinNoise::new(512),
        gpu,
    }
}

pub fn update(app: &App, m: &mut Model, _update: Update) {
    if m.controls.any_changed_in(&["agent_count"]) {
        let agent_count = m.controls.float("agent_count") as usize;

        if m.agents.len() > agent_count {
            m.agents.truncate(agent_count);
        } else if m.agents.len() < agent_count {
            let new_agents = (m.agents.len()..agent_count)
                .map(|_| Agent::new(random_point(&m.wr)));
            m.agents.extend(new_agents);
        }

        m.controls.mark_unchanged();
    }

    let algorithm = match m.midi.get("alg").floor() as u32 {
        0 => "cos,sin",
        1 => "tanh,cosh",
        2 => "exponential_drift",
        3 => "lightning",
        4 => "plasma",
        5 => "static",
        _ => panic!("Unsupported algorithm"),
    };

    let agent_size = m.controls.float("agent_size");
    let noise_scale = m.midi.get("noise_scale");
    let noise_strength = m.midi.get("noise_strength");
    let noise_vel = m.midi.get("noise_vel");
    let step_range = m.controls.float("step_range");
    let bg_alpha = m.controls.float("bg_alpha");

    m.agents.iter_mut().for_each(|agent| {
        agent.step_size = random_range(1.0, step_range + 0.001);
        agent.update(
            algorithm,
            &m.noise,
            noise_scale,
            noise_strength,
            noise_vel,
        );
        agent.constrain(&m.wr);
    });

    let params = ShaderParams {
        resolution: [m.wr.w(), m.wr.h(), 0.0, 0.0],
        a: [
            bg_alpha,
            m.animation.lrp(&[((40.0), 4.0), (70.0, 4.0)], 0.0),
            m.midi.get("displace"),
            m.midi.get("slice_glitch"),
        ],
        b: [
            m.midi.get("lightning"),
            m.animation.ping_pong(6.0),
            m.controls.float("b3"),
            m.controls.float("b4"),
        ],
    };

    let randomize_point_size = m.controls.bool("randomize_point_size");
    let (size_min, _) = m.controls.slider_range("agent_size");
    let size_range = safe_range(size_min - 0.000_1, agent_size);

    let mut vertices =
        generate_quad_vertices(vec2(0.0, 0.0), 1.0, VERTEX_TYPE_BG);
    vertices.reserve(m.agents.len() * 6);

    for agent in &m.agents {
        let size = if randomize_point_size {
            random_range(size_range.0, size_range.1)
        } else {
            agent_size
        };
        vertices.extend(generate_quad_vertices(
            vec2(agent.pos.x / m.wr.hw(), agent.pos.y / m.wr.hh()),
            size,
            VERTEX_TYPE_AGENT,
        ));
    }

    m.gpu.update(app, &params, &vertices);
}

pub fn view(_app: &App, m: &Model, frame: Frame) {
    m.gpu.render(&frame);
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
