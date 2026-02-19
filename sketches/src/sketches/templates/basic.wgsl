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

    let pulse = 0.55 + 0.45 * sin(beats * 4.0);
    let d = length(p);
    let ring = smoothstep(radius, radius - 0.015, d);
    let bg = vec3f(0.00, 0.0, 0.8);
    let fg = vec3f(0.15, 0.85, 0.0) * pulse;

    var color = mix(bg, fg, ring);

    // Adjustable fragment stress test to validate FPS/perf telemetry.
    let stress = u32(clamp(params.b.x, 0.0, 300.0));
    var p2 = p;
    var accum = vec3f(0.0);
    for (var i: u32 = 0u; i < stress; i = i + 1u) {
        let t = f32(i) * 0.021 + beats * 0.35;
        let c = cos(t);
        let s = sin(t);
        p2 = vec2f(p2.x * c - p2.y * s, p2.x * s + p2.y * c);
        accum += 0.5 + 0.5 * cos(vec3f(t, t * 1.31, t * 1.73) + vec3f(p2, p2.x + p2.y));
    }

    if (stress > 0u) {
        let noisy = accum / f32(stress);
        color = mix(color, noisy, 0.25);
    }

    return vec4f(color, 1.0);
}
