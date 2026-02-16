// Based on Tyler Hobbs' watercolor simulation article.

const TAU: f32 = 6.283185307;
const MAX_LAYERS: i32 = 64;
const MAX_OCTAVES: i32 = 4;

// Performance + look constants.
const BASE_OCTAVES: i32 = 3;
const DETAIL_OCTAVES: i32 = 2;
const VARIANCE_OCTAVES: i32 = 2;
const BASE_FREQ: f32 = 2.1;
const DETAIL_FREQ: f32 = 10.0;
const VARIANCE_SCALE: f32 = 3.4;
const VARIANCE_ANIM: f32 = 0.28;
const EDGE_SOFTNESS: f32 = 0.016;
const LAYER_DECAY_END: f32 = 0.72;
const ALT_BLOCK_SIZE: i32 = 4;
const ALT_COLOR_MIX: f32 = 0.26;
const MASK_SCROLL: vec2f = vec2f(0.01, -0.008);
const MASK_RADIUS: f32 = 0.115;
const MASK_SOFTNESS: f32 = 0.028;
const GRAIN_SCALE: vec2f = vec2f(430.0, 330.0);
const HALO_WIDTH: f32 = 0.12;
const HALO_STRENGTH: f32 = 0.78;
const BLEED_DRIFT: f32 = 0.22;
const BLEED_EXPAND: f32 = 0.14;
const BLEED_THETA_SHIFT: f32 = 0.7;
const BLEED_TIME_DRIFT: f32 = 0.08;
const COARSE_MASK_DENSITY_RATIO: f32 = 0.34;
const COARSE_MASK_WEIGHT: f32 = 0.52;
const CLOUD_MASK_WEIGHT: f32 = 0.42;

struct VertexInput {
    @location(0) position: vec2f,
}

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
}

struct Params {
    // w, h, beats, time_scale
    a: vec4f,
    // sides, base_radius, zoom, rotation_rate
    b: vec4f,
    // base_deform, detail_deform, layer_count, layer_alpha
    c: vec4f,
    // variance_strength, texture_strength, texture_density, top_hue
    d: vec4f,
    // bottom_hue, top_sat, top_val, bottom_sat
    e: vec4f,
    // bottom_val, paper_tint, grain_strength, unused
    f: vec4f,
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
    let time_scale = max(params.a.w, 0.0);
    let time = beats * time_scale;

    let sides = clamp(params.b.x, 3.0, 12.0);
    let base_radius = max(params.b.y, 0.02);
    let zoom = max(params.b.z, 0.05);
    let rotation_rate = params.b.w;

    let base_deform = max(params.c.x, 0.0);
    let detail_deform = max(params.c.y, 0.0);
    let layer_count = clamp(i32(params.c.z), 1, MAX_LAYERS);
    let layer_alpha = clamp(params.c.w, 0.0, 1.0);

    let variance_strength = clamp(params.d.x, 0.0, 1.0);
    let texture_strength = clamp(params.d.y, 0.0, 1.0);
    let texture_density = max(params.d.z, 0.1);
    let top_hue = fract(params.d.w);

    let bottom_hue = fract(params.e.x);
    let top_sat = clamp(params.e.y, 0.0, 1.0);
    let top_val = clamp(params.e.z, 0.0, 1.0);
    let bottom_sat = clamp(params.e.w, 0.0, 1.0);

    let bottom_val = clamp(params.f.x, 0.0, 1.0);
    let paper_tint = clamp(params.f.y, 0.0, 1.0);
    let grain_strength = max(params.f.z, 0.0);

    let top_color = hsv_to_rgb(vec3f(top_hue, top_sat, top_val));
    let bottom_color = hsv_to_rgb(
        vec3f(bottom_hue, bottom_sat, bottom_val),
    );

    let warm_paper = vec3f(0.985, 0.973, 0.952);
    let cool_paper = vec3f(0.95, 0.957, 0.985);
    let paper_color = mix(warm_paper, cool_paper, paper_tint);

    let aspect = w / max(h, 0.0001);
    var uv = position / zoom;
    uv.x *= aspect;
    let base_rot = time * rotation_rate * TAU * 0.25;
    let p = rotate_2d(uv, base_rot);
    let radius = length(p);
    let theta = atan2(p.y, p.x);
    let radial_dir = p / max(radius, 0.0001);
    let poly_r = polygon_radius(theta, sides);

    let paper_uv = position * 0.5 + vec2f(0.5);
    var out_color = paper_color;

    for (var i = 0; i < MAX_LAYERS; i++) {
        if i >= layer_count {
            break;
        }

        let layer_f = f32(i);
        let layer_seed = layer_f * 17.37 + time * 0.33;
        let layer_norm = layer_f / max(f32(layer_count - 1), 1.0);
        let decay = mix(1.0, LAYER_DECAY_END, layer_norm);
        let spread = layer_norm * variance_strength;
        let layer_jitter = hash22(vec2f(layer_f + 1.7, layer_f + 7.1))
            - vec2f(0.5);
        let time_drift = vec2f(
            sin(time * 0.23 + layer_seed * 0.11),
            cos(time * 0.19 + layer_seed * 0.13),
        ) * BLEED_TIME_DRIFT * spread;
        let drift = layer_jitter * BLEED_DRIFT * spread + time_drift;
        let layer_radius = radius + dot(radial_dir, drift);
        let layer_theta = theta + (drift.x - drift.y) * BLEED_THETA_SHIFT;

        let base_noise = fbm1(
            layer_theta * BASE_FREQ + layer_seed * 0.21 + time * 0.2,
            BASE_OCTAVES,
        ) * 2.0 - 1.0;
        let detail_noise = fbm1(
            layer_theta * DETAIL_FREQ + layer_seed * 1.91,
            DETAIL_OCTAVES,
        ) * 2.0 - 1.0;

        let variance_field = fbm1(
            layer_theta * VARIANCE_SCALE + layer_seed * VARIANCE_ANIM + 1.7,
            VARIANCE_OCTAVES,
        );
        let variance_mixed = mix(
            1.0,
            mix(0.35, 1.25, smoothstep(0.15, 0.85, variance_field)),
            variance_strength,
        );

        let warped_radius = base_radius * poly_r
            + base_noise * base_deform * decay
            + detail_noise * detail_deform * variance_mixed * decay
            + spread * BLEED_EXPAND;
        let dist = layer_radius - warped_radius;
        let shape_alpha = 1.0
            - smoothstep(-EDGE_SOFTNESS, EDGE_SOFTNESS, dist);
        let outside = step(0.0, dist);
        let halo_alpha = outside
            * (1.0 - smoothstep(0.0, HALO_WIDTH, dist))
            * HALO_STRENGTH
            * spread;
        let base_alpha = max(shape_alpha, halo_alpha);
        if base_alpha <= 0.0001 {
            continue;
        }

        let mask_uv = paper_uv + vec2f(
            hash11(layer_f * 1.21),
            hash11(layer_f * 2.73),
        ) + time * MASK_SCROLL;
        let texture_mask = circle_mask_fast(
            mask_uv,
            texture_density,
            layer_seed,
        );
        let texture_mask_coarse = circle_mask_fast(
            mask_uv + vec2f(0.23, -0.19),
            texture_density * COARSE_MASK_DENSITY_RATIO,
            layer_seed * 1.91 + 3.7,
        );
        let cloud_noise = value_noise_2d(
            mask_uv * texture_density * 0.31
                + vec2f(layer_seed * 0.09, time * 0.05),
        );
        let cloud_mask = smoothstep(0.28, 0.88, cloud_noise);
        let multi_mask = clamp(
            texture_mask
                + texture_mask_coarse * COARSE_MASK_WEIGHT
                + cloud_mask * CLOUD_MASK_WEIGHT,
            0.0,
            1.0,
        );

        let alpha = clamp(
            base_alpha
                * mix(1.0, multi_mask, texture_strength)
                * layer_alpha,
            0.0,
            1.0,
        );

        let gradient_color = mix(bottom_color, top_color, layer_norm);
        let block = i / ALT_BLOCK_SIZE;
        let alt_color = select(
            bottom_color,
            top_color,
            (block % 2) == 0,
        );
        let layer_color = mix(gradient_color, alt_color, ALT_COLOR_MIX);
        out_color = mix(out_color, layer_color, alpha);
    }

    let grain_uv = paper_uv * GRAIN_SCALE + vec2f(time * 0.7, -time * 0.4);
    let grain = (value_noise_2d(grain_uv) - 0.5) * grain_strength;
    let final_color = clamp(
        out_color + vec3f(grain),
        vec3f(0.0),
        vec3f(1.0),
    );
    return vec4f(final_color, 1.0);
}

fn circle_mask_fast(
    uv: vec2f,
    density: f32,
    seed: f32,
) -> f32 {
    let scaled = uv * density;
    let cell = floor(scaled);
    let local = fract(scaled) - vec2f(0.5);
    let jitter = hash22(cell + vec2f(seed, seed * 1.23)) - vec2f(0.5);
    let d = length(local - jitter);
    let radius = MASK_RADIUS * (0.7 + 0.6 * (jitter.x + 0.5));
    return 1.0 - smoothstep(radius, radius + MASK_SOFTNESS, d);
}

fn polygon_radius(theta: f32, sides: f32) -> f32 {
    let sector = TAU / sides;
    let half_sector = sector * 0.5;
    let local = abs(modulo(theta + half_sector, sector) - half_sector);
    return cos(half_sector) / max(cos(local), 0.0001);
}

fn fbm1(x: f32, octaves: i32) -> f32 {
    var value = 0.0;
    var amp = 0.5;
    var freq = 1.0;

    for (var i = 0; i < MAX_OCTAVES; i++) {
        if i >= octaves {
            break;
        }
        value += value_noise_1d(x * freq) * amp;
        freq *= 2.0;
        amp *= 0.5;
    }

    return value;
}

fn value_noise_1d(x: f32) -> f32 {
    let i = floor(x);
    let f = fract(x);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(hash11(i), hash11(i + 1.0), u);
}

fn value_noise_2d(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let a = hash21(i);
    let b = hash21(i + vec2f(1.0, 0.0));
    let c = hash21(i + vec2f(0.0, 1.0));
    let d = hash21(i + vec2f(1.0, 1.0));

    let x1 = mix(a, b, u.x);
    let x2 = mix(c, d, u.x);
    return mix(x1, x2, u.y);
}

fn hash11(n: f32) -> f32 {
    let p = fract(n * 0.1031);
    let q = p * (p + 33.33);
    return fract((q + p) * q);
}

fn hash21(p: vec2f) -> f32 {
    var p3 = fract(vec3f(p.x, p.y, p.x) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash22(p: vec2f) -> vec2f {
    let x = hash21(p + vec2f(37.0, 17.0));
    let y = hash21(p + vec2f(11.0, 53.0));
    return vec2f(x, y);
}

fn rotate_2d(p: vec2f, angle: f32) -> vec2f {
    let s = sin(angle);
    let c = cos(angle);
    return vec2f(c * p.x - s * p.y, s * p.x + c * p.y);
}

fn modulo(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}

fn hsv_to_rgb(hsv: vec3f) -> vec3f {
    let h = hsv.x;
    let s = hsv.y;
    let v = hsv.z;

    if s <= 0.0001 {
        return vec3f(v);
    }

    let hp = h * 6.0;
    let i = i32(floor(hp));
    let f = hp - f32(i);
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));

    let idx = i % 6;
    if idx == 0 {
        return vec3f(v, t, p);
    }
    if idx == 1 {
        return vec3f(q, v, p);
    }
    if idx == 2 {
        return vec3f(p, v, t);
    }
    if idx == 3 {
        return vec3f(p, q, v);
    }
    if idx == 4 {
        return vec3f(t, p, v);
    }
    return vec3f(v, p, q);
}
