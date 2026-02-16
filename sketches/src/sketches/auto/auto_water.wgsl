// Based on Tyler Hobbs' watercolor simulation article.

const TAU: f32 = 6.283185307;
const MAX_LAYERS: i32 = 96;
const MAX_OCTAVES: i32 = 8;

// Watercolor model inspired by Tyler Hobbs:
// 1) Start from a regular polygon.
// 2) Repeatedly deform edges with low/high variance noise.
// 3) Accumulate many transparent layers.
// 4) Apply a circle-texture mask per layer.
// 5) Interleave two colors in fixed layer blocks.

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
    // base_deform, detail_deform, base_octaves, detail_octaves
    c: vec4f,
    // layer_count, layer_alpha, edge_softness, interleave_every
    d: vec4f,
    // variance_low, variance_high, variance_scale, variance_anim
    e: vec4f,
    // texture_density, texture_radius, texture_strength, texture_softness
    f: vec4f,
    // hue_a, sat_a, val_a, hue_b
    g: vec4f,
    // sat_b, val_b, paper_tint, grain_strength
    h: vec4f,
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

    let base_deform = params.c.x;
    let detail_deform = params.c.y;
    let base_octaves = clamp(i32(params.c.z), 1, MAX_OCTAVES);
    let detail_octaves = clamp(i32(params.c.w), 1, MAX_OCTAVES);

    let layer_count = clamp(i32(params.d.x), 1, MAX_LAYERS);
    let layer_alpha = clamp(params.d.y, 0.0, 1.0);
    let edge_softness = max(params.d.z, 0.0005);
    let interleave_every = max(i32(params.d.w), 1);

    let variance_low = clamp(params.e.x, 0.0, 2.0);
    let variance_high = clamp(params.e.y, 0.0, 2.0);
    let variance_scale = max(params.e.z, 0.01);
    let variance_anim = max(params.e.w, 0.0);

    let texture_density = max(params.f.x, 0.1);
    let texture_radius = max(params.f.y, 0.0005);
    let texture_strength = clamp(params.f.z, 0.0, 1.0);
    let texture_softness = max(params.f.w, 0.0005);

    let color_a = hsv_to_rgb(vec3f(params.g.x, params.g.y, params.g.z));
    let color_b = hsv_to_rgb(vec3f(params.g.w, params.h.x, params.h.y));

    let paper_tint = clamp(params.h.z, 0.0, 1.0);
    let grain_strength = max(params.h.w, 0.0);

    let base_paper = vec3f(0.985, 0.973, 0.952);
    let cool_paper = vec3f(0.95, 0.957, 0.985);
    let paper_color = mix(base_paper, cool_paper, paper_tint);

    let aspect = w / max(h, 0.0001);
    var uv = position / zoom;
    uv.x *= aspect;
    let base_rot = time * rotation_rate * TAU * 0.25;
    let base_pos = rotate_2d(uv, base_rot);

    let paper_uv = position * 0.5 + vec2f(0.5);
    var out_color = paper_color;

    for (var i = 0; i < MAX_LAYERS; i++) {
        if i >= layer_count {
            break;
        }

        let layer_f = f32(i);
        let layer_seed = layer_f * 19.13 + time * 0.37;
        let jitter = vec2f(
            hash11(layer_f + 3.1) - 0.5,
            hash11(layer_f + 7.9) - 0.5,
        );
        let drift = jitter * 0.18;
        let swirl = (hash11(layer_f + 12.4) - 0.5) * 0.8;
        let p = rotate_2d(base_pos + drift, swirl);

        let radius = length(p);
        let theta = atan2(p.y, p.x);
        let poly_r = polygon_radius(theta, sides);

        let base_noise = fbm1(
            theta * 2.0 + time * 0.2 + 0.7,
            base_octaves,
        ) * 2.0 - 1.0;
        let detail_noise = fbm1(
            theta * 11.0 + layer_seed * 1.7,
            detail_octaves,
        ) * 2.0 - 1.0;
        let variance_field = fbm1(
            theta * variance_scale + layer_seed * variance_anim + 2.3,
            4,
        );
        let variance_mix = smoothstep(0.15, 0.85, variance_field);
        let variance = mix(variance_low, variance_high, variance_mix);

        let layer_norm = layer_f / max(f32(layer_count - 1), 1.0);
        let decay = mix(1.0, 0.7, layer_norm);

        let warped_radius = base_radius * poly_r
            + base_noise * base_deform * decay
            + detail_noise * detail_deform * variance * decay;
        let dist = radius - warped_radius;
        let shape_alpha = 1.0
            - smoothstep(-edge_softness, edge_softness, dist);

        let mask_uv = paper_uv + vec2f(
            hash11(layer_f * 1.21),
            hash11(layer_f * 2.73),
        ) + vec2f(time * 0.01, -time * 0.008);
        let texture_mask = circle_mask(
            mask_uv,
            texture_density,
            layer_seed,
            texture_radius,
            texture_softness,
        );
        let masked_alpha = shape_alpha
            * mix(1.0, texture_mask, texture_strength);
        let alpha = clamp(masked_alpha * layer_alpha, 0.0, 1.0);

        let block = i / interleave_every;
        let layer_color = select(color_b, color_a, (block % 2) == 0);
        out_color = mix(out_color, layer_color, alpha);
    }

    let grain_uv = paper_uv * vec2f(480.0, 380.0)
        + vec2f(time * 0.7, -time * 0.4);
    let grain = (value_noise_2d(grain_uv) - 0.5) * grain_strength;
    let final_color = clamp(
        out_color + vec3f(grain),
        vec3f(0.0),
        vec3f(1.0),
    );
    return vec4f(final_color, 1.0);
}

fn circle_mask(
    uv: vec2f,
    density: f32,
    seed: f32,
    radius: f32,
    softness: f32,
) -> f32 {
    let scaled = uv * density;
    let cell = floor(scaled);
    let local = fract(scaled) - vec2f(0.5);
    var mask = 0.0;

    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let offset = vec2f(f32(x), f32(y));
            let id = cell + offset;
            let jitter = hash22(id + vec2f(seed, seed * 1.31));
            let center = offset + jitter - vec2f(0.5);
            let d = length(local - center);
            let r = radius * (0.45 + jitter.x * 1.1);
            let stamp = 1.0 - smoothstep(r, r + softness, d);
            mask = max(mask, stamp);
        }
    }

    return mask;
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
    return fract(sin(n * 127.1) * 43758.5453123);
}

fn hash21(p: vec2f) -> f32 {
    return fract(sin(dot(p, vec2f(127.1, 311.7))) * 43758.5453123);
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
