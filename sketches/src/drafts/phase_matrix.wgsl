const TAU: f32 = 6.283185307179586;

const ENGINE_CELLS: i32 = 0;
const ENGINE_STRIPES: i32 = 1;
const ENGINE_RINGS: i32 = 2;
const ENGINE_PLASMA: i32 = 3;
const ENGINE_SHARDS: i32 = 4;
const ENGINE_TUNNEL: i32 = 5;

const PALETTE_NEON: i32 = 0;
const PALETTE_ACID: i32 = 1;
const PALETTE_INFRARED: i32 = 2;
const PALETTE_CYANOTYPE: i32 = 3;
const PALETTE_MONO: i32 = 4;
const PALETTE_EMBER: i32 = 5;
const PALETTE_VIRIDIAN: i32 = 6;
const PALETTE_DUSK: i32 = 7;
const PALETTE_GRAPHITE: i32 = 8;
const PALETTE_CORAL: i32 = 9;
const PALETTE_AURORA: i32 = 10;
const PALETTE_SEPIA: i32 = 11;

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, beats, engine_a
    a: vec4f,
    // engine_b, engine_mix, beat_rate, beat_offset
    b: vec4f,
    // accent, complexity, zoom, warp
    c: vec4f,
    // swirl, twist, rotation, drift_x
    d: vec4f,
    // drift_y, kaleidoscope, mirror, line_mix
    e: vec4f,
    // edge_mix, cell_density, stripe_density, ring_density
    f: vec4f,
    // glitch_mix, glitch_scale, echo_mix, quantize_levels
    g: vec4f,
    // palette, hue_shift, saturation, brightness
    h: vec4f,
    // contrast, gamma, invert, color_scroll
    i: vec4f,
    // grain, vignette, bloom, scanlines
    j: vec4f,
    // strobe_on, strobe_mix, strobe_rate, soft_clip
    k: vec4f,
    // auto_mix, auto_mix_amount, pause_motion, master_gain
    l: vec4f,
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
    let resolution = vec2f(max(params.a.x, 1.0), max(params.a.y, 1.0));
    let uv = position * 0.5 + vec2f(0.5);

    let engine_a = i32(params.a.w);
    let engine_b = i32(params.b.x);
    let manual_mix = clamp(params.b.y, 0.0, 1.0);
    let beat_span = max(params.b.z, 0.001);
    let beat_offset = params.b.w;

    let accent = max(params.c.x, 0.0);
    let complexity = max(params.c.y, 0.001);
    let zoom = max(params.c.z, 0.001);
    let warp = max(params.c.w, 0.0);

    let swirl = params.d.x;
    let twist = params.d.y;
    let rotation = params.d.z;
    let drift_x = params.d.w;

    let drift_y = params.e.x;
    let kaleidoscope = max(round(params.e.y), 1.0);
    let mirror = params.e.z > 0.5;
    let line_mix = clamp(params.e.w, 0.0, 1.0);

    let edge_mix = clamp(params.f.x, 0.0, 1.0);
    let cell_density = max(params.f.y, 0.001);
    let stripe_density = max(params.f.z, 0.001);
    let ring_density = max(params.f.w, 0.001);

    let glitch_mix = clamp(params.g.x, 0.0, 1.0);
    let glitch_scale = max(params.g.y, 1.0);
    let echo_mix = clamp(params.g.z, 0.0, 1.0);
    let quantize_levels = max(2.0, round(params.g.w));

    let palette = i32(params.h.x);
    let hue_shift = params.h.y;
    let saturation = max(params.h.z, 0.0);
    let brightness = max(params.h.w, 0.0);

    let contrast = max(params.i.x, 0.001);
    let gamma = max(params.i.y, 0.001);
    let invert = params.i.z > 0.5;
    let color_scroll = params.i.w;

    let grain = max(params.j.x, 0.0);
    let vignette = max(params.j.y, 0.0);
    let bloom = clamp(params.j.z, 0.0, 1.0);
    let scanlines = clamp(params.j.w, 0.0, 1.0);

    let strobe_on = params.k.x > 0.5;
    let strobe_mix = clamp(params.k.y, 0.0, 1.0);
    let strobe_span = max(params.k.z, 0.001);
    let soft_clip = clamp(params.k.w, 0.0, 1.0);

    let auto_mix = clamp(params.l.x, 0.0, 1.0);
    let auto_mix_amount = clamp(params.l.y, 0.0, 1.0);
    let pause_motion = params.l.z > 0.5;
    let master_gain = max(params.l.w, 0.0);

    var beats = params.a.z / beat_span + beat_offset;
    if (pause_motion) {
        beats = beat_offset;
    }

    let beat_phase = fract(beats);
    let beat_env = pow(1.0 - abs(beat_phase * 2.0 - 1.0), 2.0);
    let beat_pulse = 1.0 + accent * beat_env;

    var p = position;
    p.x *= resolution.x / resolution.y;
    p += vec2f(drift_x, drift_y) * beats * 0.12;

    let swirl_angle = rotation + swirl * length(p);
    p = rotate2(p, swirl_angle);

    let twist_vec = vec2f(
        sin(p.y * twist + beats * 0.37),
        cos(p.x * twist - beats * 0.29),
    );
    p += twist_vec * 0.25 * warp;

    p *= zoom;

    let warp_a = fbm2(
        p * (0.8 + complexity * 0.25) + vec2f(beats * 0.23, -beats * 0.17),
    );
    let warp_b = fbm2(
        p.yx * (1.1 + complexity * 0.18) + vec2f(-beats * 0.19, beats * 0.21),
    );
    p += (vec2f(warp_a, warp_b) - vec2f(0.5)) * warp * 1.4;

    p = kaleido(p, kaleidoscope, mirror);

    let blend = mix(manual_mix, auto_mix, auto_mix_amount);
    let b_lane_rot = rotate2(p, 0.73 + 0.17 * sin(beats * 0.23));
    let b_lane_r = max(length(b_lane_rot), 0.0001);
    let b_lane_p = b_lane_rot * (
        1.0 + 0.12 * sin(b_lane_r * 6.0 - beats * 0.37)
    );
    let b_lane_t = beats + 1.618;
    let b_cell_density = cell_density * 1.11;
    let b_stripe_density = stripe_density * 0.93 + 0.41;
    let b_ring_density = ring_density * 1.07;
    let b_complexity = complexity * 0.94 + 0.06;

    let field_a = engine_field(
        engine_a,
        p,
        beats,
        cell_density,
        stripe_density,
        ring_density,
        complexity,
    );
    let field_b = engine_field(
        engine_b,
        b_lane_p,
        b_lane_t,
        b_cell_density,
        b_stripe_density,
        b_ring_density,
        b_complexity,
    );

    var field = mix(field_a, field_b, blend);
    field = clamp(field * beat_pulse, 0.0, 1.0);

    let echo_a_p = rotate2(p, 0.19) * 1.03;
    let echo_b_p = rotate2(b_lane_p, -0.27) * 0.97;

    let echo_a = engine_field(
        engine_a,
        echo_a_p,
        beats - 0.31,
        cell_density,
        stripe_density,
        ring_density,
        complexity,
    );
    let echo_b = engine_field(
        engine_b,
        echo_b_p,
        b_lane_t - 0.47,
        b_cell_density,
        b_stripe_density,
        b_ring_density,
        b_complexity,
    );
    let echo_field = mix(echo_a, echo_b, blend);
    field = mix(field, echo_field, echo_mix * 0.65);

    let blocks = max(1.0, glitch_scale);
    let block_cell = floor((uv + vec2f(beats * 0.07, -beats * 0.03)) * blocks);
    let gate_noise = hash21(block_cell + vec2f(floor(beats * 3.0)));
    let gate = step(0.82, gate_noise);
    let shift = (hash21(block_cell + vec2f(19.7, 43.2)) - 0.5) * 0.65;
    let glitch_field = fract(field + shift + gate_noise * 0.35);
    field = mix(field, glitch_field, glitch_mix * gate);

    let contour_freq = 2.0 + cell_density * 0.6;
    let contour = abs(fract(field * contour_freq) - 0.5) * 2.0;
    let line_strength = pow(1.0 - clamp(contour, 0.0, 1.0), 1.0 + complexity * 2.5);

    let grad = length(vec2f(dpdx(field), dpdy(field)));
    let edge_strength = smoothstep(0.03, 0.35, grad * complexity * 16.0);

    var shaped = mix(field, line_strength, line_mix);
    shaped = mix(shaped, edge_strength, edge_mix);

    shaped = floor(shaped * quantize_levels) / quantize_levels;
    shaped = clamp(shaped, 0.0, 1.0);

    let palette_t = fract(shaped + hue_shift + color_scroll * beats * 0.08);
    var color = sample_palette(palette, palette_t);

    let bloom_color = sample_palette(palette, fract(palette_t + 0.08));
    let glow = smoothstep(0.58, 1.0, shaped);
    color += bloom_color * glow * bloom;

    let luma = dot(color, vec3f(0.2126, 0.7152, 0.0722));
    color = mix(vec3f(luma), color, saturation);

    color = 0.5 + (color - 0.5) * contrast;
    color *= brightness;

    let inv_gamma = 1.0 / gamma;
    color = pow(max(color, vec3f(0.0)), vec3f(inv_gamma));

    let scan = 0.5 + 0.5 * sin(uv.y * resolution.y * 0.9 + beats * 5.0);
    let scan_mix = mix(1.0, 0.8 + 0.2 * scan, scanlines);
    color *= scan_mix;

    let grain_noise = hash21(
        uv * resolution + vec2f(beats * 37.1, -beats * 29.4),
    ) - 0.5;
    color += grain_noise * grain;

    let radial = dot(position, position);
    color *= exp(-radial * vignette);

    if (strobe_on) {
        let cycle = 0.5 + 0.5 * sin((beats / strobe_span) * TAU);
        let strobe_gain = mix(1.0, mix(0.12, 1.0, cycle), strobe_mix);
        color *= strobe_gain;
    }

    let clipped = color / (vec3f(1.0) + color);
    color = mix(color, clipped, soft_clip);
    color *= master_gain;

    if (invert) {
        color = vec3f(1.0) - color;
    }

    color = clamp(color, vec3f(0.0), vec3f(1.0));
    return vec4f(color, 1.0);
}

fn engine_field(
    mode: i32,
    p: vec2f,
    t: f32,
    cell_density: f32,
    stripe_density: f32,
    ring_density: f32,
    complexity: f32,
) -> f32 {
    if (mode == ENGINE_CELLS) {
        let q = p * cell_density;
        let cell = floor(q);
        let local = fract(q) - 0.5;

        var min_dist = 100.0;

        for (var y: i32 = -1; y <= 1; y = y + 1) {
            for (var x: i32 = -1; x <= 1; x = x + 1) {
                let offset = vec2f(f32(x), f32(y));
                let id = cell + offset;
                let jitter = hash22(id) - 0.5;
                let wobble = vec2f(
                    sin(t * 0.7 + id.x * 1.17),
                    cos(t * 0.63 + id.y * 1.41),
                ) * 0.3;
                let point = offset + jitter * (0.7 + complexity * 0.3) + wobble;
                min_dist = min(min_dist, length(local - point));
            }
        }

        return clamp(1.0 - min_dist * 1.85, 0.0, 1.0);
    }

    if (mode == ENGINE_STRIPES) {
        let freq = stripe_density * mix(0.4, 1.6, clamp(complexity * 0.4, 0.0, 1.0));
        let q = p + vec2f(
            0.15 * sin(p.y * 2.1 + t * 0.6),
            0.12 * cos(p.x * 2.4 - t * 0.5),
        );
        let s1 = sin(q.x * freq + t * 1.3);
        let s2 = sin((q.x + q.y * 0.7) * freq * 0.6 - t * 0.8);
        let s3 = sin((q.x - q.y) * freq * 0.35 + t * 1.6);
        return clamp(0.5 + 0.5 * (s1 * 0.55 + s2 * 0.3 + s3 * 0.15), 0.0, 1.0);
    }

    if (mode == ENGINE_RINGS) {
        let r = max(length(p), 0.001);
        let dir = p / r;
        let spoke_count = i32(clamp(
            round(4.0 + ring_density * 0.35),
            1.0,
            32.0,
        ));
        let radial = sin(r * ring_density * 2.0 - t * 2.2);
        let spokes = angular_wave(dir, spoke_count, t * 0.7);
        let ripple = sin((r + spokes * 0.08) * ring_density * 1.3 + t * 1.8);
        return clamp(0.5 + radial * 0.35 + ripple * 0.25, 0.0, 1.0);
    }

    if (mode == ENGINE_PLASMA) {
        let q = p * (1.1 + complexity * 0.4);
        let n0 = fbm2(q * 1.1 + vec2f(t * 0.13, -t * 0.11));
        let n1 = fbm2((q + vec2f(4.2, -3.1)) * 2.0 + vec2f(-t * 0.09, t * 0.16));
        let w = sin((q.x + q.y) * (2.0 + stripe_density * 0.1) + t);
        let value = n0 * 0.55 + n1 * 0.35 + (0.5 + 0.5 * w) * 0.1;
        return clamp(value, 0.0, 1.0);
    }

    if (mode == ENGINE_SHARDS) {
        let q = p * (2.5 + complexity);
        let r = length(q);
        let a = atan2(q.y, q.x);
        let facets = max(3.0, floor(3.0 + cell_density * 0.5));
        let angle_step = TAU / facets;
        let snapped = floor(a / angle_step + 0.5) * angle_step;
        let dir = vec2f(cos(snapped), sin(snapped));
        let plane = dot(q, dir);

        let cuts = abs(fract(plane * 0.45 + t * 0.3) - 0.5) * 2.0;
        let ridges = pow(1.0 - clamp(cuts, 0.0, 1.0), 1.5 + complexity);

        let cracks = abs(sin(a * facets + r * 1.8 - t * 1.2));
        return clamp(ridges * 0.75 + (1.0 - cracks) * 0.25, 0.0, 1.0);
    }

    if (mode == ENGINE_TUNNEL) {
        let r = max(length(p), 0.001);
        let dir = p / r;
        let spoke_count = i32(clamp(
            round(8.0 + stripe_density * 0.3),
            1.0,
            32.0,
        ));
        let rings = sin((1.0 / r) * (3.0 + ring_density) - t * 2.0);
        let swirl = angular_wave(dir, spoke_count, t * 1.1);
        let drift = fbm2(p * 2.0 + vec2f(t * 0.4, -t * 0.2));
        let value = 0.5 + rings * 0.35 + swirl * 0.2 + (drift - 0.5) * 0.15;
        return clamp(value, 0.0, 1.0);
    }

    return 0.0;
}

fn sample_palette(palette: i32, t: f32) -> vec3f {
    if (palette == PALETTE_NEON) {
        return gradient4(
            t,
            vec3f(0.05, 0.06, 0.15),
            vec3f(0.16, 0.43, 0.75),
            vec3f(0.79, 0.30, 0.67),
            vec3f(0.99, 0.72, 0.28),
        );
    }

    if (palette == PALETTE_ACID) {
        return gradient4(
            t,
            vec3f(0.08, 0.10, 0.06),
            vec3f(0.36, 0.62, 0.25),
            vec3f(0.82, 0.79, 0.30),
            vec3f(0.20, 0.46, 0.34),
        );
    }

    if (palette == PALETTE_INFRARED) {
        return gradient4(
            t,
            vec3f(0.09, 0.02, 0.03),
            vec3f(0.42, 0.05, 0.09),
            vec3f(0.86, 0.26, 0.17),
            vec3f(0.99, 0.82, 0.62),
        );
    }

    if (palette == PALETTE_CYANOTYPE) {
        return gradient4(
            t,
            vec3f(0.01, 0.03, 0.10),
            vec3f(0.02, 0.11, 0.30),
            vec3f(0.10, 0.30, 0.58),
            vec3f(0.80, 0.86, 0.92),
        );
    }

    if (palette == PALETTE_EMBER) {
        return gradient4(
            t,
            vec3f(0.06, 0.02, 0.02),
            vec3f(0.28, 0.07, 0.03),
            vec3f(0.74, 0.24, 0.07),
            vec3f(0.98, 0.74, 0.34),
        );
    }

    if (palette == PALETTE_VIRIDIAN) {
        return gradient4(
            t,
            vec3f(0.02, 0.08, 0.07),
            vec3f(0.05, 0.33, 0.22),
            vec3f(0.33, 0.64, 0.43),
            vec3f(0.92, 0.90, 0.78),
        );
    }

    if (palette == PALETTE_DUSK) {
        return gradient4(
            t,
            vec3f(0.05, 0.06, 0.16),
            vec3f(0.21, 0.18, 0.43),
            vec3f(0.58, 0.28, 0.48),
            vec3f(0.95, 0.74, 0.62),
        );
    }

    if (palette == PALETTE_GRAPHITE) {
        return gradient4(
            t,
            vec3f(0.03, 0.04, 0.06),
            vec3f(0.16, 0.20, 0.26),
            vec3f(0.46, 0.52, 0.58),
            vec3f(0.91, 0.93, 0.94),
        );
    }

    if (palette == PALETTE_CORAL) {
        return gradient4(
            t,
            vec3f(0.02, 0.05, 0.10),
            vec3f(0.07, 0.32, 0.34),
            vec3f(0.79, 0.38, 0.32),
            vec3f(0.96, 0.82, 0.67),
        );
    }

    if (palette == PALETTE_AURORA) {
        return gradient4(
            t,
            vec3f(0.01, 0.03, 0.09),
            vec3f(0.10, 0.34, 0.24),
            vec3f(0.17, 0.62, 0.66),
            vec3f(0.74, 0.70, 0.93),
        );
    }

    if (palette == PALETTE_SEPIA) {
        return gradient4(
            t,
            vec3f(0.08, 0.05, 0.03),
            vec3f(0.27, 0.17, 0.10),
            vec3f(0.58, 0.41, 0.23),
            vec3f(0.92, 0.82, 0.65),
        );
    }

    let mono = 0.08 + 0.92 * t;
    return vec3f(mono) * vec3f(1.0, 0.98, 0.94);
}

fn gradient4(
    t: f32,
    c0: vec3f,
    c1: vec3f,
    c2: vec3f,
    c3: vec3f,
) -> vec3f {
    let x = fract(t);
    if (x < 0.33333334) {
        let u = smoothstep(0.0, 0.33333334, x);
        return mix(c0, c1, u);
    }

    if (x < 0.6666667) {
        let u = smoothstep(0.33333334, 0.6666667, x);
        return mix(c1, c2, u);
    }

    let u = smoothstep(0.6666667, 1.0, x);
    return mix(c2, c3, u);
}

fn kaleido(p: vec2f, sectors: f32, mirror: bool) -> vec2f {
    if (sectors <= 1.0) {
        return p;
    }

    let r = length(p);
    var a = atan2(p.y, p.x);
    let sector = TAU / sectors;

    if (mirror) {
        a = abs(fract(a / sector + 0.5) - 0.5) * sector;
    } else {
        a = fract(a / sector + 0.5) * sector - 0.5 * sector;
    }

    return vec2f(cos(a), sin(a)) * r;
}

fn rotate2(v: vec2f, angle: f32) -> vec2f {
    let s = sin(angle);
    let c = cos(angle);
    return vec2f(c * v.x - s * v.y, s * v.x + c * v.y);
}

fn angular_wave(dir: vec2f, count: i32, phase: f32) -> f32 {
    let n = clamp(count, 1, 32);
    var c = 1.0;
    var s = 0.0;

    for (var i: i32 = 0; i < 32; i = i + 1) {
        if (i >= n) {
            break;
        }

        let next_c = c * dir.x - s * dir.y;
        let next_s = s * dir.x + c * dir.y;
        c = next_c;
        s = next_s;
    }

    return s * cos(phase) + c * sin(phase);
}

fn fbm2(p: vec2f) -> f32 {
    var sum = 0.0;
    var amplitude = 0.5;
    var freq = 1.0;

    for (var i: i32 = 0; i < 4; i = i + 1) {
        sum += noise2(p * freq) * amplitude;
        freq = freq * 2.02;
        amplitude = amplitude * 0.5;
    }

    return clamp(sum / 0.9375, 0.0, 1.0);
}

fn noise2(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);

    let a = hash21(i);
    let b = hash21(i + vec2f(1.0, 0.0));
    let c = hash21(i + vec2f(0.0, 1.0));
    let d = hash21(i + vec2f(1.0, 1.0));

    let u = f * f * (3.0 - 2.0 * f);

    return mix(a, b, u.x) +
        (c - a) * u.y * (1.0 - u.x) +
        (d - b) * u.x * u.y;
}

fn hash21(p: vec2f) -> f32 {
    let h = sin(dot(p, vec2f(127.1, 311.7)));
    return fract(h * 43758.5453123);
}

fn hash22(p: vec2f) -> vec2f {
    let q = vec2f(
        dot(p, vec2f(127.1, 311.7)),
        dot(p, vec2f(269.5, 183.3)),
    );
    return fract(sin(q) * 43758.5453123);
}
