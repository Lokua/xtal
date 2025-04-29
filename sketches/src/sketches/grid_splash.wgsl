const PI: f32 = 3.14159265359;
const TAU: f32 = 6.283185307179586;

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
    @location(1) uv: vec2f
};

struct Params {
    // w, h, t, grid_size
    a: vec4f,
    // circle_radius, line_width, freq, amp
    b: vec4f,
    // ab_mix, t_wave, a_exp, b_exp
    c: vec4f,
    // ac_rotate, a_rotation_speed, invert, ab_cd_mix
    d: vec4f,
    // red_or_cyan, blue_or_magenta, green_or_yellow, colorize
    e: vec4f,
    // cd_mix, cd_amp, cd_freq, norm_color_disp
    f: vec4f,
    // outer_spread, effects_mix, ..
    g: vec4f,
    h: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@group(1) @binding(0)
var source_sampler: sampler;

@group(1) @binding(1)
var source_texture: texture_2d<f32>;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.pos = vert.position;
    out.uv = out.pos * 0.5 + 0.5;
    return out;
}

@fragment
fn fs_main(
    @location(0) position: vec2f, 
    @location(1) uv: vec2f
) -> @location(0) vec4f {
    let t = params.a.z;
    let grid_size = params.a.w;
    var circle_radius = params.b.x;
    let line_width = params.b.y;
    let ab_freq = params.b.z;
    let ab_amp = params.b.w;
    let ab_mix = params.c.x;
    let t_wave = params.c.y;
    let invert = params.d.z == 1.0;
    let ab_cd_mix = params.d.w;
    let red_or_cyan = params.e.x;
    let green_or_magenta = params.e.y;
    let blue_or_yellow = params.e.z;
    let colorize = params.e.w;
    let cd_mix = params.f.x;
    let cd_amp = params.f.y;
    let cd_freq = params.f.z;
    let norm_color_disp = params.f.w;
    let outer_spread = params.g.x;
    let effects_mix = params.g.y;

    let p = correct_aspect(position);
    let grid_pos = fract(p * grid_size) * 2.0 - 1.0;

    let v0 = mix(
        weave_a(vec2f(0.0), p, ab_freq) * ab_amp,
        weave_b(vec2f(0.0), p, ab_freq) * ab_amp,
        ab_mix
    );
    let v1 = mix(
        weave_c(vec2f(0.0), p, cd_freq * 2.0) * cd_amp,
        weave_d(vec2f(0.0), p, cd_freq * 3.0) * cd_amp,
        cd_mix
    );

    var weave_value = mix(v0, v1, ab_cd_mix);
    
    let displacement = n(tan(weave_value + t) * t_wave);
    let radius = circle_radius * displacement;
    
    let dist = length(grid_pos);
    let cr = radius - line_width;
    let outer = smoothstep(cr - outer_spread, cr + outer_spread, dist);
    let inner = smoothstep(radius + 0.01, radius - 0.01, dist);
    let circle_outline = outer * inner;
    
    let steps = 4.0;
    let normalized_disp = smoothstep(1.0, 0.0, displacement);
    let quantized_displacement = floor(
        mix(displacement, normalized_disp, norm_color_disp) * steps
    ) / steps;

    let base_color = vec3f(
        red_or_cyan * sin(quantized_displacement * PI),
        green_or_magenta * cos(quantized_displacement * PI),
        blue_or_yellow * sin(quantized_displacement * PI)
    );

    let color_intensity = mix(0.1, 1.0, quantized_displacement);
    let color_component = base_color * color_intensity;
    let background_mask = 1.0 - smoothstep(0.0, 0.1, circle_outline);
    let color_background = color_component * background_mask;
    let circle_color = vec3f(displacement) * circle_outline;
    let colorized = circle_color + color_background;

    var color = mix(
        vec3f(displacement) * circle_outline, 
        colorized, 
        colorize
    );

    color = select(color, 1.0 - color, invert);
    color = feedback_warp(color, p, uv, effects_mix) + (color * 0.25);

    return vec4f(color, 1.0);
}

fn feedback_warp(color: vec3f, p: vec2f, uv: vec2f, mix: f32) -> vec3f {
    let t = params.a.z;

    var sample = textureSample(
        source_texture,
        source_sampler,
        vec2f(
            fract(uv.x * PI * 3.0 + sin(t * 0.0125)), 
            fract(uv.y * PI * 4.0 + sin(t * 0.125))
        )
    ).rgb;
    
    sample.r = clamp(0.2 + n(sin(sample.r + (t * 0.1666667))), 0.0, 1.0);
    sample.g = clamp(0.5 + n(sin(sample.r + (t * 0.25))), 0.0, 1.0);
    sample.b = clamp(0.3 + n(sin(sample.r + (t * 0.0125))), 0.0, 1.0);
    
    return mix(color, sample, mix);
}

fn weave_a(p1: vec2f, p2: vec2f, frequency: f32) -> f32 {
    let t = params.a.z;
    let exp = params.c.z;
    let ac_rotate = params.d.x == 1.0;
    let a_rotation_speed = params.d.y;
    
    let p = rotate_point(
        p2, 
        select(135.0, (t * a_rotation_speed) % 360.0, ac_rotate)
    );

    let dx = powf(abs(p2.x - p1.x), exp);
    let dy = powf(abs(p2.y - p1.y), exp);
    return (sin(p.x * frequency) + sin(p.y * frequency))
        * sin(sqrt(dx + dy) * 0.05) * 100.0;
}

fn rotate_point(p: vec2f, angle_degrees: f32) -> vec2f {
    let angle = radians(angle_degrees);
    let cos_angle = cos(angle);
    let sin_angle = sin(angle);
    
    return vec2f(
        p.x * cos_angle - p.y * sin_angle,
        p.x * sin_angle + p.y * cos_angle
    );
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
    let ac_rotate = params.d.x == 1.0;
    let a_rotation_speed = params.d.y;
    
    let degrees = select(135.0, (-t * a_rotation_speed) % 360.0, ac_rotate);
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
    let wave_pattern = cos(p2.x * frequency) + sin(p2.y * frequency);
    let distance = length(vec2f(dx, dy));
    let angle_factor = abs(atan2(dy, dx));
    let center_distance = length(p2 - p1);
    let blend = smoothstep(0.0, 0.2, center_distance);
    let modified_angle = mix(1.0, angle_factor, blend);
    return wave_pattern * modified_angle * 2.0;
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



