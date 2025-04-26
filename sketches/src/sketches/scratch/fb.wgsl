struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
    @location(1) uv: vec2f,
};

struct Params {
    // w, h, fb
    a: vec4f,
    b: vec4f,
    c: vec4f,
    d: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@group(1) @binding(0)
var source_sampler: sampler;

@group(1) @binding(1)
var feedback_texture: texture_2d<f32>;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.pos = out.position.xy;
    out.uv = vert.position * 0.5 + 0.5;
    return out;
}

@fragment
fn fs_main(
    @location(0) pos: vec2f,
    @location(1) uv: vec2f
) -> @location(0) vec4f {
    let t = params.a.z;
    let fb = params.a.w;
    let p = correct_aspect(pos);

    let fb_sample = textureSample(feedback_texture, source_sampler, uv);
    
    let center = vec2f(sin(t) + tan(-t) * 0.125, 0.0) * 0.5;
    let radius = 0.5;
    let dist = length(p - center);
    
    var color = vec3f(0.0);

    if (dist < radius) {
        color = vec3f(sin(t) * 0.5 + 0.5, cos(t) * 0.5 + 0.5, 0.3);
    }
    
    color = mix(color, fb_sample.rgb, fb);

    return vec4f(color + (color * 0.125), 1.0);
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = params.a.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}

