struct Params {
    a: vec4f,
    b: vec4f,
    c: vec4f,
    d: vec4f,
    e: vec4f,
    f: vec4f,
    g: vec4f,
    h: vec4f,
    i: vec4f,
    j: vec4f,
    k: vec4f,
    l: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@group(1) @binding(0)
var source_sampler: sampler;

@group(1) @binding(1)
var feedback_tex: texture_2d<f32>;

@group(1) @binding(2)
var current_tex: texture_2d<f32>;

struct VsOut {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

struct VertexInput {
    @location(0) position: vec2f,
}

@vertex
fn vs_main(vert: VertexInput) -> VsOut {
    let p = vert.position;
    var out: VsOut;
    out.position = vec4f(p, 0.0, 1.0);
    out.uv = p * 0.5 + vec2f(0.5, 0.5);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4f {
    let current = textureSample(current_tex, source_sampler, in.uv).rgb;
    let angle = params.l.z;
    let centered = vec2f(in.uv.x, 1.0 - in.uv.y) - vec2f(0.5);
    let s = sin(angle);
    let c = cos(angle);
    let feedback_uv = vec2f(
        c * centered.x - s * centered.y,
        s * centered.x + c * centered.y,
    ) + vec2f(0.5);
    let feedback = textureSample(
        feedback_tex,
        source_sampler,
        clamp(feedback_uv, vec2f(0.0), vec2f(1.0)),
    ).rgb;
    let blend = sqrt(clamp(params.j.z, 0.0, 1.0));
    return vec4f(mix(current, feedback, blend), 1.0);
}
