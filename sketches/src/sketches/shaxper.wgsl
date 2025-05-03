const PI: f32 = 3.14159265359;
const TAU: f32 = 6.283185307179586;

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    resolution: vec4f,
    // dist_freq, dist_echo_x, dist_echo_y, dist_echo_mix
    a: vec4f,
    // fract_noise_mix, fract_noise_scale, fract_noise_fract, fract_noise_shape
    b: vec4f,
    // wave_phase, wave_dist, wave_x_freq, wave_y_freq 
    c: vec4f,
    // fract_count, fract_zoom, fract_contrast, wave_reduce_mix, 
    d: vec4f,
    // dist_alg_mix, dist_alg_y_mult, wave_3_alg_mix, UNUSED
    e: vec4f,
    // ....
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
    let color_mix = params.e.z;
    var p = correct_aspect(position);

    let fract_dist_reduce_mix = 0.5;
    let fract_dist_map_mix = 0.5;
    let wave_reduce_mix = params.d.w;

    var reduced = mix(
        fractal_reduce(p), 
        distort_reduce(p), 
        fract_dist_reduce_mix
    );

    reduced = mix(
        reduced, 
        wave_reduce(p), 
        wave_reduce_mix
    );

    let mapped = mix(
        fractal_map(reduced), 
        distort_map(reduced), 
        fract_dist_map_mix
    );

    return vec4f(mapped, 1.0);
}

fn wave_reduce(p: vec2f) -> f32 {
    let wave_phase = params.c.x * 10.0;
    let wave_dist = params.c.y * 10.0;
    let wave_x_freq = params.c.z * 10.0;
    let wave_y_freq = params.c.w * 10.0;
    let wave_3_alg_mix = params.e.z;

    let d = length(p);
    let wave1 = tanh(d * wave_dist - wave_phase * PI);
    let wave2 = cos(p.x * wave_x_freq);
    let wave3 = mix(
        cosh(p.y * wave_y_freq),
        tan(p.y * wave_y_freq),
        wave_3_alg_mix
    );

    return wave1 + wave2 + wave3;
}

fn wave_map(wave: f32) -> vec3f {
    let color = vec3(0.5 + (cos(wave) * 0.5));
    return color;
}

fn distort_reduce(pos: vec2f) -> f32 {
    let dist_freq = params.a.x;
    let dist_echo_mix = params.a.w;
    let dist_echo_x = params.a.y;
    let dist_echo_y = params.a.z;
    let dist_alg_mix = params.e.x;
    let dist_alg_y_mult = params.e.y;

    var p = vec2f(pos.y, pos.x);

    let a = vec2f(
        sin(p.x * dist_echo_x) * cos(p.y * dist_echo_y),
        cos(p.x * dist_echo_x) * sin(p.y * dist_echo_y) * dist_alg_y_mult
    );

    let b = vec2f(
        tan(p.x * dist_echo_x), 
        tan(p.y * dist_echo_y * dist_alg_y_mult)
    );

    p = mix(a, b, dist_alg_mix);

    let echo_x = fract(p.x * dist_echo_x);
    let echo_y = fract(p.y * dist_echo_y);
    let echo = echo_x + echo_y;

    return mix(length(p), echo, dist_echo_mix);
}

fn distort_map(d: f32) -> vec3f {
    return vec3(smoothstep(0.0, 1.0, d * 0.5));
}

fn fractal_reduce(pos: vec2f) -> f32 {
    let fract_count = params.d.x; 
    let fract_noise_mix = params.b.x;
    let fract_noise_scale = params.b.y;
    let fract_noise_fract = params.b.z;
    let fract_noise_shape = params.b.w;
    let fract_zoom = params.d.y;

    var noise_x = fract(pos.y * fract_noise_fract) * 
        fract_noise_fract * -1.0;
    var noise_y = fract(pos.x * fract_noise_fract) * 
        fract_noise_fract;
    noise_x = mix(noise_x, cos(noise_x), fract_noise_shape);
    noise_y = mix(noise_y, sin(noise_y), fract_noise_shape);

    let pn = noise(vec2f((noise_x), (noise_y))) * fract_noise_scale;

    var p = mix(pos, vec2f(pn), fract_noise_mix);
    
    var color = 0.0;
    let MAX_ITERATIONS = 1000;
    for (var i = 0; i < MAX_ITERATIONS; i++) {
        let weight = 1.0 - smoothstep(fract_count - 1.0, fract_count, f32(i));
        if (weight <= 0.0) { break; }
        p = abs(p) * 2.0 - 1.0;
        let len = max(length(p), 0.1);
        color += (1.0 / len) * weight;
    }
    
    return color / fract_count;
}

fn fractal_map(color_value: f32) -> vec3f {
    let fract_contrast = params.d.z;
    let fract_steps = 1.0;
    let contrasted = pow(color_value, fract_contrast);
    let stepped = floor(contrasted * fract_steps) / fract_steps;
    return vec3f(stepped);
}

fn noise(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    
    // Cubic Hermine Curve for smoother interpolation
    let u = f * f * (3.0 - 2.0 * f);
    
    // Four corners
    let a = hash(i + vec2f(0.0, 0.0));
    let b = hash(i + vec2f(1.0, 0.0));
    let c = hash(i + vec2f(0.0, 1.0));
    let d = hash(i + vec2f(1.0, 1.0));
    
    // Bilinear interpolation
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn hash(p: vec2f) -> f32 {
    let p3 = fract(vec3f(p.xyx) * 0.13);
    let p4 = p3 + vec3f(7.0, 157.0, 113.0);
    return fract(dot(p4, vec3f(268.5453123, 143.2354234, 424.2424234)));
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.resolution.x;
    let h = params.resolution.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}
