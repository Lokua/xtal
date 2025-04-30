// -----------------------------------------------------------------------------
//  CONSTANTS
// -----------------------------------------------------------------------------

const PI: f32 = 3.14159265359;
const TAU: f32 = 6.283185307179586;
const PHI: f32 = 1.61803398875;
const EPSILON: f32 = 1.1920929e-7;

// -----------------------------------------------------------------------------
//  UTILS
// -----------------------------------------------------------------------------

fn random_v2(p: vec2f) -> f32 {
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

fn n(x: f32) -> f32 {
    return x * 0.5 + 0.5;
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

// -----------------------------------------------------------------------------
//  POST PROCESSING
// -----------------------------------------------------------------------------

fn film_grain(color: vec3f, p: vec2f, intensity: f32) -> vec3f {
    let random = random_v2(p);
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