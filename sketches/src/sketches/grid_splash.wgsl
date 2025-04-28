struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, t, grid_size
    a: vec4f,
    // circle_radius, line_width, freq, amp
    b: vec4f,
    // alg_mix, t_wave, a_exp, b_exp
    c: vec4f,
    // a_rotate, a_rotation_speed, invert, center_style_mix
    d: vec4f,
    e: vec4f,
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
    let t = params.a.z;
    let grid_size = params.a.w;
    var circle_radius = params.b.x;
    let line_width = params.b.y;
    let freq = params.b.z;
    let amp = params.b.w;
    let alg_mix = params.c.x;
    let t_wave = params.c.y;
    let invert = params.d.z == 1.0;
    let center_style_mix = params.d.w;

    let p = correct_aspect(position);
    let grid_pos = fract(p * grid_size) * 2.0 - 1.0;

    let v0 = mix(
        weave_a(vec2f(0.0), p, freq) * amp,
        weave_b(vec2f(0.0), p, freq) * amp,
        alg_mix
    );
    let v1 = mix(
        weave_c(vec2f(0.0), p, freq * 4.0) * amp,
        weave_d(vec2f(0.0), p, freq * 12.0) * amp,
        alg_mix
    );

    var weave_value = mix(v0, v1, center_style_mix);
    
    let displacement = n(tan(weave_value + t) * t_wave);
    let radius = circle_radius * displacement;
    
    let dist = length(grid_pos);
    let cr = radius - line_width;
    let outer = smoothstep(cr - 0.01, cr + 0.01, dist);
    let inner = smoothstep(radius + 0.01, radius - 0.01, dist);
    let circle_outline = outer * inner;

    let darkness = 0.2 + 0.8 * displacement;
    let color = vec3f(darkness) * circle_outline;
    return vec4f(select(color, 1.0 - color, invert), 1.0);
}

fn weave_a(p1: vec2f, p2: vec2f, frequency: f32) -> f32 {
    let t = params.a.z;
    let exp = params.c.z;
    let rotate_a = params.d.x == 1.0;
    let a_rotation_speed = params.d.y;
    
    let degrees = select(135.0, (t * a_rotation_speed) % 360.0, rotate_a);
    let angle = radians(degrees);
    let cos_angle = cos(angle);
    let sin_angle = sin(angle);
    let rotated_x = p2.x * cos_angle - p2.y * sin_angle;
    let rotated_y = p2.x * sin_angle + p2.y * cos_angle;

    let dx = powf(abs(p2.x - p1.x), exp);
    let dy = powf(abs(p2.y - p1.y), exp);
    return (sin(rotated_x * frequency) + sin(rotated_y * frequency))
        * sin(sqrt(dx + dy) * 0.05) * 100.0;
}

fn weave_b(p1: vec2f, p2: vec2f, frequency: f32) -> f32 {
    let exp = params.c.w;
    let dx = powf(abs(p2.x - p1.x), exp);
    let dy = powf(abs(p2.y - p1.y), exp);
    return (cos(p2.x * frequency) + sin(p2.y * frequency))
        * sin(sqrt(dx + dy) * 0.05) * 100.0;
}

fn weave_c(p1: vec2f, p2: vec2f, frequency: f32) -> f32 {
    let t = params.a.z;
    let exp = params.c.z;
    let rotate_a = params.d.x == 1.0;
    let a_rotation_speed = params.d.y;
    
    let degrees = select(135.0, (-t * a_rotation_speed) % 360.0, rotate_a);
    let angle = radians(degrees);
    let cos_angle = cos(angle);
    let sin_angle = sin(angle);
    let rotated_x = p2.x * cos_angle - p2.y * sin_angle;
    let rotated_y = p2.x * sin_angle + p2.y * cos_angle;

    let dx = powf(abs(p2.x - p1.x), exp);
    let dy = powf(abs(p2.y - p1.y), exp);
    return (sin(rotated_x * frequency) + sin(rotated_y * frequency))
        * sin(exp(-length(vec2f(dx, dy))) * 5.0) * 10.0;
}

fn weave_d(p1: vec2f, p2: vec2f, frequency: f32) -> f32 {
    let exp = params.c.w;
    let dx = powf(abs(p2.x - p1.x), exp);
    let dy = powf(abs(p2.y - p1.y), exp);
    return (cos(p2.x * frequency) + sin(p2.y * frequency))
        * abs(atan2(dy, dx)) * 2.0;
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = params.a.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}

fn n(x: f32) -> f32 {
    return x * 0.5 + 0.5;
}

fn powf(x: f32, y: f32) -> f32 {
    let y_rounded = round(y);
    if (abs(y - y_rounded) < 1e-4 && modulo(y_rounded, 2.0) == 1.0) {
        return sign(x) * pow(abs(x), y);
    }
    return pow(abs(x), y);
}

fn modulo(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}



