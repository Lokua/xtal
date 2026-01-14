struct VertexInput {
    @location(0) position: vec3f
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f
};

struct Params {
    a: vec4f,
    b: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@group(1) @binding(0)
var portrait_sampler: sampler;

@group(1) @binding(1)
var portrait_texture: texture_2d<f32>;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.uv = (vert.position.xy + 1.0) * 0.5;
    out.uv.y = 1.0 - out.uv.y;

    let grid_scale = params.b.z;
    let pos = vert.position.xy * grid_scale;

    out.clip_position = vec4f(pos, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    let sampled = textureSample(portrait_texture, portrait_sampler, input.uv);
    return vec4f(sampled.rgb, 1.0);
}
