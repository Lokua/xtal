// Following along at https://iquilezles.org/articles/warp/

const TAU: f32 = 6.283185307179586;

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, t, amp
    a: vec4f,
    // freq, l, c, h
    b: vec4f,
    // ma1a, ma2a, ma3a, ma4a
    c: vec4f,
    // ma1b, ma2b, ma3b, ma4b
    d: vec4f,
    // contrast, use_ma_b, hash_alg, show_masks
    e: vec4f,
    // grain_size, swirl, posterize, posterize_steps
    f: vec4f,
    // show_grains, ...
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
    let t = params.a.z;
    let l = params.b.y;
    let c = params.b.z;
    let h = params.b.w;
    let ma1a = params.c.x;
    let ma2a = params.c.y;
    let ma3a = params.c.z;
    let ma4a = params.c.w;
    let ma1b = params.d.x;
    let ma2b = params.d.y;
    let ma3b = params.d.z;
    let ma4b = params.d.w;
    let use_ma_b = params.e.y == 1.0;
    let ma1 = select(ma1a, ma1b, use_ma_b);
    let ma2 = select(ma2a, ma2b, use_ma_b);
    let ma3 = select(ma3a, ma3b, use_ma_b);
    let ma4 = select(ma4a, ma4b, use_ma_b);
    let contrast = params.e.x;
    let show_masks = params.e.w == 1.0;
    let grain_size = params.f.x * 10.0;
    let swirl = params.f.y;
    let posterize = bool(params.f.z);
    let posterize_steps = params.f.w;
    let show_grains = bool(params.g.x);

    let p = correct_aspect(position);
    let d = length(p);

    let angle = atan2(p.y, p.x);
    let mod_angle = angle + sin(d * 3.0 + t) * swirl;
    let mod_p = vec2f(cos(mod_angle), sin(mod_angle)) * d;

    let q = vec2f(
        fbm(mod_p + vec2f(0.0) + t * 0.1),
        fbm(mod_p + vec2f(5.2, 1.3) + t * 0.1)
    );

    var r: vec2f;

    let grain_opacity = 1.0;
    let grain = select(
        0.0, 
        // grain_size * length(q),
        grain_size + (sin(t * 0.1) + cos(t * 0.2)) * (grain_size * 0.1), 
        show_grains
    );

    if (show_masks) {
        let mask1 = make_wrapped_mask(p, vec2f(ma1, ma2), 0.7, t * 0.75);
        let mask2 = make_wrapped_mask(p, vec2f(ma4, ma2), 0.5, t * 0.10);
        let mask3 = make_wrapped_mask(p, vec2f(ma3, ma4), 0.3, t * 0.33);
        let mask = clamp(mask1 + mask2 + mask3, 0.0, 1.0);
        let r_strength = mix(1.0, 5.0, mask);
        r = vec2f(
            fbm(p + r_strength * q + vec2f(1.7, 9.2)),
            // fbm(p + grain + r_strength * q + vec2f(1.7, 9.2)),
            fbm(p + r_strength * q + vec2f(8.3, 2.8))
        );
    } else {
        r = vec2f(
            fbm(p * q + vec2f(1.7, 9.2)),
            // fbm(p + grain * q + vec2f(1.7, 9.2)),
            fbm(p + q + vec2f(8.3, 2.8))
        );
    }

    let m = vec2f(
        fbm(mod_p + grain * r + vec2f(5.2, 1.3)),
        fbm(mod_p + 5.0 * r + vec2f(1.7, 9.2))
    );

    let f_a = pow(clamp(length(q), 0.0, 1.0), 3.0);
    var f_b = pow(clamp(length(r), 0.0, 1.0), 3.0);
    let f_c = pow(clamp(length(m), 0.0, 1.0), 3.0);

    if (show_grains) {
        // Remove the whitest specks
        let r_intensity = length(r);
        let cutoff = 0.9;
        let speck_mask = smoothstep(cutoff, cutoff - 0.05, r_intensity);
        f_b = f_b * speck_mask;
    }

    // Not exactly efficient since we're going to convert right back to oklch
    // but I'm not really sure how to blend lch hues properly yet
    let rgb_a = oklch_to_rgb(vec3f(l, c, h));
    let rgb_b = oklch_to_rgb(vec3f(l, c, fract(h + 1.0 / 2.0)));
    let rgb_c = oklch_to_rgb(vec3f(l, c, fract(h + 2.0 / 3.0)));
    var color = (rgb_a * f_a) + (rgb_b * f_b) + (rgb_c * f_c);

    color = rgb_to_oklch(color);
    color.x = pow(color.x, contrast);

    if (posterize) {
        color.x = floor(color.x * posterize_steps) / posterize_steps;
        color.y = max(0.1, floor(color.y * posterize_steps) / posterize_steps);
    }

    // let outline = step(0.2, abs(f_a - f_b)) + step(0.09, abs(f_b - f_c));
    // color *= 1.0 - clamp(outline, 0.0, 1.0);

    // color = film_grain(color, color.xz, 0.5);

    return vec4f(oklch_to_rgb(color), 1.0);
}

fn film_grain(color: vec3f, p: vec2f, intensity: f32) -> vec3f {
    let random = rand_sine_dot(p);
    return clamp(color + (random - 0.5) * intensity, vec3f(0.0), vec3f(1.0));
}

fn make_wrapped_mask(p: vec2f, center: vec2f, radius: f32, t: f32) -> f32 {
    let z = sin(p.x * 4.0 + t) * 0.5 + 1.5;
    let scale = 1.0 / z;
    let proj = p * scale;
    var min_dist = 1e6;

    for (var dx = -1; dx <= 1; dx++) {
        for (var dy = -1; dy <= 1; dy++) {
            let offset = vec2f(f32(dx) * 2.0, f32(dy) * 2.0); 
            let dist = distance(proj, center + offset);
            min_dist = min(min_dist, dist);
        }
    }

    // return step(min_dist, radius);
    return smoothstep(radius, 0.0, min_dist);
}

fn make_mask(p: vec2f, center: vec2f, radius: f32) -> f32 {
    return smoothstep(radius, 0.0, distance(p, center));
}

fn fbm(p: vec2f) -> f32 {
    var a = params.a.w;
    var f = params.b.x;

    let octaves = 5;
    let H = 1.0;
    let G = pow(3.0, -H);

    var t = 0.0;

    for (var i = 0; i < octaves; i++) {
        t += a * noise(p * f);
        f *= 2.0;
        a *= G;
    }

    return t;
}

fn rotate(v: vec2f, angle: f32) -> vec2f {
    let c = cos(angle);
    let s = sin(angle);
    return vec2f(c * v.x - s * v.y, s * v.x + c * v.y);
}

fn noise(p: vec2f) -> f32 {
    let hash_alg = params.e.z;

    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    
    var a: f32;
    var b: f32;
    var c: f32;
    var d: f32;
    
    if (hash_alg == 0.0) {
        a = rand_hash_dot3(i + vec2f(0.0, 0.0));
        b = rand_hash_dot3(i + vec2f(1.0, 0.0));
        c = rand_hash_dot3(i + vec2f(0.0, 1.0));
        d = rand_hash_dot3(i + vec2f(1.0, 1.0));
    } else if hash_alg == 1.0 {
        a = rand_sine_dot(i + vec2f(0.0, 0.0));
        b = rand_sine_dot(i + vec2f(1.0, 0.0));
        c = rand_sine_dot(i + vec2f(0.0, 1.0));
        d = rand_sine_dot(i + vec2f(1.0, 1.0));
    } else if hash_alg == 2.0 {
        a = rand_fract(i + vec2f(0.0, 0.0));
        b = rand_fract(i + vec2f(1.0, 0.0));
        c = rand_fract(i + vec2f(0.0, 1.0));
        d = rand_fract(i + vec2f(1.0, 1.0));
    } else if hash_alg == 3.0 {
        a = rand_bit_style(i + vec2f(0.0, 0.0));
        b = rand_bit_style(i + vec2f(1.0, 0.0));
        c = rand_bit_style(i + vec2f(0.0, 1.0));
        d = rand_bit_style(i + vec2f(1.0, 1.0));
    } else if hash_alg == 4.0 {
        a = rand_int_floor(i + vec2f(0.0, 0.0));
        b = rand_int_floor(i + vec2f(1.0, 0.0));
        c = rand_int_floor(i + vec2f(0.0, 1.0));
        d = rand_int_floor(i + vec2f(1.0, 1.0));
    }
    
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn rand_hash_dot3(p: vec2f) -> f32 {
    let p3 = fract(vec3f(p.xyx) * 0.13);
    let p4 = p3 + vec3f(7.0, 157.0, 113.0);
    return fract(dot(p4, vec3f(268.5453123, 143.2354234, 424.2424234)));
}

fn rand_sine_dot(p: vec2f) -> f32 {
    return fract(sin(dot(p, vec2f(12.9898, 78.233))) * 43758.5453);
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

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = params.a.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}

fn modulo(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}

fn mix_angle(a: f32, b: f32, t: f32) -> f32 {
    let delta = fract(b - a + 0.5) - 0.5;
    return fract(a + delta * t);
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