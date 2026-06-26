const TAU: f32 = 6.28318530718;

const STANDARD_LUMINANCE: vec3f = vec3f(0.2126, 0.7152, 0.0722);

var<private> OFFSETS: array<vec2f, 4> = array<vec2f, 4>(
    vec2f(-1.0, 0.0),
    vec2f(1.0, 0.0),
    vec2f(0.0, -1.0),
    vec2f(0.0, 1.0)
);

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
    @location(1) uv: vec2f
};

struct Params {
    // w, h, t, grid_size
    a: vec4f,
    // circle_radius, line_width, a_freq, a_amp
    b: vec4f,
    // ab_mix, t_wave, a_exp, b_exp
    c: vec4f,
    // ac_rotate, a_rotation_speed, invert, ab_cd_mix
    d: vec4f,
    // palette_hue, hue_spread, saturation, color_amount
    e: vec4f,
    // cd_mix, c_amp, d_freq, color_contrast
    f: vec4f,
    // outer_spread, feedback, band_dist, b_freq
    g: vec4f,
    // dry_add, b_amp, d_freq, d_amp
    h: vec4f,
    // link_ab_amp, link_ab_freq, link_cd_amp, link_cd_freq
    i: vec4f,
    // t_cycle_beats
    j: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@group(1) @binding(0)
var source_sampler: sampler;

@group(1) @binding(1)
var source_texture: texture_2d<f32>;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.pos = vert.position;
    out.uv = out.pos * 0.5 + 0.5;
    return out;
}

@fragment
fn fs_main(
    @location(0) position: vec2f,
    @location(1) uv: vec2f
) -> @location(0) vec4f {
    let t = params.a.z * TAU / params.j.x;
    let grid_size = params.a.w;
    var circle_radius = params.b.x;
    let line_width = params.b.y;
    let a_freq = params.b.z;
    let a_amp = params.b.w;
    let ab_mix = params.c.x;
    let t_wave = params.c.y;
    let invert = params.d.z == 1.0;
    let ab_cd_mix = params.d.w;
    let palette_hue = params.e.x;
    let hue_spread = params.e.y;
    let saturation = params.e.z;
    let color_amount = params.e.w;
    let cd_mix = params.f.x;
    let c_amp = params.f.y;
    let c_freq = params.f.z;
    let color_contrast = params.f.w;
    let outer_spread = params.g.x;
    let feedback = params.g.y;
    let dry_add = params.h.x;
    let band_dist = params.g.z;
    let link_ab_amp = params.i.x == 1.0;
    let link_ab_freq = params.i.y == 1.0;
    let link_cd_amp = params.i.z == 1.0;
    let link_cd_freq = params.i.w == 1.0;
    var b_freq = select(params.g.w, a_freq, link_ab_freq);
    var b_amp = select(params.h.y, a_amp, link_ab_amp);
    var d_freq = select(params.h.z, c_freq, link_cd_freq);
    var d_amp = select(params.h.w, c_amp, link_cd_amp);

    let p = correct_aspect(position);
    let grid_pos = fract(p * grid_size) * 2.0 - 1.0;

    let v0 = mix(
        weave_a(vec2f(0.0), p, a_freq) * a_amp,
        weave_b(vec2f(0.0), p, b_freq) * b_amp,
        ab_mix
    );
    let v1 = mix(
        weave_c(vec2f(0.0), p, c_freq * 2.0) * c_amp,
        weave_d(vec2f(0.0), p, d_freq * 3.0) * d_amp,
        cd_mix
    );

    var wave_pattern = mix(v0, v1, ab_cd_mix);

    let displacement = n(tan(wave_pattern + t) * t_wave);
    let radius_variation = circle_radius * displacement;

    let dist = length(grid_pos);
    let cr = radius_variation - line_width;
    let outer = smoothstep(cr - outer_spread, cr + outer_spread, dist);
    let inner = smoothstep(
        radius_variation + 0.01,
        radius_variation - 0.01,
        dist
    );
    let circle_outline = outer * inner;

    let steps = 4.0;
    let color_band_select = smoothstep(1.0, 0.0, displacement) *
        (dist * band_dist);
    let color_band_quantized = floor(
        mix(
            displacement - dist,
            color_band_select + dist,
            color_contrast
        ) * steps
    ) / steps;

    let band_norm = clamp(
        color_band_quantized * 0.5 + 0.5,
        0.0,
        1.0
    );
    let shaped_band = smoothstep(
        0.5 - color_contrast * 0.45,
        0.5 + color_contrast * 0.45,
        band_norm
    );
    let hue = fract(
        palette_hue + (shaped_band - 0.5) * hue_spread
    );
    let accent_hue = fract(hue + hue_spread * 0.35);

    let dark_background = hsv_to_rgb(vec3f(
        hue,
        saturation * mix(0.45, 0.75, shaped_band),
        mix(0.035, 0.24, shaped_band)
    ));
    let dark_outline = hsv_to_rgb(vec3f(
        accent_hue,
        saturation,
        mix(0.42, 0.95, clamp(displacement, 0.0, 1.0))
    ));
    let light_background = hsv_to_rgb(vec3f(
        hue,
        saturation * mix(0.06, 0.20, shaped_band),
        mix(0.97, 0.88, shaped_band)
    ));
    let light_outline = hsv_to_rgb(vec3f(
        accent_hue,
        saturation * 0.72,
        mix(0.28, 0.68, clamp(displacement, 0.0, 1.0))
    ));

    let palette_background = select(
        dark_background,
        light_background,
        invert
    );
    let palette_outline = select(
        dark_outline,
        light_outline,
        invert
    );
    let palette_color = mix(
        palette_background,
        palette_outline,
        circle_outline
    );
    let gray = vec3f(dot(palette_color, STANDARD_LUMINANCE));
    var color = mix(gray, palette_color, color_amount);

    let luminance = (color.r + color.g + color.b) / 3.0;
    let mask = smoothstep(0.4, 0.9, luminance);

    let fb_color = apply_feedback(color, p, uv, feedback) ;
    color = mix(color, fb_color, mask) + (color * dry_add);

    return vec4f(color, 1.0);
}

fn apply_feedback(
    color: vec3f,
    p: vec2f,
    uv: vec2f,
    mix_amount: f32
) -> vec3f {
    var best_offset = vec2f(0.0);
    var max_brightness = 0.0;
    let pixel_size = vec2f(1.0 / params.a.x, 1.0 / params.a.y);
    let zoom = 0.97;
    let flipped_uv = vec2f(uv.x, 1.0 - uv.y);
    let centered_uv = (flipped_uv - 0.5) * zoom + 0.5;

    for (var i = 0; i < 4; i++) {
        let sample_uv = centered_uv + (OFFSETS[i] * pixel_size);
        let color = textureSample(source_texture, source_sampler, sample_uv);
        let brightness = dot(color.rgb, STANDARD_LUMINANCE);
        if (brightness > max_brightness) {
            max_brightness = brightness;
            best_offset = OFFSETS[i];
        }
    }

    let sample = textureSample(
        source_texture,
        source_sampler,
        centered_uv + best_offset * 0.01
    );

    let sample_rgb = sample.rgb;
    let sample_brightness = dot(sample_rgb, STANDARD_LUMINANCE);
    let is_dark = sample_brightness < 0.01;

    return select(mix(color, sample_rgb, mix_amount), color, is_dark);
}

fn powf(x: f32, y: f32) -> f32 {
    let y_rounded = round(y);
    if (abs(y - y_rounded) < 1e-4 && modulo(y_rounded, 2.0) == 1.0) {
        return sign(x) * pow(abs(x), y);
    }
    return pow(abs(x), y);
}

fn modulo(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}

fn weave_a(p1: vec2f, p2: vec2f, frequency: f32) -> f32 {
    let t = params.a.z;
    let exp = params.c.z;
    let ac_rotate = params.d.x == 1.0;
    let a_rotation_speed = params.d.y;
    let rotation = select(135.0, (t * a_rotation_speed) % 360.0, ac_rotate);
    let p = rotate_point(p2, rotation);
    let dx = powf(abs(p2.x - p1.x), exp);
    let dy = powf(abs(p2.y - p1.y), exp);
    return (sin(p.x * frequency) + sin(p.y * frequency))
        * sin(sqrt(dx + dy) * 0.05) * 100.0;
}

fn weave_b(p1: vec2f, p2: vec2f, frequency: f32) -> f32 {
    let exp = params.c.w;
    let dx = powf(abs(p2.x - p1.x), exp);
    let dy = powf(abs(p2.y - p1.y), exp);
    let wave_pattern = cos(p2.x * frequency) + sin(p2.y * frequency);
    return wave_pattern * sin(sqrt(dx + dy) * 0.05) * 100.0;
}

fn weave_c(p1: vec2f, p2: vec2f, frequency: f32) -> f32 {
    let t = params.a.z;
    let exp = params.c.z;
    let ac_rotate = params.d.x == 1.0;
    let a_rotation_speed = params.d.y;
    let rotation = select(135.0, (t * a_rotation_speed) % 360.0, ac_rotate);
    let p = rotate_point(p2, rotation);
    let dx = powf(abs(p2.x - p1.x), exp);
    let dy = powf(abs(p2.y - p1.y), exp);
    return (sin(p.x * frequency) + sin(p.y * frequency))
        * sin(exp(-length(vec2f(dx, dy))) * 5.0) * 10.0;
}

fn weave_d(p1: vec2f, p2: vec2f, frequency: f32) -> f32 {
    let exp = params.c.w;
    let dx = powf(abs(p2.x - p1.x), exp);
    let dy = powf(abs(p2.y - p1.y), exp);
    let wave_pattern = cos(p2.x * frequency) + sin(p2.y * frequency);
    let distance = length(vec2f(dx, dy));
    let angle_factor = abs(atan2(dy, dx));
    let center_distance = length(p2 - p1);
    let blend = smoothstep(0.0, 0.2, center_distance);
    let modified_angle = mix(1.0, angle_factor, blend);
    return wave_pattern * modified_angle * 2.0;
}

fn n(v: f32) -> f32 {
    return v * 0.5 + 0.5;
}

fn hsv_to_rgb(c: vec3f) -> vec3f {
    let k = vec4f(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + k.xyz) * 6.0 - k.www);
    return c.z * mix(k.xxx, clamp(p - k.xxx, vec3f(0.0), vec3f(1.0)), c.y);
}

fn correct_aspect(pos: vec2f) -> vec2f {
    let aspect = params.a.x / params.a.y;
    return vec2f(pos.x * aspect, pos.y);
}

fn rotate_point(pos: vec2f, degrees: f32) -> vec2f {
    let radians = degrees * 3.14159 / 180.0;
    let c = cos(radians);
    let s = sin(radians);
    return vec2f(
        pos.x * c - pos.y * s,
        pos.x * s + pos.y * c
    );
}
