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
    f: vec4f,
    g: vec4f,
    h: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@group(1) @binding(0)
var source_texture: texture_2d<f32>;

@group(1) @binding(1)
var source_sampler: sampler;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.uv = vert.position * 0.5 + 0.5;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let t = params.g.y;
    let invert = params.c.z;
    let dots = params.c.w;

    let uv = in.uv;
    let p = in.position;
    let sample = textureSample(source_texture, source_sampler, in.uv);

    let s1 = textureSample(source_texture, source_sampler, vec2f(uv.x + 0.125, uv.y));
    let s2 = textureSample(source_texture, source_sampler, vec2f(uv.x, uv.y + 0.125));
    let s3 = textureSample(source_texture, source_sampler, vec2f(uv.x - 0.125, uv.y));
    let s4 = textureSample(source_texture, source_sampler, vec2f(uv.x, uv.y - 0.125));
    var s = s1 + s2 + s3 + s4;

    let wave_1 = sin(t + uv.x * dots) * 0.5 + 0.5;
    let wave_2 = cos(t + uv.y * dots) * 0.5 + 0.5;
    
    var color = vec3f(sample.rgb);
    color *= wave_1 + wave_2;
    color = select(color, 1.0 - color, invert == 1.0);
    // color = mix(color, s.rgb, 0.5);
    // color *= s.rgb;
    // color = mix_dodge(color, s.rgb);

    return vec4f(color, 1.0);
}


