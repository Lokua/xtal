struct Params {
    a: vec4f,
    b: vec4f,
    c: vec4f,
    d: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@group(1) @binding(0)
var tex_sampler: sampler;

@group(1) @binding(1)
var tex: texture_2d<f32>;

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
    let grade = max(0.0, params.b.y);

    let c = textureSample(tex, tex_sampler, in.uv);
    let boosted = mix(c.rgb, pow(c.rgb, vec3f(0.75, 0.75, 0.75)), clamp(grade, 0.0, 1.0));

    return vec4f(boosted, 1.0);
}
