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
var hatch_sampler: sampler;

@group(1) @binding(1)
var hatch_texture: texture_2d<f32>;

struct VertexInput {
    @location(0) position: vec2f,
}

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.uv = vert.position * 0.5 + vec2f(0.5);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let texture_effect_amount = params.h.x;
    let texture_copy_size = params.h.y;
    let texture_copy_darkness = params.h.z;
    let source = textureSample(hatch_texture, hatch_sampler, in.uv);
    let mirrored_uv = mirror_background_uv(
        in.uv,
        texture_copy_size,
    );
    let mirrored = textureSample(
        hatch_texture,
        hatch_sampler,
        mirrored_uv,
    );

    let source_ink = 1.0 - dot(
        source.rgb,
        vec3f(0.299, 0.587, 0.114),
    );
    let background_mask = 1.0 - smoothstep(0.02, 0.32, source_ink);
    let effect_mix = texture_effect_amount
        * texture_copy_darkness
        * background_mask;
    let layered = mix(source.rgb, mirrored.rgb, effect_mix);

    return vec4f(layered, source.a);
}

fn mirror_background_uv(uv: vec2f, size: f32) -> vec2f {
    var centered = uv * 2.0 - vec2f(1.0);
    centered.x = -centered.x;
    let enlarged = centered / max(size, 1.0);
    return enlarged * 0.5 + vec2f(0.5);
}
