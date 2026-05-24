struct Params {
    a: vec4f,
    b: vec4f,
    c: vec4f,
    d: vec4f,
    e: vec4f,
    f: vec4f,
    g: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@group(1) @binding(0)
var tex_sampler: sampler;

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

fn sample_video(uv: vec2f) -> vec3f {
    let tex_uv = vec2f(uv.x, 1.0 - uv.y);
    return textureSample(video_tex, tex_sampler, tex_uv).rgb;
}

fn luma(color: vec3f) -> f32 {
    return dot(color, vec3f(0.299, 0.587, 0.114));
}

fn hash21(p: vec2f) -> f32 {
    let q = vec2f(
        dot(p, vec2f(127.1, 311.7)),
        dot(p, vec2f(269.5, 183.3)),
    );
    return fract(sin(q.x + q.y) * 43758.5453);
}

fn rotate2(v: vec2f, angle: f32) -> vec2f {
    let ca = cos(angle);
    let sa = sin(angle);
    return vec2f(v.x * ca - v.y * sa, v.x * sa + v.y * ca);
}

fn apply_vhs_warp(uv: vec2f, time: f32, amount: f32, speed: f32) -> vec2f {
    let line_wave = sin((uv.y * 42.0 + time * speed) * 6.2831853);
    let fine_wave = sin((uv.y * 210.0 - time * speed * 1.7) * 6.2831853);
    let roll = fract(time * speed * 0.08);
    let roll_band = smoothstep(0.08, 0.0, abs(uv.y - roll));
    let x_offset =
        (line_wave * 0.012 + fine_wave * 0.004 + roll_band * 0.035) * amount;
    let y_offset = sin(time * speed * 0.35) * 0.018 * amount;
    return uv + vec2f(x_offset, y_offset);
}

fn apply_slice_shift(
    uv: vec2f,
    amount: f32,
    slices: f32,
    offset: f32,
    axis: f32,
) -> vec2f {
    let use_vertical = step(0.5, axis);
    let coord = mix(uv.y, uv.x, use_vertical);
    let n = floor(max(slices, 1.0));
    let slice_index = floor(coord * n);
    let alternating = select(-1.0, 1.0, (i32(slice_index) & 1) == 0);
    let varied = alternating
        * (0.45 + 0.55 * hash21(vec2f(slice_index, n)));
    let shift = varied * offset * amount;
    return uv + mix(vec2f(shift, 0.0), vec2f(0.0, shift), use_vertical);
}

fn chroma_split(
    color: vec3f,
    uv: vec2f,
    amount: f32,
    spread: f32,
    angle: f32,
) -> vec3f {
    let direction = vec2f(cos(angle), sin(angle));
    let offset = direction * spread * amount;
    let shifted = vec3f(
        sample_video(uv + offset).r,
        color.g,
        sample_video(uv - offset).b,
    );
    return mix(color, shifted, amount);
}

fn posterize(color: vec3f, amount: f32, threshold: f32, levels: f32) -> vec3f {
    let level_count = max(levels, 2.0);
    let stepped = floor(color * level_count) / max(level_count - 1.0, 1.0);
    let mask = step(threshold, luma(color));
    let thresholded = mix(vec3f(0.02, 0.025, 0.03), vec3f(1.0), mask);
    return mix(color, mix(stepped, thresholded, amount), amount);
}

fn pixel_smear(
    color: vec3f,
    uv: vec2f,
    amount: f32,
    distance: f32,
    angle: f32,
) -> vec3f {
    let direction = vec2f(cos(angle), sin(angle));
    let lum = luma(color);
    let signed_luma = lum * 2.0 - 1.0;
    let offset = direction * signed_luma * distance * amount;
    let smear = (
        sample_video(uv + offset * 0.35)
        + sample_video(uv + offset * 0.7)
        + sample_video(uv + offset)
    ) / 3.0;
    return mix(color, smear, amount);
}

fn edge_glow(
    color: vec3f,
    uv: vec2f,
    amount: f32,
    radius: f32,
    gain: f32,
) -> vec3f {
    let dx = vec2f(radius, 0.0);
    let dy = vec2f(0.0, radius);
    let center = luma(sample_video(uv));
    let edge = abs(center - luma(sample_video(uv + dx)))
        + abs(center - luma(sample_video(uv - dx)))
        + abs(center - luma(sample_video(uv + dy)))
        + abs(center - luma(sample_video(uv - dy)));
    let glow = smoothstep(0.04, 0.28, edge * gain) * amount;
    return color + vec3f(0.35, 0.75, 1.0) * glow;
}

fn signal_texture(
    color: vec3f,
    uv: vec2f,
    time: f32,
    amount: f32,
    density: f32,
    noise_amount: f32,
) -> vec3f {
    let scan = 0.5 + 0.5 * sin((uv.y * density + time * 0.25) * 6.2831853);
    var out = color * mix(1.0, mix(0.72, 1.08, scan), amount);
    let noise = hash21(uv * vec2f(640.0, 1137.0) + vec2f(time, -time));
    out += (noise - 0.5) * noise_amount * amount;
    return out;
}

fn hue_mask(color: vec3f, targ: vec3f, width: f32) -> f32 {
    let c = normalize(max(color, vec3f(0.0001)));
    let t = normalize(targ);
    return smoothstep(1.0 - width, 1.0, dot(c, t));
}

fn natural_color_boost(
    color: vec3f,
    green_amount: f32,
    green_selectivity: f32,
    yellow_amount: f32,
    yellow_selectivity: f32,
) -> vec3f {
    let green_mask = hue_mask(color, vec3f(0.18, 0.78, 0.22), green_selectivity)
        * smoothstep(0.02, 0.38, color.g - max(color.r, color.b) * 0.72);
    let yellow_mask =
        hue_mask(color, vec3f(1.0, 0.74, 0.16), yellow_selectivity)
        * smoothstep(0.22, 0.86, luma(color));

    var out = color;
    out = mix(out, out * vec3f(0.86, 1.22, 0.78), green_mask * green_amount);
    out = mix(out, out * vec3f(1.24, 1.12, 0.72), yellow_mask * yellow_amount);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4f {
    let time = params.a.z;

    let chroma_amount = clamp(params.a.w, 0.0, 1.0);
    let chroma_spread = max(params.b.x, 0.0);
    let chroma_angle = params.b.y;

    let signal_amount = clamp(params.b.z, 0.0, 1.0);
    let signal_density = max(params.b.w, 1.0);
    let signal_noise = clamp(params.c.x, 0.0, 1.0);

    let poster_amount = clamp(params.c.y, 0.0, 1.0);
    let poster_threshold = clamp(params.c.z, 0.0, 1.0);
    let poster_levels = max(params.c.w, 2.0);

    let smear_amount = clamp(params.d.x, 0.0, 1.0);
    let smear_distance = max(params.d.y, 0.0);
    let smear_angle = params.d.z;

    let vhs_amount = clamp(params.d.w, 0.0, 1.0);
    let edge_amount = clamp(params.e.x, 0.0, 1.0);
    let edge_radius = max(params.e.y, 0.0);
    let edge_gain = max(params.e.z, 0.0);

    let slice_amount = clamp(params.e.w, 0.0, 1.0);
    let slice_count = max(params.f.x, 1.0);
    let slice_offset = max(params.f.y, 0.0);
    let slice_axis = params.f.z;

    let green_boost = clamp(params.f.w, 0.0, 1.0);
    let green_selectivity = clamp(params.g.x, 0.03, 0.95);
    let yellow_boost = clamp(params.g.y, 0.0, 1.0);
    let yellow_selectivity = clamp(params.g.z, 0.03, 0.95);

    var warped_uv = apply_vhs_warp(in.uv, time, vhs_amount, 1.0);
    warped_uv = apply_slice_shift(
        warped_uv,
        slice_amount,
        slice_count,
        slice_offset,
        slice_axis,
    );
    var color = sample_video(warped_uv);

    color = chroma_split(
        color, warped_uv, chroma_amount, chroma_spread, chroma_angle,
    );
    color = posterize(color, poster_amount, poster_threshold, poster_levels);
    color = pixel_smear(
        color, warped_uv, smear_amount, smear_distance, smear_angle,
    );
    color = natural_color_boost(
        color,
        green_boost,
        green_selectivity,
        yellow_boost,
        yellow_selectivity,
    );
    color = edge_glow(color, warped_uv, edge_amount, edge_radius, edge_gain);
    color = signal_texture(
        color, in.uv, time, signal_amount, signal_density, signal_noise,
    );

    let vignette = smoothstep(0.95, 0.25, length(in.uv - vec2f(0.5, 0.5)));
    color *= mix(1.0, vignette, 0.12 * luma(color));

    return vec4f(clamp(color, vec3f(0.0), vec3f(1.0)), 1.0);
}
