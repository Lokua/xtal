struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
};

struct Params {
    // w, h, ..unused
    resolution: vec4f,
    // time, distort_amount, ..unused
    a: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

// This will be our texture from the first pass
@group(1) @binding(0)
var source_texture: texture_2d<f32>;
@group(1) @binding(1)
var source_sampler: sampler;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    // Convert from vertex position [-1,1] to UV coordinates [0,1]
    out.uv = vert.position * 0.5 + 0.5;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    // For now, just sample the texture directly to verify it works
    var color = textureSample(source_texture, source_sampler, in.uv);
    color.g = 0.2;
    return color;
}