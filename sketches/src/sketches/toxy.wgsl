//! Forked from https://www.shadertoy.com/view/wdyBWK

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, time, rpos_x
    a: vec4f,
    // rpos_y, n_freq, alg_mix, q_freq_anim
    b: vec4f,
    // q_freq, animate_q_freq, q_x_dist, q_y_dist
    c: vec4f,
    // color_mode, color_param_a, color_param_c
    d: vec4f,
    // n_glitch, n_amp, rand_a, rand_b
    e: vec4f,
    // rand_c, rand_mult, ..
    f: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.pos = vert.position;
    return out;
}

@fragment
fn fs_main(@location(0) position: vec2f) -> @location(0) vec4f {
    let resolution = vec2f(params.a.x, params.a.y);
    let t = params.a.z * 0.5;
    let rpos_x = params.a.w;
    let rpos_y = params.b.x;
    let r_pos = correct_aspect(vec2f(rpos_x, rpos_y));
    let alg_mix = params.b.z;
    let q_freq_slider = params.b.w;
    let q_freq_anim = params.c.x;
    let animate_q_freq = params.c.y;
    let q_freq = select(q_freq_slider, q_freq_anim, animate_q_freq == 0.0);
    let q_x_d = params.c.z;
    let q_x_y = params.c.w;
    let color_mode = params.d.x;
    let color_param_a = params.d.y;
    let color_param_b = params.d.z;
    let color_param_c = params.d.w;
    
    let st = correct_aspect(position);

    var q = vec2f(0.0);
    q.x = fbm(q_freq * st + 0.55 * t);
    q.y = fbm(q_freq * st + 0.21 * t);
    q.x = mix(sin(q.x * q_x_d), tan(q.x * q_x_d), alg_mix);
    q.y = mix(cos(q.y * q_x_y), 1.0 / tan(q.y * q_x_y), alg_mix);
    
    var color = fbm(10.0 * st + 1.0 * q + 3.0 * r_pos);

    let component_a = color * clamp(length(q * color_param_a), 0.0, 1.0);
    let component_b = pow(color, color_param_b);
    let component_c = color * 
        clamp(q.x + q.y, color_param_c, 1.0 - color_param_c);

    if color_mode == 0.0 {
        return vec4f(
            component_b,
            component_a,
            component_c,
            1.0
        );
    } else if color_mode == 1.0 {
        return vec4f(
            component_c,
            component_a,
            component_b,
            1.0
        );
    } else {
        return vec4f(
            component_a,
            component_c,
            component_b,
            1.0
        );
    }
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = params.a.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}

fn rand(p: vec2f) -> f32 {
    let rand_a = params.e.z;
    let rand_b = params.e.w;
    let rand_c = params.f.x;
    let rand_mult = params.f.y;
    let xy = vec2f(
        rand_a * 2.0 + (rand_mult * 31.7667), 
        rand_b * 3.0 + (rand_mult + 14.9876)
    );
    return fract(sin(dot(p, xy)) * rand_c * 4.0 + (rand_mult * 833443.123456));
}

fn bilinear_noise(st: vec2f) -> f32 {
    let n_glitch = params.e.x;

    let i = floor(st);
    let f = fract(st);
    
    let f00 = rand(i);
    let f10 = rand(i + vec2f(1.0 - n_glitch, n_glitch));
    let f01 = rand(i + vec2f(n_glitch, 1.0 - (n_glitch)));
    let f11 = rand(i + vec2f(1.0 - n_glitch, 1.0 - n_glitch));
    
    let u = smoothstep(vec2f(0.0), vec2f(1.0), (1.0 - f));

    return u.x * u.y * f00 + 
        (1.0 - u.x) * u.y * f10 +
        u.x * (1.0 - u.y) * f01 + 
        (1.0 - u.x) * (1.0 - u.y) * f11;
}

fn fbm(st: vec2f) -> f32 {
    var n_amp = params.e.y;

    var value = 0.0;
    var frequency = params.b.y;
    
    for (var i = 0; i < 5; i++) {
        value += n_amp * bilinear_noise(frequency * st);
        frequency *= 2.0;
        n_amp *= 0.5;
    }

    return value;
}