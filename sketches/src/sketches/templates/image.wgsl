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

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var positions = array<vec2f, 4>(
        vec2f(-1.0, -1.0),
        vec2f(1.0, -1.0),
        vec2f(-1.0, 1.0),
        vec2f(1.0, 1.0),
    );

    let p = positions[vertex_index];

    var out: VsOut;
    out.position = vec4f(p, 0.0, 1.0);
    out.uv = p * 0.5 + vec2f(0.5, 0.5);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4f {
    let t = params.a.z;
    let zoom = max(0.2, params.a.w);
    let spin = params.b.x;
    let chroma = clamp(params.b.y, 0.0, 1.0);

    let centered = in.uv - vec2f(0.5, 0.5);
    let angle = spin * 0.35 * sin(t * 0.6);

    let ca = cos(angle);
    let sa = sin(angle);
    let rotated = vec2f(
        centered.x * ca - centered.y * sa,
        centered.x * sa + centered.y * ca,
    );

    let uv = rotated / zoom + vec2f(0.5, 0.5);
    let shift = vec2f(0.01 * chroma, 0.0);

    let r = textureSample(tex, tex_sampler, uv + shift).r;
    let g = textureSample(tex, tex_sampler, uv).g;
    let b = textureSample(tex, tex_sampler, uv - shift).b;

    return vec4f(r, g, b, 1.0);
}
