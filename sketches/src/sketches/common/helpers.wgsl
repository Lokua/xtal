// -----------------------------------------------------------------------------
//  CONSTANTS
// -----------------------------------------------------------------------------

const PI: f32 = 3.14159265359;
const TAU: f32 = 6.283185307179586;
const PHI: f32 = 1.61803398875;
const EPSILON: f32 = 1.1920929e-7;

// -----------------------------------------------------------------------------
//  GENERAL
// -----------------------------------------------------------------------------

// wgsl % operator is a remainder operator, not modulo
fn modulo(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}
fn mod_1(x: f32) -> f32 {
    return modulo(x, 1.0);
}
fn mod_v2(x: vec2f, y: vec2f) -> vec2f {
    return x - y * floor(x / y);
}
fn mod_v3(x: vec3f, y: vec3f) -> vec3f {
    return x - y * floor(x / y);
}
fn mod_v4(x: vec4f, y: vec4f) -> vec4f {
    return x - y * floor(x / y);
}

fn powf(x: f32, y: f32) -> f32 {
    let y_rounded = round(y);
    if (abs(y - y_rounded) < 1e-4 && modulo(y_rounded, 2.0) == 1.0) {
        return sign(x) * pow(abs(x), y);
    }
    return pow(abs(x), y);
}

// smooth minimum
fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = max(k - abs(a - b), 0.0) / k;
    return min(a, b) - h * h * k * 0.25;
}

fn smax(a: f32, b: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(a, b, h) - k * h * (1.0 - h);
}

fn rotate_x(p: vec3f, radians: f32) -> vec3f {
    let c = cos(radians);
    let s = sin(radians);
    
    return vec3f(
        p.x,
        p.y * c - p.z * s,
        p.y * s + p.z * c
    );
}

fn rotate_y(p: vec3f, radians: f32) -> vec3f {
    let c = cos(radians);
    let s = sin(radians);
    
    return vec3f(
        p.x * c - p.z * s,
        p.y,
        p.x * s + p.z * c
    );
}

fn rotate_z(p: vec3f, radians: f32) -> vec3f {
    let c = cos(radians);
    let s = sin(radians);
    
    return vec3f(
        p.x * c - p.y * s,
        p.x * s + p.y * c,
        p.z
    );
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

fn rotate(v: vec2f, angle: f32) -> vec2f {
    let c = cos(angle);
    let s = sin(angle);
    return vec2f(c * v.x - s * v.y, s * v.x + c * v.y);
}

fn n(x: f32) -> f32 {
    return x * 0.5 + 0.5;
}

// -----------------------------------------------------------------------------
//  RANDOM
// -----------------------------------------------------------------------------

fn fbm(p: vec2f) -> f32 {
    let OCTAVES = 5;
    let G = 0.5;

    var value = 0.0;
    var amplitude = 1.0;
    var frequency = 1.0;

    for (var i = 0; i < OCTAVES; i++) {
        value = value + random2(p * frequency) * amplitude;
        frequency = frequency * 2.0;
        amplitude = amplitude * G;
    }

    return value;
}

fn random2(p: vec2f) -> f32 {
    return fract(sin(dot(p, vec2f(12.9898, 78.233))) * 43758.5453);
}

// 2D noise functions adapted from:
// https://gist.github.com/patriciogonzalezvivo/670c22f3966e662d2f83
// Not sure what's better? random_v2 or hash...
fn hash(p: vec2f) -> f32 {
    let p3 = fract(vec3f(p.xyx) * 0.13);
    let p4 = p3 + vec3f(7.0, 157.0, 113.0);
    return fract(dot(p4, vec3f(268.5453123, 143.2354234, 424.2424234)));
}

// Basic random number generation (PCG)
fn rand_pcg(seed: u32) -> f32 {
    var state = seed * 747796405u + 2891336453u;
    var word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    var result = (word >> 22u) ^ word;
    return f32(result) / 4294967295.0;
}

// Box-Muller transform for normal distribution
fn random_normal(seed: u32, mean: f32, stddev: f32) -> f32 {
    let u1 = rand_pcg(seed);
    let u2 = rand_pcg(seed + 1u);
    
    let mag = sqrt(-2.0 * log(u1));
    let z0 = mag * cos(6.28318530718 * u2);
    
    return mean + stddev * z0;
}

fn rand_hash_dot3(p: vec2f) -> f32 {
    let p3 = fract(vec3f(p.xyx) * 0.13);
    let p4 = p3 + vec3f(7.0, 157.0, 113.0);
    return fract(dot(p4, vec3f(268.5453123, 143.2354234, 424.2424234)));
}

fn rand_fract(p: vec2f) -> f32 {
    let q = fract(p * vec2f(123.34, 456.21));
    return fract(q.x * q.y * 19.19);
}

fn rand_3d_hash_collapse(p: vec2f) -> f32 {
    let p3 = fract(vec3f(p.xyx) * 0.1031);
    let p4 = p3 + dot(p3, p3.yzx + 19.19);
    return fract((p4.x + p4.y) * p4.z);
}

fn rand_bit_style(p: vec2f) -> f32 {
    let k1 = 0.3183099; // 1/PI
    let k2 = 0.3678794; // 1/e
    let x = sin(dot(p, vec2f(127.1, 311.7))) * 43758.5453;
    return fract(x * k1 + k2);
}

fn rand_int_floor(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let n = dot(i, vec2f(1.0, 57.0));
    return fract(sin(n) * 43758.5453123);
}

// -----------------------------------------------------------------------------
//  COLOR
// -----------------------------------------------------------------------------

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

fn lch_to_rgb(lch: vec3f) -> vec3f {
    let l = lch.x;
    let c = lch.y; 
    let h_deg = lch.z;

    // Convert LCH to Lab
    let h_rad = radians(h_deg);
    let a = cos(h_rad) * c;
    let b = sin(h_rad) * c;

    // Convert Lab to XYZ
    let fy = (l + 16.0) / 116.0;
    let fx = fy + (a / 500.0);
    let fz = fy - (b / 200.0);

    let fx3 = pow(fx, 3.0);
    let fz3 = pow(fz, 3.0);
    let x = select((fx - 16.0 / 116.0) / 7.787, fx3, fx3 > 0.008856);
    let y = select(
        (fy - 16.0 / 116.0) / 7.787, 
        pow(fy, 3.0), 
        pow(fy, 3.0) > 0.008856
    );
    let z = select((fz - 16.0 / 116.0) / 7.787, fz3, fz3 > 0.008856);

    // D65 white point
    let X = x * 0.95047;
    let Y = y;
    let Z = z * 1.08883;

    // Convert XYZ to linear RGB
    let r_lin =  3.2406 * X - 1.5372 * Y - 0.4986 * Z;
    let g_lin = -0.9689 * X + 1.8758 * Y + 0.0415 * Z;
    let b_lin =  0.0557 * X - 0.2040 * Y + 1.0570 * Z;

    // Linear to sRGB gamma correction
    return vec3f(
        clamp(gamma_correct(r_lin), 0.0, 1.0),
        clamp(gamma_correct(g_lin), 0.0, 1.0),
        clamp(gamma_correct(b_lin), 0.0, 1.0)
    );
}

fn gamma_correct(c: f32) -> f32 {
    return select(12.92 * c, 1.055 * pow(c, 1.0 / 2.4) - 0.055, c > 0.0031308);
}

// returned components are not normalized; 
// ranges: ([0.0, 100.0], [0.0, 100.0], [0.0, 360.0])
fn rgb_to_lch(rgb: vec3f) -> vec3f {
    let rgb_lin = linearize_rgb(rgb);

    // Linear RGB to XYZ
    let x = 0.4124 * rgb_lin.x + 0.3576 * rgb_lin.y + 0.1805 * rgb_lin.z;
    let y = 0.2126 * rgb_lin.x + 0.7152 * rgb_lin.y + 0.0722 * rgb_lin.z;
    let z = 0.0193 * rgb_lin.x + 0.1192 * rgb_lin.y + 0.9505 * rgb_lin.z;

    // Normalize by D65 white point
    let xn = x / 0.95047;
    let yn = y;
    let zn = z / 1.08883;

    // Convert to Lab
    let fx = lab_f(xn);
    let fy = lab_f(yn);
    let fz = lab_f(zn);

    let l = 116.0 * fy - 16.0;
    let a = 500.0 * (fx - fy);
    let b = 200.0 * (fy - fz);

    // Convert to LCH
    let c = length(vec2f(a, b));
    let h_rad = atan2(b, a);
    let h = fract(degrees(h_rad) / 360.0) * 360.0;

    return vec3f(l, c, h);
}

fn inv_gamma_correct(c: f32) -> f32 {
    return select(c / 12.92, pow((c + 0.055) / 1.055, 2.4), c > 0.04045);
}

fn linearize_rgb(rgb: vec3f) -> vec3f {
    return vec3f(
        inv_gamma_correct(rgb.x),
        inv_gamma_correct(rgb.y),
        inv_gamma_correct(rgb.z)
    );
}

fn lab_f(t: f32) -> f32 {
    return select(pow(t, 1.0 / 3.0), 7.787 * t + 16.0 / 116.0, t > 0.008856);
}

fn rgb_to_hsl(rgb: vec3f) -> vec3f {
    let r = rgb.x;
    let g = rgb.y;
    let b = rgb.z;

    let max_c = max(max(r, g), b);
    let min_c = min(min(r, g), b);
    let delta = max_c - min_c;

    let l = (max_c + min_c) * 0.5;

    var h = 0.0;
    var s = 0.0;

    if delta > 0.0 {
        s = delta / (1.0 - abs(2.0 * l - 1.0));

        if max_c == r {
            h = (g - b) / delta;
            if g < b {
                h += 6.0;
            }
        } else if max_c == g {
            h = (b - r) / delta + 2.0;
        } else {
            h = (r - g) / delta + 4.0;
        }

        h /= 6.0;
    }

    return vec3f(h, s, l);
}

fn hsl_to_rgb(hsl: vec3f) -> vec3f {
    let h = hsl.x;
    let s = hsl.y;
    let l = hsl.z;

    if s == 0.0 {
        return vec3f(l, l, l);
    }

    let q = select(l * (1.0 + s), l + s - l * s, l < 0.5);
    let p = 2.0 * l - q;

    return vec3f(
        hue_to_rgb(p, q, h + 1.0 / 3.0),
        hue_to_rgb(p, q, h),
        hue_to_rgb(p, q, h - 1.0 / 3.0)
    );
}

fn hue_to_rgb(p: f32, q: f32, t: f32) -> f32 {
    var t_mod = t;
    if t_mod < 0.0 { t_mod += 1.0; }
    if t_mod > 1.0 { t_mod -= 1.0; }

    if t_mod < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t_mod;
    } else if t_mod < 0.5 {
        return q;
    } else if t_mod < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t_mod) * 6.0;
    } else {
        return p;
    }
}

fn rgb_to_oklch(rgb: vec3f) -> vec3f {
    let r = select(
        rgb.x / 12.92, 
        pow((rgb.x + 0.055) / 1.055, 2.4), 
        rgb.x > 0.04045
    );
    let g = select(
        rgb.y / 12.92, 
        pow((rgb.y + 0.055) / 1.055, 2.4), 
        rgb.y > 0.04045
    );
    let b = select(
        rgb.z / 12.92, 
        pow((rgb.z + 0.055) / 1.055, 2.4), 
        rgb.z > 0.04045
    );

    let l = 0.41222147 * r + 0.53633254 * g + 0.05144599 * b;
    let m = 0.21190350 * r + 0.68069954 * g + 0.10739696 * b;
    let s = 0.08830246 * r + 0.28171884 * g + 0.62997870 * b;

    let l_ = pow(l, 1.0 / 3.0);
    let m_ = pow(m, 1.0 / 3.0);
    let s_ = pow(s, 1.0 / 3.0);

    let ok_l = 0.21045426 * l_ + 0.79361779 * m_ - 0.00407205 * s_;
    let ok_a = 1.97799850 * l_ - 2.42859220 * m_ + 0.45059371 * s_;
    let ok_b = 0.02590404 * l_ + 0.78277177 * m_ - 0.80867577 * s_;

    let c = length(vec2f(ok_a, ok_b));
    let h = fract(degrees(atan2(ok_b, ok_a)) / 360.0);

    return vec3f(ok_l, c, h);
}

fn oklch_to_rgb(oklch: vec3f) -> vec3f {
    let l = oklch.x;
    let c = oklch.y;
    let h = oklch.z * 360.0;

    let cx = cos(radians(h)) * c;
    let cy = sin(radians(h)) * c;

    let l_ = l + 0.39633778 * cx + 0.21580376 * cy;
    let m_ = l - 0.10556135 * cx - 0.06385417 * cy;
    let s_ = l - 0.08948418 * cx - 1.29148555 * cy;

    let l3 = l_ * l_ * l_;
    let m3 = m_ * m_ * m_;
    let s3 = s_ * s_ * s_;

    let r_lin = 4.07674166 * l3 - 3.30771159 * m3 + 0.23096993 * s3;
    let g_lin = -1.26843800 * l3 + 2.60975740 * m3 - 0.34131940 * s3;
    let b_lin = -0.00419609 * l3 - 0.70341861 * m3 + 1.70761470 * s3;

    let r = select(
        12.92 * r_lin, 
        1.055 * pow(r_lin, 1.0 / 2.4) - 0.055, 
        r_lin > 0.0031308
    );
    let g = select(
        12.92 * g_lin, 
        1.055 * pow(g_lin, 1.0 / 2.4) - 0.055, 
        g_lin > 0.0031308
    );
    let b = select(
        12.92 * b_lin, 
        1.055 * pow(b_lin, 1.0 / 2.4) - 0.055, 
        b_lin > 0.0031308
    );

    return clamp(vec3f(r, g, b), vec3f(0.0), vec3f(1.0));
}

fn mix_additive(c1: vec3f, c2: vec3f) -> vec3f {
    return clamp(c1 + c2, vec3f(0.0), vec3f(1.0));
    
}

fn mix_subtractive(c1: vec3f, c2: vec3f) -> vec3f {
    return clamp(c1 * c2, vec3f(0.0), vec3f(1.0));
}

fn mix_multiply(c1: vec3f, c2: vec3f) -> vec3f {
    return c1 * c2;
}

fn mix_screen(c1: vec3f, c2: vec3f) -> vec3f {
    return 1.0 - (1.0 - c1) * (1.0 - c2);
}

fn mix_overlay(c1: vec3f, c2: vec3f) -> vec3f {
    return select(
        2.0 * c1 * c2, 
        1.0 - 2.0 * (1.0 - c1) * (1.0 - c2), 
        c1 <= vec3(0.5)
    );
}

fn mix_max(c1: vec4f, c2: vec4f) -> vec4f {
    return max(c1, c2);
}

fn mix_min(c1: vec4f, c2: vec4f) -> vec4f {
    return min(c1, c2);
}

fn mix_hue_shift(c1: vec3f, c2: vec3f, t: f32) -> vec3f {
    let h1 = atan2(c1.g - c1.b, c1.r - c1.g);
    let h2 = atan2(c2.g - c2.b, c2.r - c2.g);
    let new_hue = mix(h1, h2, t);

    let len1 = length(vec3(c1.r, c1.g, c1.b));
    return vec3(len1 * cos(new_hue), len1 * sin(new_hue), c1.b);
}

fn mix_average(c1: vec3f, c2: vec3f) -> vec3f {
    return (c1 + c2) / 2.0;
}

fn mix_dodge(c1: vec3f, c2: vec3f) -> vec3f {
    return clamp(c1 / (1.0 - c2), vec3f(0.0), vec3f(1.0));
}

fn mix_burn(c1: vec3f, c2: vec3f) -> vec3f {
    return 1.0 - clamp((1.0 - c1) / c2, vec3f(0.0), vec3f(1.0));
}

fn swap_rgb(c: vec3f, mode: i32) -> vec3f {
    // Original and single-channel fills
    if (mode == 0) { return c; }
    if (mode == 1) { return c.rrr; }
    if (mode == 2) { return c.ggg; }
    if (mode == 3) { return c.bbb; }
    
    // Double-channel duplications (one channel copied to another)
    if (mode == 4) { return c.rrg; }
    if (mode == 5) { return c.rrb; }
    if (mode == 6) { return c.rgg; }
    if (mode == 7) { return c.rbb; }
    if (mode == 8) { return c.grr; }
    if (mode == 9) { return c.brr; }
    if (mode == 10) { return c.ggr; }
    if (mode == 11) { return c.bbr; }
    
    // Channel permutations (all three channels used, just reordered)
    if (mode == 12) { return c.rbg; }
    if (mode == 13) { return c.grb; }
    if (mode == 14) { return c.gbr; }
    if (mode == 15) { return c.brg; }
    if (mode == 16) { return c.bgr; }
    
    return c;
}

// -----------------------------------------------------------------------------
//  POST PROCESSING
// -----------------------------------------------------------------------------

fn film_grain(color: vec3f, p: vec2f, intensity: f32) -> vec3f {
    let random = random2(p);
    return clamp(color + (random - 0.5) * intensity, vec3f(0.0), vec3f(1.0));
}

fn glitch_blocks(
    color: vec3f, 
    p: vec2f, 
    block_size: f32, 
    intensity: f32
) -> vec3f {
    let block = floor(p * block_size);
    let noise = fract(sin(dot(block, vec2f(12.9898, 78.233))) * 43758.5453);
    return mix(color, vec3f(1.0) - color, step(1.0 - intensity, noise));
}

// -----------------------------------------------------------------------------
//  FEEDBACK
// -----------------------------------------------------------------------------

// const STANDARD_LUMINANCE: vec3f = vec3f(0.2126, 0.7152, 0.0722);

// var<private> OFFSETS: array<vec2f, 4> = array<vec2f, 4>(
//     vec2f(-1.0, 0.0),
//     vec2f(1.0, 0.0),
//     vec2f(0.0, -1.0),
//     vec2f(0.0, 1.0)
// );

// // Commented out to avoid linter errors
// fn apply_feedback(color: vec3f, p: vec2f, uv: vec2f, mix: f32) -> vec3f {
//     var best_offset = vec2f(0.0);
//     var max_brightness = 0.0;
//     let pixel_size = vec2f(1.0 / params.a.x, 1.0 / params.a.y);

//     for (var i = 0; i < 4; i++) {
//         let sample_uv = uv + (OFFSETS[i] * pixel_size);
//         let color = textureSample(source_texture, source_sampler, sample_uv);
//         let brightness = dot(color.rgb, STANDARD_LUMINANCE);
//         if (brightness > max_brightness) {
//             max_brightness = brightness;
//             best_offset = OFFSETS[i];
//         }
//     }

//     let sample = textureSample(
//         source_texture, 
//         source_sampler, 
//         uv + best_offset * 0.01
//     );

//     return mix(color, 1.0 - sample.rgb, mix);
// }