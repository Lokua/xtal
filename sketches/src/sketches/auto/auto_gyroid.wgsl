// Based on https://www.shadertoy.com/view/wXVyWc

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // a: width, height, beats, x
    a: vec4f,
    // b: y, color_phase, color_shift_speed, march_base
    b: vec4f,
    // c: march_feedback, gyroid_sin_scale, gyroid_cos_div, grid_cell
    c: vec4f,
    // d: grid_freq, tone_div, iterations_mode, cell_mix
    d: vec4f,
    // e: color_mode, glow, palette_comp, reserved
    e: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;
const MAX_ITERATIONS: i32 = 96;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.pos = vert.position;
    return out;
}

@fragment
fn fs_main(@location(0) position: vec2f) -> @location(0) vec4f {
    let uv = correct_aspect(position);
    let time = params.a.z;
    let x = params.a.w;
    let y = params.b.x;
    let color_phase = params.b.y;
    let color_shift_speed = max(params.b.z, 0.0);
    let march_base = max(params.b.w, 0.000001);
    let march_feedback = max(params.c.x, 0.0);
    let gyroid_sin_scale = max(params.c.y, 0.0001);
    let gyroid_cos_div = max(params.c.z, 0.0001);
    let grid_cell = max(params.c.w, 0.0001);
    let grid_freq = max(params.d.x, 0.0001);
    let tone_div = max(params.d.y, 1.0);
    let iterations_f = clamp(round(params.d.z), 16.0, f32(MAX_ITERATIONS));
    let cell_mix = clamp(params.d.w, 0.0, 1.0);
    let color_mode = i32(round(params.e.x));
    let glow = max(params.e.y, 0.0);
    let palette_comp = clamp(params.e.z, 0.0, 1.0);
    let iterations = i32(iterations_f);
    let yaw = 6.0 * x;
    let pitch = 6.0 * y;
    let rot_pitch = rot_like(pitch);
    let rot_yaw = rot_like(yaw);

    var d = 0.0;
    var s = 0.0;
    var out_color = vec3f(0.0);

    for (var iter = 0; iter < MAX_ITERATIONS; iter += 1) {
        if (iter >= iterations) {
            break;
        }
        let i = f32(iter) + 1.0;

        d += march_base + march_feedback * abs(s);

        var p = vec3f(uv * d, d);
        let p_yz = rot_pitch * vec2f(p.y, p.z);
        p = vec3f(p.x, p_yz.x, p_yz.y);
        let p_xz = rot_yaw * vec2f(p.x, p.z);
        p = vec3f(p_xz.x, p.y, p_xz.y);

        s = 1.0 + dot(
            sin(p * gyroid_sin_scale),
            cos(p.zxy / gyroid_cos_div),
        );
        let pi_xz = rot_like(i) * vec2f(p.x, p.z);
        p = vec3f(pi_xz.x, p.y, pi_xz.y);
        let q = p / grid_cell;
        let q_soft = floor(q) + smoothstep(
            vec3f(0.0),
            vec3f(1.0),
            fract(q),
        );
        let q_hard = ceil(q);
        let morph = smoothstep(0.0, 1.0, cell_mix);
        let hard_mix = smoothstep(0.45, 1.0, cell_mix);
        let q_mix = mix(q_soft, q_hard, vec3f(hard_mix));
        let q_morph = mix(q, q_mix, vec3f(morph));
        let cell_phase = mix(1.0, 2.8, cell_mix);
        let cell_term = abs(dot(
            cos(q_morph * cell_phase),
            sin(p.yzx * grid_freq),
        ));

        let edge = abs(fract(q) - vec3f(0.5));
        let edge_strength = smoothstep(
            0.3,
            0.0,
            min(edge.x, min(edge.y, edge.z)),
        );
        let edge_mix = smoothstep(0.55, 1.0, cell_mix);
        let edge_term = edge_mix * edge_strength * 0.75;
        s -= cell_term + edge_term;

        let phase = time * color_shift_speed + color_phase + p.z;
        let comp_gain = mix(1.0, palette_gain(color_mode), palette_comp);
        let wave = color_wave(color_mode, phase) * glow * comp_gain;
        let safe_s = select(s, 0.0001, abs(s) < 0.0001);
        out_color += wave / safe_s;
    }

    let color = tanh(out_color * out_color / tone_div);
    return vec4f(color, 1.0);
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = params.a.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}

fn rot_like(a: f32) -> mat2x2f {
    let c0 = vec2f(cos(a), cos(a + 33.0));
    let c1 = vec2f(cos(a + 11.0), cos(a));
    return mat2x2f(c0, c1);
}

fn color_wave(mode: i32, phase: f32) -> vec3f {
    switch mode {
        case 0 {
            return 1.0 + cos(phase + vec3f(2.0, 1.0, 0.0));
        }
        case 1 {
            let t = 0.5 + 0.5 * sin(phase * 0.45);
            let a = vec3f(0.10, 0.14, 0.05);
            let b = vec3f(0.34, 0.28, 0.10);
            let base = mix(a, b, t);
            let grain1 = 0.11 * cos(phase * 0.72 + vec3f(0.0, 1.9, 3.7));
            let grain2 = 0.07 * sin(phase * 1.15 + vec3f(2.4, 0.8, 1.6));
            return max(base + grain1 + grain2, vec3f(0.0));
        }
        case 2 {
            let t = 0.5 + 0.5 * sin(phase * 0.35 + 0.8);
            let a = vec3f(0.05, 0.13, 0.18);
            let b = vec3f(0.25, 0.33, 0.24);
            let base = mix(a, b, t);
            let foam1 = 0.10 * cos(phase * 0.58 + vec3f(0.2, 1.7, 3.1));
            let foam2 = 0.06 * sin(phase * 1.05 + vec3f(1.3, 2.9, 0.4));
            return max(base + foam1 + foam2, vec3f(0.0));
        }
        case 3 {
            let t = 0.5 + 0.5 * sin(phase * 0.3 - 0.6);
            let a = vec3f(0.18, 0.09, 0.06);
            let b = vec3f(0.40, 0.22, 0.12);
            let base = mix(a, b, t);
            let dust1 = 0.09 * cos(phase * 0.62 + vec3f(0.1, 1.6, 3.0));
            let dust2 = 0.05 * sin(phase * 1.2 + vec3f(2.8, 0.5, 1.4));
            return max(base + dust1 + dust2, vec3f(0.0));
        }
        default {
            return 1.0 + cos(phase + vec3f(2.0, 1.0, 0.0));
        }
    }
}

fn palette_gain(mode: i32) -> f32 {
    switch mode {
        case 0 {
            return 1.0;
        }
        case 1 {
            return 1.75;
        }
        case 2 {
            return 1.6;
        }
        case 3 {
            return 1.7;
        }
        default {
            return 1.0;
        }
    }
}
