// domain_warps.wgsl
//
// Purpose:
// Performable domain-warp shader tuned for low runtime cost.

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // a = (width, height, beats, frame_count)
    a: vec4f,
    // b = zoom, warp_amt, speed_beats, energy
    b: vec4f,
    // c = drop, pulse_beats, contrast, brightness
    c: vec4f,
    // d = threshold, invert, unused, unused
    d: vec4f,
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
    let beats = params.a.z;

    let zoom = max(0.001, params.b.x);
    let warp_amt = max(0.0, params.b.y);
    let speed_beats = max(0.125, params.b.z);
    let energy = clamp(params.b.w, 0.0, 1.0);

    let drop = clamp(params.c.x, 0.0, 1.0);
    let pulse_beats = max(0.125, params.c.y);
    let contrast = max(0.1, params.c.z);
    let brightness = max(0.0, params.c.w);
    let threshold = clamp(params.d.x, 0.0, 1.0);
    let invert = clamp(params.d.y, 0.0, 1.0);

    let beat_phase = fract(beats / pulse_beats);
    let beat_pulse = 1.0 - abs(beat_phase * 2.0 - 1.0);
    let pulse = (beat_pulse - 0.5) * 2.0;

    // 1.0 = 1 beat cycle, 4.0 = 4 beat cycle.
    let t = beats / speed_beats;
    let energy_warp = mix(0.55, 1.8, energy);
    let energy_speed = mix(0.7, 1.8, energy);
    let energy_pattern = mix(0.8, 2.0, energy);
    let drop_duck = drop;
    let detail_keep = 1.0 - drop_duck * 0.8;

    var uv = position * 0.5 * zoom;
    uv.x *= w / h;

    // Lightweight domain warp path.
    let warp1 = vec2f(
        noise2(uv * 1.6 + vec2f(0.0, t * (0.22 * energy_speed))),
        noise2(uv * 1.6 + vec2f(3.9, t * (0.19 * energy_speed)))
    );

    let uv2 = uv + (warp1 - 0.5) * (0.95 * warp_amt * energy_warp * detail_keep);

    let warp2 = vec2f(
        noise2(uv2 * 2.4 + vec2f(7.2, -t * (0.16 * energy_speed))),
        noise2(uv2 * 2.4 + vec2f(1.6, t * (0.13 * energy_speed)))
    );

    let uv3 = uv2 + (warp2 - 0.5) * (0.65 * warp_amt * energy_warp * detail_keep);
    let field = fbm2_fast(
        uv3 * (2.4 + 1.8 * energy_pattern) +
            vec2f(t * 0.10 * energy_speed, -t * 0.08 * energy_speed)
    );

    let rings = sin((length(uv3) * (6.0 + 10.0 * energy_pattern) - t * (0.8 + 1.3 * energy_speed)) + field * (1.6 + 4.2 * energy_pattern));
    let stripes = sin((uv3.x * (5.0 + 6.0 * energy_pattern) + uv3.y * (3.0 + 7.0 * energy_pattern)) + t * (0.7 + 1.5 * energy_speed) + field * (2.5 + 4.0 * energy_pattern));
    let pulse_lift = pulse * 0.08 * (0.4 + 0.6 * energy);

    let mix_val = clamp(
        field * (0.72 + 0.18 * energy) +
            rings * (0.20 * detail_keep) +
            stripes * (0.16 * detail_keep) +
            pulse_lift,
        0.0,
        1.0
    );
    var mono = mix_val;
    mono = clamp((mono - 0.5) * (contrast * mix(0.9, 1.3, energy)) + 0.5, 0.0, 1.0);
    mono = mix(mono, step(threshold, mono), 0.35);
    mono *= brightness * mix(1.0, 0.22, drop_duck);
    mono = mix(mono, 1.0 - mono, invert);

    return vec4f(vec3f(mono), 1.0);
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

fn fbm2_fast(p: vec2f) -> f32 {
    var value = 0.0;
    var amp = 0.5;
    var freq = 1.0;
    var q = p;

    for (var i = 0; i < 2; i++) {
        value += amp * noise2(q * freq);
        amp *= 0.5;
        freq *= 2.0;
        q = vec2f(q.y * 1.08 + q.x * 0.2, q.x * 0.92 - q.y * 0.15);
    }

    return value;
}
