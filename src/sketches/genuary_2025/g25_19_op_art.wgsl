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
    // w, h, ..unused
    resolution: vec4f,

    // wave_phase, wave_radial_freq, wave_horiz_freq, wave_vert_freq
    a: vec4f,

    // bg_freq, bg_radius, bg_gradient_strength, wave_power
    b: vec4f,

    // reduce_mix, map_mix, wave_bands, wave_threshold
    c: vec4f,

    // bg_invert, wave1_mod, mix_mode, wave_scale
    d: vec4f,

    // wave1_mix, wave2_mix, wave3_mix, unused
    e: vec4f,
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
    let reduce_mix = params.c.x;
    let map_mix = params.c.y;
    let mix_mode = params.d.z;

    let p = correct_aspect(position);

    let reduced = mix(wave_reduce(p), fractal_reduce(p), reduce_mix);

    var mapped: vec4f;

    if mix_mode == 0.0 {
        mapped = mix(wave_map(reduced), fractal_map(reduced), map_mix);
    } else if mix_mode == 1.0 {
        mapped = mix(
            mix_min(wave_map(reduced), fractal_map(reduced)), 
            mix_max(wave_map(reduced), fractal_map(reduced)), 
            map_mix
        );
    }

    return mapped;
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.resolution.x;
    let h = params.resolution.y;

    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    
    return p;
}

fn wave_reduce(p: vec2f) -> f32 {
    let phase = params.a.x;
    let radial_freq = params.a.y;
    let horiz_freq = params.a.z;
    let vert_freq = params.a.w;
    let power = params.b.w;
    let scale = params.d.w;
    let wave1_mod = params.d.y;
    let wave1_mix = params.e.x;
    let wave2_mix = params.e.y;
    let wave3_mix = params.e.z;

    let d = length(p);

    var wave1 = sin(d * radial_freq - phase * TAU);
    wave1 = mix(wave1, tan(p.y * wave1_mod), wave1_mix);

    var wave2 = sin(p.x * horiz_freq) * scale;
    wave2 = mix(wave2, cosh(p.x), wave2_mix);

    var wave3 = sin(p.y * vert_freq) * scale;
    wave3 = mix(wave3, fract(p.y * 8.0), wave3_mix);

    return powf(wave1 + wave2 + wave3, power);
}

fn wave_map(wave: f32) -> vec4f {
    let n_bands = floor(params.c.z);
    let threshold = params.c.w;

    let angle = wave * TAU;
    let band_angle = floor(angle * n_bands) / n_bands;
    let value = cos(band_angle);
    
    let thresholded = step(threshold, value);
    
    let normalized = (value + 1.0) * 0.5 * thresholded;
    let result = floor(normalized * n_bands) / (n_bands - 1.0);
    
    return vec4f(vec3f(result), 1.0);
}

fn fractal_reduce(pos: vec2f) -> f32 {
    let freq = params.b.x;
    let base_radius = params.b.y;
    let gradient_strength = params.b.z;

    var p = pos * freq;
    let cell = fract(p) - 0.5;

    let adjusted_radius = base_radius / freq; 
    let dist = length(cell);

    return smoothstep(
        adjusted_radius + gradient_strength, 
        adjusted_radius, 
        dist
    );
}

fn fractal_map(color_value: f32) -> vec4f {
    let invert = params.d.x;
    if invert == 1.0 {
        return vec4f(vec3f(1.0 - color_value), 1.0);
    }
    return vec4f(vec3f(color_value), 1.0);
}

fn mix_min(c1: vec4f, c2: vec4f) -> vec4f {
    return min(c1, c2);
}

fn mix_max(c1: vec4f, c2: vec4f) -> vec4f {
    return max(c1, c2);
}

fn powf(x: f32, y: f32) -> f32 {
    return sign(x) * exp(log(abs(x)) * y);
}

fn modulo(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}