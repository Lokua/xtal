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
var source_sampler: sampler;

@group(1) @binding(1)
var source_texture: texture_2d<f32>;

@group(1) @binding(2)
var feedback_texture: texture_2d<f32>;

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
    let fb_amt = params.e.z;

    let uv = in.uv;
    let p = in.position;

    let sample = textureSample(source_texture, source_sampler, uv);
    var fb = textureSample(feedback_texture, source_sampler, uv);

    let b = fb.b;
    fb.b = fb.r;
    fb.r = b;
    fb = fb * 0.8;
    
    var s = mix(sample, fb, fb_amt);
    s += sample;

    let wave_1 = sin(t + uv.x * dots) * 0.5 + 0.5;
    let wave_2 = cos(t + uv.y * dots) * 0.5 + 0.5;
    
    var color = vec3f(s.rgb);
    color *= wave_1 + wave_2;
    color = select(color, 1.0 - color, invert == 1.0);

    return vec4f(color, 1.0);
}


