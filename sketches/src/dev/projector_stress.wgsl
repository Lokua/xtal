const TAU: f32 = 6.28318530718;

struct Params {
    // width, height, beats, unused
    a: vec4f,

    // stress, unused, unused, unused
    b: vec4f,

    c: vec4f,
    d: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

struct VsOut {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
}

struct VertexInput {
    @location(0) position: vec2f,
}

@vertex
fn vs_main(vert: VertexInput) -> VsOut {
    var out: VsOut;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.pos = vert.position;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4f {
    let width = max(params.a.x, 1.0);
    let height = max(params.a.y, 1.0);
    let aspect = width / height;
    let time = params.a.z * 0.32;
    let stress = u32(clamp(params.b.x, 0.0, 900.0));

    var p = in.pos;
    p.x *= aspect;

    // Animated stress work. It remains visually subtle, but forces the patch
    // to become expensive enough to compare Projector Mode quality levels.
    var workload = 0.0;
    var q = p;
    for (var i: u32 = 0u; i < stress; i = i + 1u) {
        let fi = f32(i);
        q = rotate(q, 0.007 + 0.00003 * fi);
        q = abs(q) / max(dot(q, q), 0.18) - vec2f(0.72, 0.64);
        workload += sin(length(q) * 4.0 + fi * 0.031 + time);
        workload += (hash(q + vec2f(fi)) - 0.5) * 0.08;
    }

    var stress_warp = 0.0;
    if (stress > 0u) {
        stress_warp = workload / f32(stress);
    }

    // A simple moving weave inspired by the Core interference patch. The
    // narrow animated contours make scaling quality differences obvious.
    let drift = vec2f(0.18 * cos(time * 0.7), 0.14 * sin(time * 0.9));
    let wave_p = rotate(p + drift, 0.22 * sin(time * 0.45));
    let curve_x = 0.18 * sin(wave_p.y * 5.0 - time * 1.7);
    let curve_y = 0.16 * cos(wave_p.x * 4.0 + time * 1.3);

    let wave_a = sin((wave_p.x + curve_x + stress_warp * 0.035) * 34.0 - time * 3.1);
    let wave_b = sin((wave_p.y + curve_y - stress_warp * 0.025) * 42.0 + time * 2.5);

    let moving_center = vec2f(0.28 * cos(time * 0.8), 0.22 * sin(time * 0.6));
    let radius = length(p - moving_center);
    let rings = sin(radius * 78.0 - time * 5.0 + stress_warp * 0.6);

    let weave_a = line(wave_a, 0.11);
    let weave_b = line(wave_b, 0.10);
    let ring_lines = line(rings, 0.09);

    let cyan = vec3f(0.10, 0.92, 1.00);
    let magenta = vec3f(1.00, 0.16, 0.72);
    let gold = vec3f(1.00, 0.78, 0.14);

    var color = vec3f(0.008, 0.012, 0.022);
    color += cyan * weave_a;
    color += magenta * weave_b;
    color += gold * ring_lines * 0.82;
    color += vec3f(0.92, 0.96, 1.0) * weave_a * weave_b * 0.85;

    let vignette = smoothstep(1.45, 0.32, length(p));
    return vec4f(color * (0.62 + 0.38 * vignette), 1.0);
}

fn rotate(p: vec2f, angle: f32) -> vec2f {
    let c = cos(angle);
    let s = sin(angle);
    return vec2f(p.x * c - p.y * s, p.x * s + p.y * c);
}

fn line(value: f32, width: f32) -> f32 {
    let edge = max(fwidth(value), 0.0005);
    return 1.0 - smoothstep(width, width + edge, abs(value));
}

fn hash(p: vec2f) -> f32 {
    return fract(sin(dot(p, vec2f(127.1, 311.7))) * 43758.5453);
}
