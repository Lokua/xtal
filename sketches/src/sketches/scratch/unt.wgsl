//! Forked from https://www.shadertoy.com/view/mtyGWy

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, t, t_mult
    a: vec4f,
    // zoom, phase_gap, brightness, rotate
    b: vec4f,
    // enable_sym, sym, iterations, movement
    c: vec4f,
    // movement_t, ...
    d: vec4f,
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
    let t_mult = params.a.w;
    let t = params.a.z * t_mult;
    let zoom = params.b.x;
    let phase_gap = params.b.y;
    let brightness = params.b.z;
    let rotate = params.b.w;
    let enable_sym = params.c.x;
    let sym = params.c.y;
    let iterations = params.c.z;
    let movement = params.c.w;
    let movement_t = params.d.x;

    let angle = t * 0.125;
    var current_pos = correct_aspect(position);
    if enable_sym == 1.0 {
        current_pos = fract(abs(current_pos) * sym) - 0.5;
    }
    if rotate == 1.0 {
        current_pos = vec2f(
            current_pos.x * cos(angle) - current_pos.y * sin(angle),
            current_pos.x * sin(angle) + current_pos.y * cos(angle)
        );
    }
    let m_t = t * movement_t;
    let initial_pos = current_pos + vec2f(sin(m_t), cos(m_t)) * movement;
    var color = vec3f(0.0);
    
    for (var i = 0; i < i32(iterations); i++) {
        current_pos = fract(current_pos * zoom) - 0.5;
        let c_t = 0.5;
        let c = palette(length(initial_pos) + f32(i) * c_t + t * c_t);
        var d = length(current_pos) * exp(-length(initial_pos));
        d = sin(d * phase_gap + t) / phase_gap;
        d = abs(d);
        d = pow(brightness / d, 1.2);
        color += c * d;
    }
    
    return vec4f(color, 1.0);
}

fn palette(t: f32) -> vec3f {
    let a = vec3f(0.5, 0.5, 0.5);
    let b = vec3f(0.5, 0.0, 0.5);
    let c = vec3f(1.0, 1.0, 1.0);
    let d = vec3f(0.263, 0.416, 0.557);
    return a + b * cos(6.28318 * (c * t + d));
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = params.a.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}