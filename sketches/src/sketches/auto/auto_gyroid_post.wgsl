struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
};

struct Params {
    a: vec4f,
    b: vec4f,
    c: vec4f,
    d: vec4f,
    e: vec4f,
    // f: reserved, reserved, chroma_mix, reserved
    f: vec4f,
    // g: reserved, reserved, reserved, chroma_px
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
var source_texture: texture_2d<f32>;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.uv = vert.position * 0.5 + 0.5;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let center = textureSample(source_texture, source_sampler, in.uv);
    let w = params.a.x;
    let h = params.a.y;
    let texel = vec2f(1.0 / w, 1.0 / h);
    let chroma_mix = clamp(params.f.z, 0.0, 1.0);
    let chroma_px = max(params.g.w, 0.0);

    if (chroma_mix <= 0.0 || chroma_px <= 0.0) {
        return center;
    }

    let to_center = in.uv - vec2f(0.5, 0.5);
    let dir = normalize(to_center + vec2f(1e-5, 0.0));
    let off = dir * texel * chroma_px;

    let r = sample_rgb(in.uv + off).r;
    let g = center.g;
    let b = sample_rgb(in.uv - off).b;
    let chroma = vec3f(r, g, b);
    let color = mix(center.rgb, chroma, chroma_mix);
    return vec4f(color, center.a);
}

fn sample_rgb(uv: vec2f) -> vec3f {
    return textureSample(source_texture, source_sampler, uv).rgb;
}
