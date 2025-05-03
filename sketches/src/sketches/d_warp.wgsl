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
    // freq, h, s, UNUSED
    b: vec4f,
    // ma1a, ma2a, ma3a, ma4a
    c: vec4f,
    // ma1b, ma2b, ma3b, ma4b
    d: vec4f,
    // contrast, use_ma_b, ..
    e: vec4f,
    f: vec4f,
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
    let h = params.b.y;
    let s = params.b.z;
    // let h3 = params.b.w;
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

    let p = correct_aspect(position);

    let q = vec2f(
        fbm(p + vec2f(0.0) + t * 0.1),
        fbm(p + vec2f(5.2, 1.3) + t * 0.1)
    );

    let mask1 = make_wrapped_mask(p, vec2f(ma1, ma2), 0.7);
    let mask2 = make_wrapped_mask(p, vec2f(ma4, ma2), 0.5);
    let mask3 = make_wrapped_mask(p, vec2f(ma3, ma4), 0.3);
    
    let swirl_mask = clamp(mask1 + mask2 + mask3, 0.0, 1.0);
    let r_strength = mix(1.0, 5.0, swirl_mask);

    let r = vec2f(
        fbm(p + r_strength * q + vec2f(1.7, 9.2)),
        fbm(p + r_strength * q + vec2f(8.3, 2.8))
    );

    let m = vec2f(
        fbm(p + 5.0 * r + vec2f(5.2, 1.3)),
        fbm(p + 5.0 * r + vec2f(1.7, 9.2))
    );

    let f_q = pow(clamp(length(q), 0.0, 1.0), 3.0);
    let f_r = pow(clamp(length(r), 0.0, 1.0), 3.0);
    let f_m = pow(clamp(length(m), 0.0, 1.0), 3.0);

    let h2 = fract(h + 1.0 / 2.0);
    let h3 = fract(h + 2.0 / 3.0);     
    
    let rgb_q = lch_to_rgb(70.0, s * 100.0, h * 360.0);
    let rgb_r = lch_to_rgb(70.0, s * 100.0, h2 * 360.0);
    let rgb_m = lch_to_rgb(70.0, s * 100.0, h3 * 360.0);

    var color = (rgb_q * f_q) + (rgb_r * f_r) + (rgb_m * f_m);
    color = pow(color, vec3f(contrast));

    return vec4f(color, 1.0);
}

fn make_wrapped_mask(p: vec2f, center: vec2f, radius: f32) -> f32 {
    var min_dist = 1e6;

    for (var dx = -1; dx <= 1; dx++) {
        for (var dy = -1; dy <= 1; dy++) {
            // wrap in [-1, 1] space
            let offset = vec2f(f32(dx) * 2.0, f32(dy) * 2.0); 
            let dist = distance(p, center + offset);
            min_dist = min(min_dist, dist);
        }
    }

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

fn harmonic_noise(p: vec2f) -> f32 {
    let t = params.a.z;
    return sin(p.x * 4.0 + sin(p.y * 4.0 + t)) * 0.5 + 0.5;
}

fn rotate(v: vec2f, angle: f32) -> vec2f {
    let c = cos(angle);
    let s = sin(angle);
    return vec2f(c * v.x - s * v.y, s * v.x + c * v.y);
}

fn noise(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    
    let a = hash(i + vec2f(0.0, 0.0));
    let b = hash(i + vec2f(1.0, 0.0));
    let c = hash(i + vec2f(0.0, 1.0));
    let d = hash(i + vec2f(1.0, 1.0));
    
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn hash(p: vec2f) -> f32 {
    let p3 = fract(vec3f(p.xyx) * 0.13);
    let p4 = p3 + vec3f(7.0, 157.0, 113.0);
    return fract(dot(p4, vec3f(268.5453123, 143.2354234, 424.2424234)));
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

fn mix_hue(h1: f32, h2: f32, t: f32) -> f32 {
    let a1 = vec2f(cos(TAU * h1), sin(TAU * h1));
    let a2 = vec2f(cos(TAU * h2), sin(TAU * h2));
    let a = normalize(mix(a1, a2, t));
    return atan2(a.y, a.x) / TAU + select(1.0, 0.0, a.y < 0.0);
}

fn lch_to_rgb(l: f32, c: f32, h_deg: f32) -> vec3f {
    let h_rad = radians(h_deg);
    let a = cos(h_rad) * c;
    let b = sin(h_rad) * c;

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

    let X = x * 0.95047;
    let Y = y;
    let Z = z * 1.08883;

    let r_lin =  3.2406 * X - 1.5372 * Y - 0.4986 * Z;
    let g_lin = -0.9689 * X + 1.8758 * Y + 0.0415 * Z;
    let b_lin =  0.0557 * X - 0.2040 * Y + 1.0570 * Z;

    return vec3f(
        clamp(gamma_correct(r_lin), 0.0, 1.0),
        clamp(gamma_correct(g_lin), 0.0, 1.0),
        clamp(gamma_correct(b_lin), 0.0, 1.0)
    );
}

fn gamma_correct(c: f32) -> f32 {
    return select(12.92 * c, 1.055 * pow(c, 1.0 / 2.4) - 0.055, c > 0.0031308);
}