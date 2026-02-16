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
    let resolution = vec2f(max(params.a.x, 1.0), max(params.a.y, 1.0));
    let p = (in.uv * resolution - 0.5 * resolution) / resolution.y;

    let beats = params.a.z;
    let radius = max(0.05, params.a.w);

    let pulse = 0.55 + 0.45 * sin(beats * 2.0);
    let d = length(p);
    let ring = smoothstep(radius, radius - 0.015, d);
    let bg = vec3f(0.04, 1.0, 0.08);
    let fg = vec3f(0.15, 0.85, 0.0) * pulse;

    let color = mix(bg, fg, ring);
    return vec4f(color, 1.0);
}
