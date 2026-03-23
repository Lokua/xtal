struct VertexInput {
    @builtin(vertex_index) index: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
};

struct Params {
    // w, h, beats, n_lines
    a: vec4f,
    // segments, points_per_segment, passes, point_size
    b: vec4f,
    // noise_scale, angle_variation, line_span, line_jitter
    c: vec4f,
    // anim_speed, wave_amp, wave_freq, alpha
    d: vec4f,
    // blue_r, blue_g, blue_b, line_width
    e: vec4f,
    // noise_map_mode, noise_map_mix, unused, unused
    f: vec4f,
    // stripe_freq, stripe_sharpness, interference_amp, interference_freq
    g: vec4f,
    // color_variation, grain, color_freq, alpha_variation
    h: vec4f,
    // length_pattern_mode, length_pattern_amount, length_pattern_freq, length_pattern_rate
    i: vec4f,
    // freq_dist_mode, freq_dist_amount, freq_dist_span, freq_dist_rate
    j: vec4f,
}

const TAU: f32 = 6.28318530718;
const U32_MAX_F: f32 = 4294967295.0;
@group(0) @binding(0)
var<uniform> params: Params;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    if (vert.index < 3u) {
        var bg_pos = vec2f(-1.0, 1.0);
        if (vert.index == 0u) {
            bg_pos = vec2f(-1.0, -3.0);
        } else if (vert.index == 1u) {
            bg_pos = vec2f(3.0, 1.0);
        }

        var out: VertexOutput;
        out.position = vec4f(bg_pos, 0.0, 1.0);
        out.color = vec4f(0.0);
        return out;
    }

    let idx = vert.index - 3u;
    let point_idx = idx / 6u;
    let corner_idx = idx % 6u;

    let n_lines = max(1u, u32(params.a.w));
    let segments = max(1u, u32(params.b.x));
    let points_per_segment = max(1u, u32(params.b.y));
    let passes = max(1u, u32(params.b.z));

    let points_per_line = segments * points_per_segment;
    let points_per_pass = n_lines * points_per_line;

    let pass_idx = point_idx / points_per_pass;
    let in_pass_idx = point_idx % points_per_pass;
    let line_idx = in_pass_idx / points_per_line;
    let line_point_idx = in_pass_idx % points_per_line;

    let seg_idx = line_point_idx / points_per_segment;
    let point_in_seg = line_point_idx % points_per_segment;

    let beats = params.a.z;
    let line_span = params.c.z;
    let line_jitter = params.c.w;
    let noise_scale = params.c.x;
    let angle_variation = params.c.y;
    let anim_speed = params.d.x;
    let wave_amp = params.d.y;
    let wave_freq = params.d.z;
    let line_width = clamp(params.e.w, 0.05, 1.0);
    let stripe_freq = params.g.x;
    let stripe_sharpness = params.g.y;
    let interference_amp = params.g.z;
    let interference_freq = params.g.w;
    let color_variation = params.h.x;
    let grain = params.h.y;
    let color_freq = params.h.z;
    let alpha_variation = params.h.w;
    let length_pattern_mode = params.i.x;
    let length_pattern_amount = clamp(params.i.y, 0.0, 1.0);
    let length_pattern_freq = max(params.i.z, 0.0);
    let length_pattern_rate = params.i.w;
    let freq_dist_mode = params.j.x;
    let freq_dist_amount = max(params.j.y, 0.0);
    let freq_dist_span = max(params.j.z, 0.0);
    let freq_dist_rate = params.j.w;

    var line_t = 0.5;
    if (n_lines > 1u) {
        line_t = f32(line_idx) / f32(n_lines - 1u);
    }
    let noise_map_mode = params.f.x;
    let noise_map_mix = params.f.y;
    var noise_map = 1.0;
    if (noise_map_mode >= 0.5 && noise_map_mode < 1.5) {
        noise_map = line_t;
    } else if (noise_map_mode >= 1.5 && noise_map_mode < 2.5) {
        noise_map = 1.0 - line_t;
    } else if (noise_map_mode >= 2.5 && noise_map_mode < 3.5) {
        noise_map = 1.0 - abs(line_t * 2.0 - 1.0);
    } else if (noise_map_mode >= 3.5) {
        noise_map = abs(line_t * 2.0 - 1.0);
    }
    let mapped_noise_scale =
        noise_scale * mix(1.0, noise_map, clamp(noise_map_mix, 0.0, 1.0));
    let y_center = mix(-line_span, line_span, line_t);
    let musical_beats = beats * 0.25;
    let time = musical_beats * anim_speed;

    let length_time = musical_beats * length_pattern_rate;
    let length_phase = line_t * length_pattern_freq + length_time;
    var length_profile = 1.0;
    if (length_pattern_mode >= 0.5 && length_pattern_mode < 1.5) {
        length_profile = 0.5 + 0.5 * sin(length_phase * TAU);
    } else if (length_pattern_mode >= 1.5 && length_pattern_mode < 2.5) {
        length_profile = triangle01(fract(length_phase));
    } else if (length_pattern_mode >= 2.5 && length_pattern_mode < 3.5) {
        let noise_seed = line_idx * 3571u + u32(floor(abs(length_time) * 97.0));
        length_profile = unit(noise_seed);
    } else if (length_pattern_mode >= 3.5) {
        let stripe = abs(sin(length_phase * TAU));
        length_profile = 1.0 - stripe;
    }
    let length_mapped = mix(1.0, length_profile, length_pattern_amount);
    let local_line_width = line_width * max(0.05, length_mapped);

    let x0 = -local_line_width;
    let x1 = local_line_width;
    let seg_t0 = f32(seg_idx) / f32(segments);
    let seg_t1 = f32(seg_idx + 1u) / f32(segments);

    let seed_base = line_idx * 8191u + pass_idx * 131071u;
    let jitter0 = signed_unit(seed_base + seg_idx * 97u) * line_jitter;
    let jitter1 = signed_unit(seed_base + (seg_idx + 1u) * 97u) * line_jitter;

    let freq_time = musical_beats * freq_dist_rate;
    let freq_phase = line_t + freq_time;
    var freq_profile = 0.5;
    if (freq_dist_mode >= 0.5 && freq_dist_mode < 1.5) {
        freq_profile = clamp(freq_phase, 0.0, 1.0);
    } else if (freq_dist_mode >= 1.5 && freq_dist_mode < 2.5) {
        freq_profile = 1.0 - clamp(freq_phase, 0.0, 1.0);
    } else if (freq_dist_mode >= 2.5 && freq_dist_mode < 3.5) {
        freq_profile = triangle01(fract(freq_phase));
    } else if (freq_dist_mode >= 3.5) {
        let noise_seed = line_idx * 4219u + u32(floor(abs(freq_time) * 113.0));
        freq_profile = unit(noise_seed);
    }
    let freq_offset = (freq_profile * 2.0 - 1.0) * freq_dist_span * freq_dist_amount;
    let wave_freq_line = max(0.0, wave_freq + freq_offset);

    let px0 = mix(x0, x1, seg_t0);
    let px1 = mix(x0, x1, seg_t1);
    let wave0 = sin(px0 * wave_freq_line + time) * wave_amp;
    let wave1 = sin(px1 * wave_freq_line + time) * wave_amp;

    let p0 = vec2f(px0, y_center + jitter0 + wave0);
    let p1 = vec2f(px1, y_center + jitter1 + wave1);

    let tangent = normalize(p1 - p0 + vec2f(1e-6, 0.0));
    let normal = vec2f(-tangent.y, tangent.x);

    let t_seed = point_idx * 31u + pass_idx * 997u;
    let t_rand = unit(t_seed);
    let t = (f32(point_in_seg) + t_rand) / f32(points_per_segment);
    let base = mix(p0, p1, t);

    let offset_seed = point_idx * 59u + pass_idx * 281u;
    let offset_mag = signed_unit(offset_seed) * mapped_noise_scale;
    let angle = signed_unit(offset_seed + 1u) * angle_variation;
    let dir = rotate(normal, angle);

    let stripe_phase = (base.x * 0.5 + 0.5) * TAU * stripe_freq;
    let stripe_wave = abs(sin(stripe_phase));
    let stripe_gate = pow(max(0.0, 1.0 - stripe_wave), stripe_sharpness);
    let inter = sin(base.x * interference_freq + time)
        * sin((line_t * 2.0 - 1.0) * interference_freq * 1.37 - time * 0.73);
    let pattern_offset = normal * inter * interference_amp * stripe_gate;

    let pos = base + dir * offset_mag + pattern_offset;

    let point_size_px = params.b.w;
    let point_size_ndc = vec2f(
        (2.0 * point_size_px) / max(params.a.x, 1.0),
        (2.0 * point_size_px) / max(params.a.y, 1.0),
    );
    let final_pos = pos + corner_offset(corner_idx, point_size_ndc);

    let tone_wave = sin((line_t * 2.0 - 1.0) * color_freq + time * 0.21);
    let grain_noise = signed_unit(offset_seed + 12345u);
    let tone = 1.0 + color_variation * (0.6 * inter + 0.4 * tone_wave);
    let grain_mix = 1.0 + grain * grain_noise;
    let tint_shift = vec3f(0.0, 0.08, 0.16) * grain * grain_noise;
    let color = clamp(params.e.xyz * tone * grain_mix + tint_shift, vec3f(0.0), vec3f(1.0));
    let alpha = clamp(
        params.d.w * (1.0 + alpha_variation * grain_noise),
        0.0,
        1.0,
    );

    var out: VertexOutput;
    out.position = vec4f(final_pos, 0.0, 1.0);
    out.color = vec4f(color, alpha);
    return out;
}

@fragment
fn fs_main(@location(0) color: vec4f) -> @location(0) vec4f {
    return color;
}

fn corner_offset(index: u32, point_size: vec2f) -> vec2f {
    let sx = point_size.x;
    let sy = point_size.y;
    if (index == 0u) {
        return vec2f(-sx, -sy);
    }
    if (index == 1u) {
        return vec2f(sx, -sy);
    }
    if (index == 2u) {
        return vec2f(-sx, sy);
    }
    if (index == 3u) {
        return vec2f(sx, -sy);
    }
    if (index == 4u) {
        return vec2f(sx, sy);
    }
    return vec2f(-sx, sy);
}

fn rotate(v: vec2f, angle: f32) -> vec2f {
    let c = cos(angle);
    let s = sin(angle);
    return vec2f(v.x * c - v.y * s, v.x * s + v.y * c);
}

fn hash_u32(value: u32) -> u32 {
    var x = value;
    x = x ^ (x >> 16u);
    x = x * 0x7feb352du;
    x = x ^ (x >> 15u);
    x = x * 0x846ca68bu;
    x = x ^ (x >> 16u);
    return x;
}

fn unit(seed: u32) -> f32 {
    return f32(hash_u32(seed)) / U32_MAX_F;
}

fn signed_unit(seed: u32) -> f32 {
    return unit(seed) * 2.0 - 1.0;
}

fn triangle01(x: f32) -> f32 {
    return 1.0 - abs(x * 2.0 - 1.0);
}
