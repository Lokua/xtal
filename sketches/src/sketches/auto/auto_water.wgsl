// Based on Tyler Hobbs' watercolor simulation article.

const TAU: f32 = 6.283185307;
const MAX_LAYERS: i32 = 56;
const MAX_RENDER_LAYERS: i32 = 8;
const MAX_DROPS: i32 = 8;
const MAX_OCTAVES: i32 = 4;
const BLOT_COUNT: i32 = 3;

// Performance + look constants.
const BASE_OCTAVES: i32 = 2;
const DETAIL_OCTAVES: i32 = 1;
const FIELD_BASE_SCALE: f32 = 1.9;
const FIELD_DETAIL_SCALE: f32 = 5.8;
const WARP_SCALE: f32 = 1.35;
const WARP_STRENGTH: f32 = 0.16;
const EDGE_SOFTNESS: f32 = 0.014;
const LAYER_DECAY_END: f32 = 0.72;
const ALT_BLOCK_SIZE: i32 = 4;
const ALT_COLOR_MIX: f32 = 0.26;
const TEXTURE_DENSITY: f32 = 10.0;
const GRAIN_SCALE: vec2f = vec2f(430.0, 330.0);
const HALO_WIDTH: f32 = 0.045;
const HALO_STRENGTH_MAX: f32 = 1.0;
const LAYER_SHIFT_STRENGTH: f32 = 0.95;
const SPLASH_STRETCH_X: f32 = 0.2;
const SPLASH_STRETCH_Y: f32 = 1.85;
const SPLASH_NOISE_SCALE: f32 = 4.6;
const SPLASH_GATE: f32 = 0.72;
const SPLASH_STRENGTH: f32 = 0.85;
const EDGE_BREAKUP_SCALE: f32 = 12.0;
const EDGE_BREAKUP_STRENGTH: f32 = 0.09;
const MIN_VISIBLE_ALPHA: f32 = 0.004;
const PAPER_TINT: f32 = 0.0;
const GRAIN_STRENGTH: f32 = 0.06;

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
    // shape_anomaly, spread_anomaly, base_radius, zoom
    b: vec4f,
    // base_deform, detail_deform, layer_count, layer_alpha
    c: vec4f,
    // variance_strength, texture_gate_mix, halo_strength, hue
    d: vec4f,
    // hue_spread, blot_count, blot_rate, blot_fade
    e: vec4f,
    // blot_spread, envelope_size, envelope_softness, envelope_noise
    f: vec4f,
    // blot_darken, blot_contrast, pool_strength, granulation_strength
    g: vec4f,
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

    let shape_anomaly = params.b.x;
    let spread_anomaly = params.b.y;
    let base_radius = max(params.b.z, 0.02);
    let zoom = max(params.b.w, 0.05);

    let base_deform = max(params.c.x, 0.0);
    let detail_deform = max(params.c.y, 0.0);
    let layer_count = clamp(i32(params.c.z), 1, MAX_LAYERS);
    let layer_alpha = clamp(params.c.w, 0.0, 1.0);

    let variance_strength = clamp(params.d.x, 0.0, 1.0);
    let texture_gate_mix = clamp(params.d.y, 0.0, 1.0);
    let halo_strength = clamp(params.d.z, 0.0, 1.0);
    let hue = fract(params.d.w);

    let hue_spread = params.e.x;
    let blot_count = clamp(i32(params.e.y), 1, MAX_DROPS);
    let blot_rate = max(params.e.z, 0.01);
    let blot_fade = params.e.w;

    let blot_spread = params.f.x;
    let envelope_size = max(params.f.y, 0.05);
    let envelope_softness = max(params.f.z, 0.001);
    let envelope_noise_strength = params.f.w;
    let blot_darken = params.g.x;
    let blot_contrast = params.g.y;
    let pool_strength = params.g.z;
    let granulation_strength = params.g.w;

    let warm_paper = vec3f(0.985, 0.973, 0.952);
    let cool_paper = vec3f(0.95, 0.957, 0.985);
    let paper_color = mix(warm_paper, cool_paper, PAPER_TINT);

    let top_hue = hue;
    let bottom_hue = fract(hue + hue_spread);

    let aspect = w / max(h, 0.0001);
    var uv = position / zoom;
    uv.x *= aspect;

    let paper_uv = position * 0.5 + vec2f(0.5);
    var out_color = paper_color;
    let target_layers = layer_count;
    let rendered_layers = min(target_layers, MAX_RENDER_LAYERS);
    let target_span = max(f32(target_layers - 1), 1.0);
    let rendered_span = max(f32(rendered_layers - 1), 1.0);
    let layer_ratio = max(
        f32(target_layers) / max(f32(rendered_layers), 1.0),
        1.0,
    );
    let alpha_boost = pow(layer_ratio, 0.55);

    for (var drop = 0; drop < MAX_DROPS; drop++) {
        if drop >= blot_count {
            break;
        }

        let drop_f = f32(drop);
        let slot_seed = drop_f * 53.17 + 29.1;
        let slot_clock = time * blot_rate + hash11(slot_seed * 0.37) * 11.0;
        let cycle = floor(slot_clock);
        let phase = fract(slot_clock);
        let fade_in = smoothstep(0.0, 0.12, phase);
        let fade_out = 1.0 - smoothstep(blot_fade, 1.0, phase);
        let drop_life = fade_in * fade_out;
        if drop_life <= 0.0001 {
            continue;
        }

        let cycle_seed = slot_seed + cycle * 37.29;
        let center = vec2f(
            (hash11(cycle_seed * 1.11) * 2.0 - 1.0) * aspect * blot_spread,
            (hash11(cycle_seed * 1.83) * 2.0 - 1.0) * blot_spread,
        );
        let drop_radius = base_radius * mix(
            0.7,
            1.65,
            hash11(cycle_seed * 2.47),
        );
        let drop_angle = hash11(cycle_seed * 3.13) * TAU;
        let base_p = rotate_2d(uv - center, drop_angle);

        let envelope_scale = envelope_size * (1.1 + spread_anomaly * 0.35);
        let envelope_shape = 0.7 + shape_anomaly * 0.18;
        let envelope_spread = 0.55 + spread_anomaly * 0.22;
        let envelope_field = blot_field(
            base_p * 0.9 + vec2f(0.11, -0.07) * drop_radius,
            cycle_seed + 83.17,
            drop_radius * envelope_scale,
            envelope_shape,
            envelope_spread,
        );
        let envelope_noise = value_noise_2d(
            (base_p / max(drop_radius, 0.001)) * 1.7 + vec2f(4.0, -6.0),
        ) - 0.5;
        let envelope_cut = 0.22 + envelope_noise * envelope_noise_strength;
        let paint_envelope = smoothstep(
            envelope_cut,
            envelope_cut + envelope_softness,
            envelope_field,
        );
        if paint_envelope <= 0.0001 {
            continue;
        }

        let drop_hue_shift = (hash11(cycle_seed * 4.61) - 0.5) * 0.1;
        let drop_top = hsv_to_rgb(vec3f(
            fract(top_hue + drop_hue_shift),
            0.7,
            0.92,
        ));
        let drop_bottom = hsv_to_rgb(vec3f(
            fract(bottom_hue + drop_hue_shift),
            0.64,
            0.86,
        ));

        for (var i = 0; i < MAX_RENDER_LAYERS; i++) {
            if i >= rendered_layers {
                break;
            }

            let layer_t = select(
                0.0,
                f32(i) / rendered_span,
                rendered_layers > 1,
            );
            let virtual_layer_f = layer_t * target_span;
            let layer_seed = virtual_layer_f * 17.37 + cycle_seed;
            let layer_norm = virtual_layer_f / target_span;
            let decay = mix(1.0, LAYER_DECAY_END, layer_norm);
            let spread = layer_norm
                * variance_strength
                * mix(0.75, 1.55, spread_anomaly);
            let layer_jitter = hash22(vec2f(
                virtual_layer_f + 1.7,
                layer_seed,
            )) - vec2f(0.5);
            let shift = layer_jitter
                * LAYER_SHIFT_STRENGTH
                * (0.2 + spread)
                * drop_radius;
            let q0 = base_p + shift;
            let warp = vec2f(
                value_noise_2d(
                    q0 * WARP_SCALE
                        + vec2f(layer_seed * 0.19, -9.0),
                ),
                value_noise_2d(
                    q0 * WARP_SCALE
                        + vec2f(-layer_seed * 0.23, 7.0),
                ),
            ) * 2.0 - vec2f(1.0);
            let q = q0 + warp
                * WARP_STRENGTH
                * (0.35 + variance_strength * 0.6 + shape_anomaly * 0.7)
                * drop_radius;

            let core = blot_field(
                q,
                layer_seed,
                drop_radius,
                shape_anomaly,
                spread_anomaly,
            );
            let base_noise = fbm2(
                q * FIELD_BASE_SCALE / max(drop_radius, 0.001)
                    + vec2f(layer_seed * 0.11, 5.0),
                BASE_OCTAVES,
            ) - 0.5;
            let detail_noise = fbm2(
                q * FIELD_DETAIL_SCALE / max(drop_radius, 0.001)
                    + vec2f(layer_seed * 0.37, -3.0),
                DETAIL_OCTAVES,
            ) - 0.5;
            let chaos = mix(0.25, 1.0, shape_anomaly);

            let splash_angle = hash11(layer_seed * 1.31) * TAU;
            let splash_dir = rotate_2d(
                q / max(drop_radius, 0.001),
                splash_angle,
            );
            let splash_space = vec2f(
                splash_dir.x * SPLASH_STRETCH_X,
                splash_dir.y * SPLASH_STRETCH_Y,
            );
            let splash_noise = value_noise_2d(
                splash_space * SPLASH_NOISE_SCALE
                    + vec2f(layer_seed * 0.23, 2.0),
            );
            let splash_mask = smoothstep(SPLASH_GATE, 1.0, splash_noise)
                * smoothstep(0.12, 0.9, core);

            let field = core
                + base_noise * base_deform * 1.8 * decay * drop_radius
                + detail_noise * detail_deform * 1.2 * decay * drop_radius
                + splash_mask
                    * SPLASH_STRENGTH
                    * chaos
                    * (0.22 + spread)
                    * drop_radius;
            let edge_breakup = value_noise_2d(
                q * EDGE_BREAKUP_SCALE / max(drop_radius, 0.001)
                    + vec2f(layer_seed * 0.41, -6.0),
            ) - 0.5;
            let threshold = mix(0.62, 0.34, spread)
                + (hash11(layer_seed * 0.77) - 0.5) * 0.08 * chaos;
            let threshold_rough = threshold
                + edge_breakup * EDGE_BREAKUP_STRENGTH * (0.35 + shape_anomaly);
            let shape_alpha = smoothstep(
                threshold_rough - EDGE_SOFTNESS,
                threshold_rough + EDGE_SOFTNESS,
                field,
            );
            let halo_outer = smoothstep(
                threshold_rough - HALO_WIDTH,
                threshold_rough - HALO_WIDTH * 0.22,
                field,
            );
            let halo_inner = smoothstep(
                threshold_rough - HALO_WIDTH * 0.22,
                threshold_rough + HALO_WIDTH * 0.35,
                field,
            );
            let halo_band = halo_outer * (1.0 - halo_inner);
            let halo_alpha = halo_band
                * halo_strength
                * HALO_STRENGTH_MAX
                * (0.15 + spread * 0.75)
                * (1.0 - shape_alpha);
            let base_alpha = max(shape_alpha, halo_alpha);
            if base_alpha <= 0.0001 {
                continue;
            }

            let qn = q / max(drop_radius, 0.001);
            let edge_dist = abs(field - threshold_rough)
                / max(EDGE_SOFTNESS * 7.0, 0.0001);
            let edge_band = 1.0 - clamp(edge_dist, 0.0, 1.0);
            let edge_pool = smoothstep(0.18, 0.92, edge_band);

            let flow_space = rotate_2d(qn, splash_angle * 0.35);
            let flow_noise = value_noise_2d(
                flow_space * (TEXTURE_DENSITY * 0.4)
                    + vec2f(layer_seed * 0.13, cycle_seed * 0.07),
            );
            let flow_streak = smoothstep(0.36, 0.9, flow_noise);

            let grain_noise = value_noise_2d(
                qn * (TEXTURE_DENSITY * 1.05)
                    + vec2f(layer_seed * 0.31, -cycle_seed * 0.17),
            );
            let granulation = smoothstep(0.2, 0.85, grain_noise);

            let interior = smoothstep(
                threshold_rough + 0.04,
                threshold_rough + 0.32,
                field,
            );
            let fiber = value_noise_2d(
                paper_uv * vec2f(210.0, 170.0)
                    + vec2f(layer_seed * 0.09, cycle_seed * 0.03),
            );
            let fiber_break = smoothstep(0.18, 0.94, fiber);

            let texture_field = clamp(
                edge_pool * pool_strength
                    + flow_streak * (0.22 + 0.24 * interior)
                    + granulation * granulation_strength
                    + fiber_break * 0.16,
                0.0,
                1.0,
            );
            let texture_gate = smoothstep(0.2, 0.96, texture_field);

            let alpha = clamp(
                base_alpha
                    * mix(1.0, texture_gate, texture_gate_mix)
                    * paint_envelope
                    * drop_life
                    * alpha_boost
                    * layer_alpha,
                0.0,
                1.0,
            );
            if alpha <= MIN_VISIBLE_ALPHA {
                continue;
            }

            let gradient_color = mix(drop_bottom, drop_top, layer_norm);
            let block = i32(virtual_layer_f + 0.5) / ALT_BLOCK_SIZE;
            let alt_color = select(
                drop_bottom,
                drop_top,
                (block % 2) == 0,
            );
            let layer_color = mix(gradient_color, alt_color, ALT_COLOR_MIX);
            let darkened = layer_color * (1.0 - blot_darken);
            let contrasted = (darkened - vec3f(0.5)) * blot_contrast
                + vec3f(0.5);
            let tone_color = clamp(contrasted, vec3f(0.0), vec3f(1.0));
            out_color = mix(out_color, tone_color, alpha);
        }
    }

    let grain_uv = paper_uv * GRAIN_SCALE;
    let grain = (value_noise_2d(grain_uv) - 0.5) * GRAIN_STRENGTH;
    let final_color = clamp(
        out_color + vec3f(grain),
        vec3f(0.0),
        vec3f(1.0),
    );
    return vec4f(final_color, 1.0);
}

fn blot_field(
    p: vec2f,
    seed: f32,
    base_radius: f32,
    shape_anomaly: f32,
    spread_anomaly: f32,
) -> f32 {
    var field = 0.0;
    let spread = mix(0.45, 1.35, spread_anomaly);

    for (var i = 0; i < BLOT_COUNT; i++) {
        let i_f = f32(i);
        let angle = hash11(seed * 3.17 + i_f * 9.71) * TAU;
        let radial = base_radius
            * (0.12 + spread * hash11(seed * 5.31 + i_f * 4.37));
        let center = vec2f(cos(angle), sin(angle)) * radial
            + (hash22(vec2f(
                seed * 1.7 + i_f * 7.9,
                seed * 2.9 + i_f * 5.3,
            )) - vec2f(0.5)) * base_radius * shape_anomaly * 0.9;
        let size_rand = hash11(seed * 8.13 + i_f * 6.21);
        let r = base_radius
            * mix(
                0.35 + 0.25 * size_rand,
                0.95 + 0.45 * size_rand,
                shape_anomaly,
            );
        let orient = hash11(seed * 6.71 + i_f * 8.33) * TAU;
        let local = rotate_2d(p - center, orient);
        let stretch = vec2(
            1.0
                + (hash11(seed * 7.41 + i_f * 3.19) - 0.5)
                * shape_anomaly
                * 1.6,
            1.0
                + (hash11(seed * 8.77 + i_f * 5.91) - 0.5)
                * shape_anomaly
                * 1.6,
        );
        let q = local / max(stretch, vec2(0.25));
        let d = length(q);
        field += smoothstep(r * 1.5, r * 0.12, d);
    }

    return field / f32(BLOT_COUNT);
}

fn fbm2(p: vec2f, octaves: i32) -> f32 {
    var value = 0.0;
    var amp = 0.5;
    var freq = 1.0;

    for (var i = 0; i < MAX_OCTAVES; i++) {
        if i >= octaves {
            break;
        }
        value += value_noise_2d(p * freq) * amp;
        freq *= 2.0;
        amp *= 0.5;
    }

    return value;
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
