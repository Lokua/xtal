const PI: f32 = 3.14159265359;
const TAU: f32 = 6.283185307179586;

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
    // circle_radius, line_width, freq, amp
    b: vec4f,
    // ab_mix, t_wave, a_exp, b_exp
    c: vec4f,
    // ac_rotate, a_rotation_speed, invert, ab_cd_mix
    d: vec4f,
    // red_or_cyan, blue_or_magenta, green_or_yellow, colorize
    e: vec4f,
    // cd_mix, cd_amp, cd_freq, norm_color_disp
    f: vec4f,
    // outer_spread, feedback, band_dist, UNUSED
    g: vec4f,
    // dry_add, ...
    h: vec4f,
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
    let t = params.a.z;
    let grid_size = params.a.w;
    var circle_radius = params.b.x;
    let line_width = params.b.y;
    let ab_freq = params.b.z;
    let ab_amp = params.b.w;
    let ab_mix = params.c.x;
    let t_wave = params.c.y;
    let invert = params.d.z == 1.0;
    let ab_cd_mix = params.d.w;
    let red_or_cyan = params.e.x;
    let green_or_magenta = params.e.y;
    let blue_or_yellow = params.e.z;
    let colorize = params.e.w;
    let cd_mix = params.f.x;
    let cd_amp = params.f.y;
    let cd_freq = params.f.z;
    let norm_color_disp = params.f.w;
    let outer_spread = params.g.x;
    let feedback = params.g.y;
    let dry_add = params.h.x;
    let band_dist = params.g.z;

    let p = correct_aspect(position);
    let grid_pos = fract(p * grid_size) * 2.0 - 1.0;

    let v0 = mix(
        weave_a(vec2f(0.0), p, ab_freq) * ab_amp,
        weave_b(vec2f(0.0), p, ab_freq) * ab_amp,
        ab_mix
    );
    let v1 = mix(
        weave_c(vec2f(0.0), p, cd_freq * 2.0) * cd_amp,
        weave_d(vec2f(0.0), p, cd_freq * 3.0) * cd_amp,
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
            norm_color_disp
        ) * steps
    ) / steps;

    let base_color = vec3f(
        red_or_cyan * sin(color_band_quantized * PI),
        green_or_magenta * cos(color_band_quantized * PI),
        blue_or_yellow * sin(color_band_quantized * PI)
    );

    let color_intensity = mix(0.1, 1.0, color_band_quantized);
    let color_component = base_color * color_intensity;
    let background_mask = 1.0 - smoothstep(0.0, 0.1, circle_outline);
    let color_background = color_component * background_mask;
    let circle_color = vec3f(displacement) * circle_outline;
    let colorized = circle_color + color_background;

    var color = mix(
        vec3f(displacement) * circle_outline, 
        colorized, 
        colorize
    );

    color = select(color, 1.0 - color, invert);

    let luminance = (color.r + color.g + color.b) / 3.0;
    let mask = smoothstep(0.4, 0.9, luminance);

    let fb_color = apply_feedback(color, p, uv, feedback) ;
    color = mix(color, fb_color, mask) + (color * dry_add);

    return vec4f(color, 1.0);
}

fn apply_feedback(color: vec3f, p: vec2f, uv: vec2f, mix: f32) -> vec3f {
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
    
    return select(mix(color, 1.0 - sample_rgb, mix), color, is_dark);
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

fn rotate_point(p: vec2f, angle_degrees: f32) -> vec2f {
    let angle = radians(angle_degrees);
    let cos_angle = cos(angle);
    let sin_angle = sin(angle);
    
    return vec2f(
        p.x * cos_angle - p.y * sin_angle,
        p.x * sin_angle + p.y * cos_angle
    );
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = params.a.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}

fn n(x: f32) -> f32 {
    return x * 0.5 + 0.5;
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

fn mod_1(x: f32) -> f32 {
    return modulo(x, 1.0);
}

fn rgb_to_hsv(rgb: vec3f) -> vec3f {
    let r = rgb.x;
    let g = rgb.y;
    let b = rgb.z;
    
    let cmax = max(max(r, g), b);
    let cmin = min(min(r, g), b);
    let delta = cmax - cmin;
    
    var h = 0.0;
    if (delta > 0.0) {
        if (cmax == r) {
            h = (g - b) / delta;
            if (h < 0.0) {
                h += 6.0;
            }
        } else if (cmax == g) {
            h = ((b - r) / delta) + 2.0;
        } else {
            h = ((r - g) / delta) + 4.0;
        }
        h /= 6.0;
    }
    
    var s = 0.0;
    if (cmax > 0.0) {
        s = delta / cmax;
    }
    
    let v = cmax;
    
    return vec3f(h, s, v);
}

fn hsv_to_rgb(hsv: vec3f) -> vec3f {
    let h = hsv.x;
    let s = hsv.y;
    let v = hsv.z;
    
    if (s == 0.0) {
        return vec3f(v, v, v);
    }
    
    let i = floor(h * 6.0);
    let f = h * 6.0 - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    
    var r = 0.0;
    var g = 0.0;
    var b = 0.0;
    
    if (i % 6.0 == 0.0) {
        r = v; g = t; b = p;
    } else if (i % 6.0 == 1.0) {
        r = q; g = v; b = p;
    } else if (i % 6.0 == 2.0) {
        r = p; g = v; b = t;
    } else if (i % 6.0 == 3.0) {
        r = p; g = q; b = v;
    } else if (i % 6.0 == 4.0) {
        r = t; g = p; b = v;
    } else {
        r = v; g = p; b = q;
    }
    
    return vec3f(r, g, b);
}



