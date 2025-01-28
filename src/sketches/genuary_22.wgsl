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

    // t1, t2, t3, t4
    a: vec4f,

    // b1, b2, ..unused
    b: vec4f,
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
    let mix = params.b.x;
    let p = correct_aspect(position);
    let base = pattern_a(p);
    let color = dots(p, base);
    return vec4f(color, 1.0);
}

fn pattern_a(p: vec2f) -> vec3f {
    let t = pow(1.0 - (p.y + 1.0) * 0.5, 2.0);
    let a = vec3f(1.0, 0.1, 0.2);
    let b = vec3f(0.3, 0.0, 0.5);
    return mix(a, b, t);
}

fn dots(p: vec2f, base: vec3f) -> vec3f {
    // Create dot grid
    let freq = 50.0;
    let grid_p = p * freq;
    let dots_p = fract(grid_p) * 2.0 - 1.0;
    
    // Dots get smaller as they go up
    let t = (p.y + 1.0) * 0.5;
    let size = 0.7 * (1.0 - t);
    let dots = step(length(dots_p), size);
    
    // Brighter dots
    let dot_color = base * 1.5;
    return mix(base, dot_color, dots * 0.9);
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.resolution.x;
    let h = params.resolution.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}


fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = max(k - abs(a - b), 0.0) / k;
    return min(a, b) - h * h * k * 0.25;
}