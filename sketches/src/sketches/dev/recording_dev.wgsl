// recording_dev.wgsl
//
// Purpose:
// A realistic fullscreen shader benchmark for recording performance checks.
// This should run near realtime on modern laptops at 1080p while still doing
// enough animated work to expose recording regressions.

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // a = (width, height, time, frame_count)
    a: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.pos = vert.position;
    return out;
}

@fragment
fn fs_main(@location(0) position: vec2f) -> @location(0) vec4f {
    let w = params.a.x;
    let h = params.a.y;
    let t = params.a.z;

    var uv = position * 0.5;
    uv.x *= w / h;

    // Two-domain warp passes keep motion rich without deep nested loops.
    let warp1 = vec2f(
        fbm2(uv * 1.6 + vec2f(0.0, t * 0.22)),
        fbm2(uv * 1.6 + vec2f(3.9, t * 0.19))
    );

    let uv2 = uv + (warp1 - 0.5) * 0.85;

    let warp2 = vec2f(
        fbm2(uv2 * 2.4 + vec2f(7.2, -t * 0.16)),
        fbm2(uv2 * 2.4 + vec2f(1.6,  t * 0.13))
    );

    let field = fbm2(uv2 * 3.2 + (warp2 - 0.5) * 0.9 + vec2f(t * 0.10, -t * 0.08));

    let rings = sin((length(uv2) * 9.0 - t * 1.35) + field * 2.8);
    let stripes = sin((uv2.x * 8.0 + uv2.y * 6.0) + t * 1.2 + field * 5.0);

    let mix_val = clamp(field * 0.75 + rings * 0.2 + stripes * 0.15, 0.0, 1.0);
    var color = palette(mix_val + t * 0.03);

    // Mild vignette and contrast shaping.
    let vignette = smoothstep(1.35, 0.25, length(uv));
    color *= vignette;
    color = pow(color, vec3f(0.95));

    return vec4f(color, 1.0);
}

fn hash21(p: vec2f) -> f32 {
    let h = dot(p, vec2f(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

fn noise2(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let a = hash21(i + vec2f(0.0, 0.0));
    let b = hash21(i + vec2f(1.0, 0.0));
    let c = hash21(i + vec2f(0.0, 1.0));
    let d = hash21(i + vec2f(1.0, 1.0));

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm2(p: vec2f) -> f32 {
    var value = 0.0;
    var amp = 0.5;
    var freq = 1.0;
    var q = p;

    // Keep this small on purpose: benchmark should be demanding, not absurd.
    for (var i = 0; i < 4; i++) {
        value += amp * noise2(q * freq);
        amp *= 0.5;
        freq *= 2.0;
        q = vec2f(q.y * 1.08 + q.x * 0.2, q.x * 0.92 - q.y * 0.15);
    }

    return value;
}

fn palette(t: f32) -> vec3f {
    let a = vec3f(0.50, 0.0, 0.56);
    let b = vec3f(0.46, 0.5, 0.36);
    let c = vec3f(0.8, 0.8, 0.4);
    let d = vec3f(0.00, 0.18, 0.33);
    return a + b * cos(6.28318 * (c * t + d));
}

