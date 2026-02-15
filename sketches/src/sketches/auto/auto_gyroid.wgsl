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
    // d: grid_freq, tone_div, reserved, reserved
    d: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;
const ITERATIONS: i32 = 64;

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
    let yaw = 6.0 * x;
    let pitch = 6.0 * y;

    var i = 1.0;
    var d = 0.0;
    var s = 0.0;
    var out_color = vec3f(0.0);

    loop {
        if (i > f32(ITERATIONS)) {
            break;
        }

        d += march_base + march_feedback * abs(s);

        var p = vec3f(uv * d, d);
        let p_yz = rot_like(pitch) * vec2f(p.y, p.z);
        p = vec3f(p.x, p_yz.x, p_yz.y);
        let p_xz = rot_like(yaw) * vec2f(p.x, p.z);
        p = vec3f(p_xz.x, p.y, p_xz.y);

        s = 1.0 + dot(
            sin(p * gyroid_sin_scale),
            cos(p.zxy / gyroid_cos_div),
        );
        let pi_xz = rot_like(i) * vec2f(p.x, p.z);
        p = vec3f(pi_xz.x, p.y, pi_xz.y);
        s -= abs(dot(cos(ceil(p / grid_cell)), sin(p.yzx * grid_freq)));

        let wave = 1.0 + cos(
            time * color_shift_speed + color_phase + p.z
            + vec3f(2.0, 1.0, 0.0),
        );
        let safe_s = select(s, 0.0001, abs(s) < 0.0001);
        out_color += wave / safe_s;
        i += 1.0;
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
