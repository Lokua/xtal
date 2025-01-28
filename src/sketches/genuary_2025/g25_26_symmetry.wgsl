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

    // fractal_mix, distort_mix, wave_mix, fractal_count
    a: vec4f,

    // wave_freq, wave_scale, wave_x, wave_y
    b: vec4f,

    // distort_freq, signal_mix, unused, fractal_scale
    c: vec4f,

    // signal_contrast, signal_steps, fractal_color_scale, unused
    d: vec4f,

    // ....unused
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
    let fractal_mix = params.a.x;
    let distort_mix = params.a.y;
    let wave_mix = params.a.z;

    let p = correct_aspect(position);

    let fractal = fractal_reduce(p);
    let distort = distort_reduce(p);
    let wave = wave_reduce(p);
    var reduced = 
        fractal * fractal_mix + 
        distort * distort_mix + 
        wave * wave_mix;
    
    let total_mix = fractal_mix + distort_mix + wave_mix;
    reduced = select(reduced, reduced / total_mix, total_mix > 1.0);

    return vec4f(vec3f(map_signal(reduced)), 1.0);
}

fn map_signal(value: f32) -> vec3f {
    let contrast = params.d.x * 100.0;
    let steps = (params.d.y * 100.0) + 1.0;
    let signal_mix = params.c.y; 

    let transformed = 
        value * (1.0 - signal_mix) + 
        sin(value * PI) * 
        signal_mix;
    
    let contrasted = pow(transformed, contrast);
    let result = floor(contrasted * steps) / steps;

    return vec3f(result);
}

fn wave_reduce(p: vec2f) -> f32 {
    let frequency = params.b.x;
    let scale = params.b.y * 10.0;
    let x_frequency = params.b.z * 10.0;
    let y_frequency = params.b.w * 10.0;
    
    let d = length(p);
    let radial_wave = tanh(d * scale - frequency * PI);
    let horizontal_wave = cos(p.x * x_frequency);
    let vertical_wave = cosh(p.y * y_frequency);
    
    return radial_wave + horizontal_wave + vertical_wave;
}

fn distort_reduce(pos: vec2f) -> f32 {
    let freq = params.c.x * 20.0;
    let phase = 0.0;
    var p = vec2f(pos);
    p *= tan(p * freq + phase);
    return length(p);
}

fn fractal_reduce(pos: vec2f) -> f32 {
    let count = params.a.w * 20.0; 
    let scale = params.c.w;
    let color_scale = params.d.z;
    
    var p = pos * scale;
    var color = 0.0;
    let MAX_ITERATIONS = 20;
    
    for (var i = 0; i < MAX_ITERATIONS; i++) {
        let weight = 1.0 - smoothstep(count - 1.0, count, f32(i));
        if (weight <= 0.0) { break; }
        
        p = abs(p) * 2.0 - 1.0;
        let len = max(length(p), 0.001);
        color += (color_scale / len) * weight;
    }
    
    return color / count;
}

// --- IMPL DETAILS

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.resolution.x;
    let h = params.resolution.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}

fn modulo(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}

fn mix_min(c1: vec4f, c2: vec4f) -> vec4f {
    return min(c1, c2);
}

fn mix_max(c1: vec4f, c2: vec4f) -> vec4f {
    return max(c1, c2);
}
