struct Params {
    a: vec4f,
    b: vec4f,
    c: vec4f,
    d: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

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
    let freq = max(0.25, params.a.w);
    let p = in.uv * 2.0 - vec2f(1.0, 1.0);

    let wave = 0.5 + 0.5 * sin((p.x + p.y * 0.2) * 8.0 * freq + t * 1.5);
    let color = vec3f(wave, 0.4 + 0.6 * wave, 1.0 - wave);
    return vec4f(color, 1.0);
}
