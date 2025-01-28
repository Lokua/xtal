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
    let p = correct_aspect(position);
    let t1 = params.a.x;
    let t2 = params.a.y;
    let t3 = params.a.z;
    let t4 = params.a.w;
    let b1 = params.b.x;

    let p1 = vec2f(1.0 - t1, 1.0 - t1);
    let p2 = vec2f(1.0 - t2, -1.0 + t2);
    let p3 = vec2f(-1.0 + t3, -1.0 + t3);
    let p4 = vec2f(-1.0 + t4, 1.0 - t4);
    let p5 = vec2f(0.0);

    let scale = 1.0;
    let d1 = length(p - p1) / scale;
    let d2 = length(p - p2) / scale;
    let d3 = length(p - p3) / scale;
    let d4 = length(p - p4) / scale;
    let d5 = length(p - p5) / (scale * t2);

    let k = t1;
    
    // Mix each corner with the center point
    let mix1 = smin(d1, d5, k);
    let mix2 = smin(d2, d5, k);
    let mix3 = smin(d3, d5, k);
    let mix4 = smin(d4, d5, k);

    // Combine all mixed pairs
    let mix12 = smin(mix1, mix2, k);
    let mix34 = smin(mix3, mix4, k);
    let final_mix = smin(mix12, mix34, k);

    let brightness = 1000.0;
    let d = final_mix * brightness;

    let base_color = vec3f(
        0.5 + 0.5 * sin(p.x * 2000.0),
        0.5 + 0.5 * cos(p.y * 2000.0),
        0.5 + 0.5 * sin((p.x + p.y) * 2000.0)
    );
    
    // For areas where d is small (inside circles), use bright colors
    // For areas where d is large (background), fade to darker
    let circle_brightness = smoothstep(1.0, 0.9, d);  // Inverted and adjusted range
    var color = base_color * (0.3 + 0.99 * circle_brightness); 
    
    color = mix(color, 1.0 - color, b1);
    
    return vec4f(color, 1.0);
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