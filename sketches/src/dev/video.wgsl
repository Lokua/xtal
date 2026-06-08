struct Params {
    a: vec4f,
    b: vec4f,
    c: vec4f,
    d: vec4f,
    e: vec4f,
    f: vec4f,
    g: vec4f,
    h: vec4f,
    i: vec4f,
    j: vec4f,
    k: vec4f,
    l: vec4f,
    m: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@group(1) @binding(0)
var video_sampler: sampler;

@group(1) @binding(1)
var video_tex: texture_2d<f32>;

struct VsOut {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

struct VertexInput {
    @location(0) position: vec2f,
}

@vertex
fn vs_main(vert: VertexInput) -> VsOut {
    let p = vert.position;
    var out: VsOut;
    out.position = vec4f(p, 0.0, 1.0);
    out.uv = p * 0.5 + vec2f(0.5, 0.5);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4f {
    let uv = vec2f(in.uv.x, 1.0 - in.uv.y);
    let fit_mode = i32(params.a.w + 0.5);
    let screen_size = vec2f(max(params.a.x, 1.0), max(params.a.y, 1.0));
    let video_size = vec2f(textureDimensions(video_tex));
    let fit_uv = fit_video_uv(uv, screen_size, video_size, fit_mode);

    if (is_outside(fit_uv)) {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }

    let slice_uv = apply_slice_shift(fit_uv);
    let grid_uv = apply_grid_shuffle(slice_uv);
    let zoom_uv = apply_zoom(grid_uv);
    let displacement = displacement_vector(zoom_uv);
    let sample_uv = clamp(zoom_uv + displacement, vec2f(0.0), vec2f(1.0));
    let color = textureSample(video_tex, video_sampler, sample_uv).rgb;
    var result: vec3f;
    if (params.d.w > 0.5) {
        let edged = apply_edge_detect(sample_uv, color);
        result = apply_pixel_sort(sample_uv, edged);
    } else {
        let sorted = apply_pixel_sort(sample_uv, color);
        result = apply_edge_detect(sample_uv, sorted);
    }
    return vec4f(apply_video_fx(result), 1.0);
}

fn fit_video_uv(
    uv: vec2f,
    screen_size: vec2f,
    video_size: vec2f,
    fit_mode: i32,
) -> vec2f {
    if (fit_mode == 2) {
        return uv;
    }

    let screen = max(screen_size, vec2f(1.0, 1.0));
    let video = max(video_size, vec2f(1.0, 1.0));
    let screen_aspect = screen.x / screen.y;
    let video_aspect = video.x / video.y;
    var scale = vec2f(1.0, 1.0);

    if (fit_mode == 1) {
        if (screen_aspect > video_aspect) {
            scale = vec2f(video_aspect / screen_aspect, 1.0);
        } else {
            scale = vec2f(1.0, screen_aspect / video_aspect);
        }
    } else {
        if (screen_aspect > video_aspect) {
            scale = vec2f(1.0, screen_aspect / video_aspect);
        } else {
            scale = vec2f(video_aspect / screen_aspect, 1.0);
        }
    }

    let pan = params.c.yz;
    let pan_range = abs(1.0 - (1.0 / max(scale, vec2f(0.0001)))) * 0.5;
    return (uv - vec2f(0.5, 0.5)) / scale
        + vec2f(0.5, 0.5)
        + pan * pan_range;
}

fn is_outside(uv: vec2f) -> bool {
    return uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0;
}

fn apply_zoom(uv: vec2f) -> vec2f {
    let amount = params.c.x;
    var offset = params.c.yz;

    if (params.c.w > 0.5) {
        offset = params.d.xy;
    }

    let scale = mix(1.0, 4.0, amount);
    let center = vec2f(0.5, 0.5) + offset * 0.5;
    return (uv - center) / scale + center;
}

fn apply_slice_shift(uv: vec2f) -> vec2f {
    let amount = params.e.x;
    let slices = max(floor(params.e.y), 1.0);
    let offset = params.e.z;
    let use_vertical = step(0.5, params.e.w);
    let boundary = i32(params.f.x + 0.5);
    let coord = mix(uv.y, uv.x, use_vertical);
    let slice_index = floor(coord * slices);
    let alternating = select(-1.0, 1.0, (i32(slice_index) & 1) == 0);
    let shift = alternating * offset * amount;
    let shifted = uv
        + mix(vec2f(shift, 0.0), vec2f(0.0, shift), use_vertical);
    return apply_slice_boundary(shifted, boundary);
}

fn rand(co: vec2f) -> f32 {
    return fract(sin(dot(co, vec2f(12.9898, 78.233))) * 43758.5453);
}

fn apply_grid_shuffle(uv: vec2f) -> vec2f {
    let amount = params.j.x;
    if (amount <= 0.0001) {
        return uv;
    }

    let n = max(floor(params.j.w + 0.5), 2.0);
    let seed = floor(params.d.z + 0.5);
    let tile = floor(uv * n);
    let local_uv = fract(uv * n);
    let seeded = tile + vec2f(seed * 127.1, seed * 311.7);

    // Tiles whose hash exceeds amount stay in place
    if (rand(seeded) > amount) {
        return uv;
    }

    let tx = rand(seeded + vec2f(17.3, 41.7));
    let ty = rand(seeded + vec2f(83.1, 29.5));
    let target_tile = floor(vec2f(tx, ty) * n);

    return (target_tile + local_uv) / n;
}

fn apply_slice_boundary(uv: vec2f, boundary: i32) -> vec2f {
    if (boundary == 1) {
        return fract(uv);
    }
    if (boundary == 2) {
        return 1.0 - abs(fract(uv * 0.5) * 2.0 - 1.0);
    }
    return clamp(uv, vec2f(0.0), vec2f(1.0));
}

fn displacement_vector(uv: vec2f) -> vec2f {
    let amount = params.h.y;
    if (amount <= 0.0001) {
        return vec2f(0.0);
    }

    let frequency = max(params.h.z, 0.001);
    let beat_period = max(floor(params.h.w + 0.5), 1.0);
    let phase = params.a.z / beat_period;
    let angle = params.f.y * 6.28318530718;
    let direction = vec2f(cos(angle), sin(angle));
    let cross_direction = vec2f(-direction.y, direction.x);
    let centered = uv - vec2f(0.5);
    let primary = dot(centered, direction) * frequency + phase;
    let secondary = dot(centered, cross_direction) * frequency * 0.63;
    let warp = params.f.z;
    let wave = sin(primary * 6.28318530718);
    let folded = sin((primary + wave * warp + secondary) * 6.28318530718);
    return cross_direction * folded * amount;
}

fn luma(color: vec3f) -> f32 {
    return dot(color, vec3f(0.2126, 0.7152, 0.0722));
}

fn sort_value(color: vec3f) -> f32 {
    let channel = i32(params.k.w + 0.5);
    if (channel == 1) {
        return max(color.r, max(color.g, color.b)) - min(color.r, min(color.g, color.b));
    }
    if (channel == 2) { return color.r; }
    if (channel == 3) { return color.g; }
    if (channel == 4) { return color.b; }
    return luma(color);
}

fn apply_pixel_sort(uv: vec2f, color: vec3f) -> vec3f {
    let amount = params.k.x;
    if (abs(amount) <= 0.0001) {
        return color;
    }
    let threshold = params.k.y;
    let val = sort_value(color);
    if (val <= threshold) {
        return color;
    }
    let use_vertical = step(0.5, params.k.z);
    let t = (val - threshold) / max(1.0 - threshold, 0.001);
    let offset = t * amount;
    let displaced = uv + mix(vec2f(offset, 0.0), vec2f(0.0, offset), use_vertical);
    return textureSampleLevel(
        video_tex, video_sampler, clamp(displaced, vec2f(0.0), vec2f(1.0)), 0.0
    ).rgb;
}

fn apply_edge_detect(uv: vec2f, color: vec3f) -> vec3f {
    let amount = params.l.x;
    if (amount <= 0.0001) {
        return color;
    }
    let boost = max(params.l.y, 1.0);
    let radius = max(params.l.w, 1.0);
    let px = radius / vec2f(max(params.a.x, 1.0), max(params.a.y, 1.0));
    let edge_r = luma(textureSampleLevel(video_tex, video_sampler, clamp(uv + vec2f( px.x,  0.0), vec2f(0.0), vec2f(1.0)), 0.0).rgb);
    let edge_l = luma(textureSampleLevel(video_tex, video_sampler, clamp(uv + vec2f(-px.x,  0.0), vec2f(0.0), vec2f(1.0)), 0.0).rgb);
    let edge_t = luma(textureSampleLevel(video_tex, video_sampler, clamp(uv + vec2f( 0.0,  px.y), vec2f(0.0), vec2f(1.0)), 0.0).rgb);
    let edge_b = luma(textureSampleLevel(video_tex, video_sampler, clamp(uv + vec2f( 0.0, -px.y), vec2f(0.0), vec2f(1.0)), 0.0).rgb);
    let edge = clamp(length(vec2f(edge_r - edge_l, edge_t - edge_b)) * boost, 0.0, 1.0);
    let edge_color = hsv_to_rgb(vec3f(params.m.x, params.m.y, 1.0)) * edge;
    let additive = step(0.5, params.f.w);
    let mixed = mix(color, edge_color, amount);
    let added = clamp(color + edge_color * amount, vec3f(0.0), vec3f(1.0));
    return mix(mixed, added, additive);
}

fn apply_video_fx(color: vec3f) -> vec3f {
    let black_white = params.b.x;
    let invert = params.b.y;
    let posterize = params.b.z;
    let posterize_levels = params.b.w;
    let luma = dot(color, vec3f(0.2126, 0.7152, 0.0722));
    var out_color = mix(color, vec3f(luma), black_white);
    out_color = mix(out_color, 1.0 - out_color, invert);
    out_color = apply_invert_darken(out_color);
    let final_luma = dot(out_color, vec3f(0.2126, 0.7152, 0.0722));
    out_color = apply_highlight_color(out_color, final_luma, black_white);
    out_color = apply_chroma_shift(out_color);
    out_color = mix(
        out_color,
        posterize_color(out_color, posterize_levels),
        posterize,
    );
    return out_color;
}

fn apply_invert_darken(color: vec3f) -> vec3f {
    let amount = params.j.y;
    if (amount <= 0.0001) {
        return color;
    }

    let luma = dot(color, vec3f(0.2126, 0.7152, 0.0722));
    let white_mask = smoothstep(0.62, 1.0, luma);
    let chroma = length(color - vec3f(luma));
    let neutral_mask = 1.0 - smoothstep(0.04, 0.28, chroma);
    let mask = white_mask * neutral_mask * amount;
    return mix(color, color * (1.0 - white_mask), mask);
}

fn apply_highlight_color(
    color: vec3f,
    luma: f32,
    black_white: f32,
) -> vec3f {
    let mix_amount = params.g.x * black_white;
    let hue = fract(params.g.y);
    let sat = params.g.z;
    let threshold = params.g.w;
    let softness = max(params.h.x, 0.001);
    let mask = smoothstep(threshold, threshold + softness, luma);
    let tint = hsv_to_rgb(vec3f(hue, sat, 1.0));
    return mix(color, tint * max(luma, 0.001), mask * mix_amount);
}

fn apply_chroma_shift(color: vec3f) -> vec3f {
    let amount = params.i.z;
    if (amount <= 0.0001) {
        return color;
    }

    let luma_weights = vec3f(0.2126, 0.7152, 0.0722);
    let luma = dot(color, luma_weights);
    let chroma = color - vec3f(luma);
    let target_rgb = hsv_to_rgb(vec3f(fract(params.i.y), 1.0, 1.0));
    let target_luma = dot(target_rgb, luma_weights);
    let target_chroma = target_rgb - vec3f(target_luma);
    let chroma_len = length(chroma);
    let target_len = length(target_chroma);
    let chroma_dir = chroma / max(chroma_len, 0.0001);
    let target_dir = target_chroma / max(target_len, 0.0001);
    let softness = params.i.w;
    let directional_mask = smoothstep(
        0.95 - softness * 1.95,
        1.0,
        dot(chroma_dir, target_dir),
    );
    let chroma_mask = smoothstep(0.005, 0.12, chroma_len);
    let mask = chroma_mask * mix(directional_mask, 1.0, softness);
    let shift = params.i.x * 6.28318530718;
    let gain = 1.0 + amount * 0.75;
    let shifted = rotate_rgb_chroma(chroma, shift) * gain;
    var shifted_rgb = vec3f(luma) + shifted;
    let shifted_luma = dot(shifted_rgb, luma_weights);
    shifted_rgb += luma - shifted_luma;
    let safe_rgb = clamp(shifted_rgb, vec3f(0.0), vec3f(1.0));
    let mix_amount = clamp(amount, 0.0, 1.0) * mask;
    return mix(color, safe_rgb, mix_amount);
}

fn rotate_rgb_chroma(chroma: vec3f, angle: f32) -> vec3f {
    let axis = normalize(vec3f(1.0));
    let s = sin(angle);
    let c = cos(angle);
    return chroma * c
        + cross(axis, chroma) * s
        + axis * dot(axis, chroma) * (1.0 - c);
}

fn hsv_to_rgb(hsv: vec3f) -> vec3f {
    let k = vec3f(0.0, 2.0 / 3.0, 1.0 / 3.0);
    let p = abs(fract(hsv.xxx + k) * 6.0 - 3.0);
    let rgb = clamp(p - 1.0, vec3f(0.0), vec3f(1.0));
    return hsv.z * mix(vec3f(1.0), rgb, hsv.y);
}

fn posterize_color(color: vec3f, levels: f32) -> vec3f {
    return floor(color * levels) / max(levels - 1.0, 1.0);
}
