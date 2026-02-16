struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
};

struct Params {
    // ax, ay, az, aw
    a: vec4f,
    // bx, by, bz, bw
    b: vec4f,
    c: vec4f,
    d: vec4f,
};

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
    out.uv = vert.position * 0.5 + vec2f(0.5, 0.5);
    return out;
}

@fragment
fn fs_main(@location(0) uv: vec2f) -> @location(0) vec4f {
    let resolution = vec2f(max(params.a.x, 1.0), max(params.a.y, 1.0));
    let beats = params.a.z;

    let feedback_mix = clamp(params.a.w, 0.0, 0.999);
    let zoom = params.b.x;
    let ring_size = params.b.y;
    let hue_rate = params.b.z;

    let centered_uv = (uv - 0.5) * zoom + 0.5;
    let fb = textureSample(source_texture, source_sampler, centered_uv).rgb;

    var p = uv * 2.0 - 1.0;
    p.x *= resolution.x / resolution.y;

    let radius = ring_size + 0.08 * sin(beats * 2.0 + p.y * 5.0);
    let d = abs(length(p) - radius);
    let ring = smoothstep(0.02, 0.0, d);

    let glow = vec3f(
        0.5 + 0.5 * sin(beats * hue_rate + 0.0),
        0.5 + 0.5 * sin(beats * hue_rate + 2.1),
        0.5 + 0.5 * sin(beats * hue_rate + 4.2)
    ) * ring;

    let color = fb * feedback_mix + glow;
    return vec4f(color, 1.0);
}
