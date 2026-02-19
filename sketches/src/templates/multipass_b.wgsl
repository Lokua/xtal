struct Params {
    a: vec4f,
    b: vec4f,
    c: vec4f,
    d: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@group(1) @binding(0)
var pass_sampler: sampler;

@group(1) @binding(1)
var pass_tex: texture_2d<f32>;

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
    let t = params.a.z;
    let blur = max(0.0, params.b.x);

    let uv = in.uv;
    let shift = vec2f(0.02 * blur, 0.014 * blur);

    let c0 = textureSample(pass_tex, pass_sampler, uv);
    let c1 = textureSample(pass_tex, pass_sampler, uv + shift);
    let c2 = textureSample(pass_tex, pass_sampler, uv - shift);
    let c3 = textureSample(pass_tex, pass_sampler, uv + vec2f(shift.x, -shift.y));
    let c4 = textureSample(pass_tex, pass_sampler, uv + vec2f(-shift.x, shift.y));

    let mixed = (c0 + c1 + c2 + c3 + c4) / 5.0;

    let vignette = smoothstep(1.05, 0.25, length(uv * 2.0 - vec2f(1.0, 1.0)));
    let pulse = 0.85 + 0.15 * sin(t * 2.0);
    let glow = 1.0 + 0.35 * blur;

    return vec4f(mixed.rgb * vignette * pulse * glow, 1.0);
}
