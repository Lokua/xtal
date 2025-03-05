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
    a: vec4f,
    b: vec4f,
    c: vec4f,
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
    let p = correct_aspect(position);

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

    return mapped;
}

fn wave_reduce(p: vec2f) -> f32 {
    let wave_phase = params.c.x * 10.0;
    let wave_dist = params.c.y * 10.0;
    let wave_x_freq = params.c.z * 10.0;
    let wave_y_freq = params.c.w * 10.0;
    let d = length(p);
    let wave1 = tanh(d * wave_dist - wave_phase * PI);
    let wave2 = cos(p.x * wave_x_freq);
    let wave3 = cosh(p.y * wave_y_freq);
    return wave1 + wave2 + wave3;
}

fn wave_map(wave: f32) -> vec4f {
    let color = vec3(0.5 + (cos(wave) * 0.5));
    return vec4f(color, 1.0);
}

fn distort_reduce(pos: vec2f) -> f32 {
    let dist_freq = params.a.x;
    var p = vec2f(pos);
    p *= tan(p * dist_freq);
    return length(p);
}
fn distort_map(d: f32) -> vec4f {
    return vec4f(vec3(smoothstep(0.0, 1.0, d)), 1.0);
}

fn fractal_reduce(pos: vec2f) -> f32 {
    let fract_count = params.d.x; 
    let scale = 1.0;
    
    var p = pos * scale;
    var color = 0.0;
    let MAX_ITERATIONS = 20;
    
    for (var i = 0; i < MAX_ITERATIONS; i++) {
        let weight = 1.0 - smoothstep(fract_count - 1.0, fract_count, f32(i));
        if (weight <= 0.0) { break; }
        
        p = abs(p) * 2.0 - 1.0;
        let len = max(length(p), 0.001);
        color += (1.0 / len) * weight;
    }
    
    return color / fract_count;
}

fn fractal_map(color_value: f32) -> vec4f {
    let fract_steps = 1.0;
    let contrast = 300.0;
    let contrasted = pow(color_value, contrast);
    let stepped = floor(contrasted * fract_steps) / fract_steps;
    return vec4f(vec3f(stepped), 1.0);
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.resolution.x;
    let h = params.resolution.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}
