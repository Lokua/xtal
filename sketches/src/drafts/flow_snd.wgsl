const TAU: f32 = 6.283185307179586;
const PALETTE_DUNE: i32 = 0;
const PALETTE_GLACIER: i32 = 1;
const PALETTE_FOREST: i32 = 2;
const PALETTE_EMBER: i32 = 3;
const PALETTE_NEON_CMY: i32 = 4;
const PALETTE_MONO_INK: i32 = 5;

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, beats, unused
    a: vec4f,
    // flow_scale, flow_speed, step_size, steps
    b: vec4f,
    // field_twist, advection_angle, advection_strength, line_density
    c: vec4f,
    // line_sharpness, palette, palette_shift, palette_contrast
    d: vec4f,
    // brightness, contrast, grain, vignette
    e: vec4f,
    // bands_count, gradient_mix, light_leak_amount, spectral_drive
    f: vec4f,
    // parallax_mix, warp_amount, ...
    g: vec4f,
    // unused
    h: vec4f,
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
    let beats = params.a.z;
    let p = correct_aspect(position);

    let flow_scale = max(0.001, params.b.x);
    let flow_speed = params.b.y;
    let step_size = max(0.0001, params.b.z);
    let requested_steps = clamp(i32(params.b.w), 1, 96);

    let field_twist = params.c.x;
    let advection_angle = params.c.y;
    let advection_strength = params.c.z;
    let line_density = max(0.0001, params.c.w);

    let line_sharpness = max(0.0001, params.d.x);
    let palette = i32(params.d.y);
    let palette_shift = params.d.z;
    let palette_contrast = max(0.0001, params.d.w);

    let brightness = max(0.0, params.e.x);
    let contrast = max(0.0, params.e.y);
    let grain = max(0.0, params.e.z);
    let vignette = max(0.0, params.e.w);

    let bands_count = max(2.0, params.f.x);
    let gradient_mix = clamp(params.f.y, 0.0, 1.0);
    let light_leak_amount = clamp(params.f.z, 0.0, 1.0);
    let spectral_drive = clamp(params.f.w, 0.0, 1.0);

    let parallax_mix = clamp(params.g.x, 0.0, 1.0);
    let warp_amount = max(0.0, params.g.y);

    let t = beats * flow_speed;
    let motion_axis = vec2f(cos(advection_angle), sin(advection_angle));
    let motion_axis_perp = vec2f(-motion_axis.y, motion_axis.x);
    let flex_a = sin(t * 0.24);
    let flex_b = cos(t * 0.19 + 1.3);
    let flex_c = sin(t * 0.41 - 0.7);
    let flex_d = cos(t * 0.33 + 0.5);
    let near_drift = (
        motion_axis * flex_a +
            motion_axis_perp * flex_b * 0.9 +
            vec2f(flex_c, flex_d) * 0.35
    ) * advection_strength;
    let far_drift = (
        motion_axis * cos(t * 0.17 - 0.9) * 0.7 +
            motion_axis_perp * sin(t * 0.22 + 0.4) +
            vec2f(sin(t * 0.29), cos(t * 0.27 + 1.1)) * 0.3
    ) * advection_strength;

    let detail = mix(0.75, 1.55, f32(requested_steps) / 96.0);
    let contour_density = max(
        0.001,
        line_density * mix(0.55, 1.45, step_size / 0.08),
    );

    let near_signal = field_value(
        p,
        t,
        flow_scale * detail,
        field_twist,
        warp_amount,
        near_drift,
    );
    var scalar = near_signal;
    if (parallax_mix > 0.002) {
        let far_signal = field_value_far(
            p * 0.72 + vec2f(sin(t * 0.08), cos(t * 0.06)) * 0.24,
            t * 0.63,
            flow_scale * 0.56 * detail,
            field_twist * 0.55,
            warp_amount * 0.78,
            far_drift,
        );
        scalar = mix(near_signal, far_signal, parallax_mix);
    }

    let contour = abs(fract(scalar * contour_density) - 0.5) * 2.0;
    let contour_lines = pow(1.0 - contour, line_sharpness);
    let mixed_signal = mix(scalar, contour_lines, 0.56);

    let line_emphasis = mix(0.88, 1.16, contour_lines);
    let shaped = pow(clamp(mixed_signal, 0.0, 1.0), max(0.001, contrast)) *
        brightness * line_emphasis;
    let banded = floor(shaped * bands_count) / max(1.0, bands_count - 1.0);
    let value = mix(banded, shaped, gradient_mix);

    let base_t = pow(clamp(value, 0.0, 1.0), palette_contrast);
    let palette_t = mix(
        base_t,
        smoothstep(0.0, 1.0, base_t),
        palette_shift * 0.6,
    );
    let contour_phase = fract(scalar * contour_density);
    let contour_center = 1.0 - abs(contour_phase - 0.5) * 2.0;
    let edge_energy = contour_center * contour_center;
    let contour_side = contour_phase - 0.5;
    let harmonic_shift = (
        contour_side * 0.34 +
            contour_center * contour_center * 0.22 -
            value * 0.11
    ) * spectral_drive;
    let driven_palette_t = fract(palette_t + harmonic_shift);
    var color = sample_palette(
        palette,
        driven_palette_t,
        palette_shift,
        edge_energy,
    );

    let leak_axis = clamp(position.y * 0.52 - position.x * 0.22 + 0.54, 0.0, 1.0);
    let leak_core = smoothstep(0.36, 1.0, leak_axis);
    let leak_horizon = band(position.y, 0.26, 0.42);
    let light_leak = light_leak_amount * (leak_core * 0.78 + leak_horizon * 0.28);
    let leak_color = mix(
        vec3f(1.000, 0.080, 0.480),
        vec3f(1.200, 0.480, 0.020),
        smoothstep(0.12, 0.96, leak_axis),
    );
    color = mix(color, leak_color, light_leak * 0.38);
    color += leak_color * light_leak * 0.18;

    let plasma_orange = vec3f(1.18, 0.30, 0.01);
    let plasma_pink = vec3f(1.05, 0.00, 0.58);
    let electric_blue = vec3f(0.05, 0.16, 0.95);
    let midnight = vec3f(0.002, 0.006, 0.060);
    let contour_center_2 = contour_center * contour_center;
    let ridge_core = contour_center_2 * contour_center_2 * contour_center;
    let inner_rim = band(contour_phase, 0.38, 0.085);
    let outer_rim = band(contour_phase, 0.62, 0.085);
    let valley = 1.0 - contour_center;
    let shadow_valley = valley * valley;
    let broad_glow = contour_center * (0.35 + 0.65 * contour_center);
    let harmonic_material = gradient7(
        contour_phase,
        midnight,
        vec3f(0.030, 0.018, 0.210),
        electric_blue,
        vec3f(0.760, 0.000, 0.760),
        plasma_pink,
        plasma_orange,
        vec3f(1.120, 0.780, 0.060),
    );
    let contour_material = mix(
        harmonic_material,
        mix(plasma_pink, plasma_orange, smoothstep(0.22, 0.82, value)),
        broad_glow * 0.55,
    );

    color = mix(color, contour_material, spectral_drive * 0.62);
    color = mix(color, midnight, shadow_valley * spectral_drive * 0.36);
    color += plasma_pink * inner_rim * spectral_drive * 0.24;
    color += plasma_orange * outer_rim * spectral_drive * 0.28;
    color += vec3f(1.0, 0.20, 0.02) * ridge_core * spectral_drive * 0.22;

    let n = hash21(position * 421.7 + beats * 0.13) - 0.5;
    color += n * grain;
    color = clamp(color, vec3f(0.0), vec3f(1.0));

    let radial = length(position);
    let vig = exp(-radial * radial * vignette);
    color *= vig;

    return vec4f(clamp(color, vec3f(0.0), vec3f(1.0)), 1.0);
}

fn field_value(
    p: vec2f,
    t: f32,
    scale: f32,
    twist: f32,
    warp: f32,
    drift: vec2f,
) -> f32 {
    var q = p * scale;
    let slow = vec2f(t * 0.035, -t * 0.027);
    let guide = vec2f(
        noise2(q * 0.18 + slow),
        noise2(q * 0.21 - slow + vec2f(7.3, 2.1)),
    ) - 0.5;
    let flow_a = sin(q.y * 0.34 + guide.x * 3.2 + t * 0.09);
    let flow_b = cos(q.x * 0.31 + guide.y * 3.0 - t * 0.08);
    q += guide * (1.10 + twist * 0.18);
    q += vec2f(flow_a, flow_b) * (0.18 + twist * 0.045);
    q += drift * 0.13;

    let wx = noise2(q * 0.58 + vec2f(t * 0.09, -t * 0.06));
    let wy = noise2(q * 0.61 + vec2f(-t * 0.07, t * 0.11) + vec2f(19.2, 4.7));
    q += (vec2f(wx, wy) - 0.5) * warp * 1.15;

    let broad = noise2(q * 0.32 + guide * 0.7 + vec2f(-2.4, 1.7));
    let base = fbm2(q * 0.50 + guide * 0.55);
    let ribbon = 1.0 - abs(sin(
        q.x * 0.48 -
            q.y * 0.39 +
            broad * 2.4 +
            guide.x * 1.7 +
            t * 0.055,
    ));
    let swirl = 0.5 + 0.5 * sin(
        (q.x * 0.42 - q.y * 0.37) +
            t * 0.09 +
            twist * (broad - 0.5) * 2.2,
    );
    let undulation = 0.5 + 0.5 * cos((q.x + q.y) * 0.28 + t * 0.06);

    let signal = base * 0.42 +
        broad * 0.16 +
        ribbon * 0.20 +
        swirl * 0.14 +
        undulation * 0.11;
    return clamp(signal, 0.0, 1.0);
}

fn field_value_far(
    p: vec2f,
    t: f32,
    scale: f32,
    twist: f32,
    warp: f32,
    drift: vec2f,
) -> f32 {
    var q = p * scale + drift * 0.15;
    let w = noise2(q * 0.58 + vec2f(t * 0.08, -t * 0.05));
    q += (w - 0.5) * warp * 0.95;

    let broad = noise2(q * 0.31 + vec2f(-2.4, 1.7));
    let base = fbm2_fast(q * 0.48);
    let swirl = 0.5 + 0.5 * sin(
        (q.x * 0.36 - q.y * 0.32) +
            t * 0.07 +
            twist * (broad - 0.5) * 1.8,
    );
    let signal = base * 0.56 + broad * 0.2 + swirl * 0.18;
    return clamp(signal, 0.0, 1.0);
}

fn fbm2(p: vec2f) -> f32 {
    let n0 = noise2(p);
    let n1 = noise2(p * 1.85);
    return clamp((n0 * 0.55 + n1 * 0.3025) / 0.8525, 0.0, 1.0);
}

fn fbm2_fast(p: vec2f) -> f32 {
    let n0 = noise2(p);
    let n1 = noise2(p * 1.85);
    return clamp((n0 * 0.55 + n1 * 0.3025) / 0.8525, 0.0, 1.0);
}

fn sample_palette(
    mode: i32,
    t_in: f32,
    shift: f32,
    edge_energy: f32,
) -> vec3f {
    let t0 = clamp(t_in, 0.0, 1.0);
    let s = clamp(shift, 0.0, 1.0);
    let warped = fract(t0 + s * 0.82);
    let folded = abs(fract(warped * 1.65 + s * 0.37) * 2.0 - 1.0);
    let t = mix(warped, folded, s * 0.85);
    let variant = smoothstep(0.12, 0.88, s);

    if (mode == PALETTE_GLACIER) {
        let a = gradient4(
            t,
            vec3f(0.020, 0.078, 0.125),
            vec3f(0.124, 0.344, 0.520),
            vec3f(0.490, 0.780, 0.832),
            vec3f(0.930, 0.985, 0.980),
        );
        let b = gradient4(
            1.0 - t,
            vec3f(0.010, 0.024, 0.082),
            vec3f(0.262, 0.172, 0.510),
            vec3f(0.240, 0.725, 0.845),
            vec3f(0.885, 0.978, 1.000),
        );
        return mix(a, b, variant);
    }

    if (mode == PALETTE_FOREST) {
        let a = gradient4(
            t,
            vec3f(0.024, 0.058, 0.036),
            vec3f(0.110, 0.286, 0.128),
            vec3f(0.430, 0.560, 0.170),
            vec3f(0.862, 0.782, 0.515),
        );
        let b = gradient4(
            t,
            vec3f(0.030, 0.030, 0.040),
            vec3f(0.192, 0.094, 0.170),
            vec3f(0.445, 0.345, 0.120),
            vec3f(0.935, 0.865, 0.700),
        );
        return mix(a, b, variant);
    }

    if (mode == PALETTE_EMBER) {
        let dawn = gradient7(
            t,
            vec3f(0.012, 0.012, 0.060),
            vec3f(0.050, 0.025, 0.165),
            vec3f(0.280, 0.060, 0.340),
            vec3f(0.760, 0.085, 0.430),
            vec3f(1.000, 0.255, 0.075),
            vec3f(1.100, 0.565, 0.110),
            vec3f(1.040, 0.850, 0.455),
        );
        let dusk = gradient7(
            t,
            vec3f(0.008, 0.014, 0.075),
            vec3f(0.035, 0.040, 0.205),
            vec3f(0.175, 0.055, 0.400),
            vec3f(0.575, 0.070, 0.535),
            vec3f(1.020, 0.160, 0.155),
            vec3f(1.120, 0.455, 0.060),
            vec3f(1.025, 0.740, 0.300),
        );
        let haze = gradient7(
            fract(t * 0.82 + 0.11),
            vec3f(0.020, 0.024, 0.105),
            vec3f(0.120, 0.075, 0.335),
            vec3f(0.440, 0.065, 0.525),
            vec3f(0.885, 0.105, 0.410),
            vec3f(1.100, 0.300, 0.045),
            vec3f(1.095, 0.625, 0.140),
            vec3f(0.995, 0.880, 0.520),
        );
        let sky = mix(dawn, dusk, variant);
        let organic_variation = smoothstep(0.22, 0.92, s) * 0.30;
        var color = mix(sky, haze, organic_variation);

        let midnight_band = band(t, 0.07, 0.12);
        let hot_pink_band = band(t, 0.24, 0.17);
        let violet_band = band(t, 0.43, 0.16);
        let neon_orange_band = band(t, 0.66, 0.16);
        let gold_band = band(t, 0.82, 0.13);

        color = mix(color, vec3f(0.004, 0.016, 0.105), midnight_band * 0.44);
        color = mix(color, vec3f(1.020, 0.030, 0.560), hot_pink_band * 0.48);
        color = mix(color, vec3f(0.430, 0.115, 0.810), violet_band * 0.34);
        color = mix(color, vec3f(1.120, 0.270, 0.020), neon_orange_band * 0.54);
        color = mix(color, vec3f(1.120, 0.750, 0.110), gold_band * 0.40);

        let glow = hot_pink_band * 0.11 + neon_orange_band * 0.16 + gold_band * 0.08;
        color += vec3f(1.000, 0.240, 0.080) * glow;
        color = mix(color, vec3f(1.100, 0.020, 0.570), edge_energy * 0.30);
        color += vec3f(1.000, 0.260, 0.040) * edge_energy * 0.12;

        return color;
    }

    if (mode == PALETTE_NEON_CMY) {
        let a = gradient4(
            t,
            vec3f(0.030, 0.035, 0.080),
            vec3f(0.000, 0.860, 0.970),
            vec3f(0.970, 0.080, 0.820),
            vec3f(0.980, 0.960, 0.180),
        );
        let b = gradient4(
            t,
            vec3f(0.030, 0.020, 0.020),
            vec3f(0.180, 0.950, 0.550),
            vec3f(0.700, 0.140, 0.980),
            vec3f(0.990, 0.480, 0.260),
        );
        return mix(a, b, variant);
    }

    if (mode == PALETTE_MONO_INK) {
        let a = gradient4(
            t,
            vec3f(0.030, 0.028, 0.040),
            vec3f(0.160, 0.150, 0.210),
            vec3f(0.470, 0.455, 0.560),
            vec3f(0.920, 0.905, 0.980),
        );
        let b = gradient4(
            t,
            vec3f(0.030, 0.040, 0.050),
            vec3f(0.190, 0.250, 0.230),
            vec3f(0.560, 0.580, 0.510),
            vec3f(0.970, 0.950, 0.880),
        );
        return mix(a, b, variant);
    }

    let a = gradient4(
        t,
        vec3f(0.090, 0.070, 0.045),
        vec3f(0.360, 0.260, 0.180),
        vec3f(0.760, 0.620, 0.430),
        vec3f(0.950, 0.900, 0.760),
    );
    let b = gradient4(
        t,
        vec3f(0.070, 0.050, 0.090),
        vec3f(0.360, 0.220, 0.500),
        vec3f(0.880, 0.520, 0.300),
        vec3f(0.980, 0.920, 0.720),
    );
    return mix(a, b, variant);
}

fn gradient4(t: f32, c0: vec3f, c1: vec3f, c2: vec3f, c3: vec3f) -> vec3f {
    if (t < 0.33333334) {
        return mix(c0, c1, t / 0.33333334);
    }
    if (t < 0.6666667) {
        return mix(c1, c2, (t - 0.33333334) / 0.33333334);
    }
    return mix(c2, c3, (t - 0.6666667) / 0.3333333);
}

fn gradient7(
    t: f32,
    c0: vec3f,
    c1: vec3f,
    c2: vec3f,
    c3: vec3f,
    c4: vec3f,
    c5: vec3f,
    c6: vec3f,
) -> vec3f {
    let x = clamp(t, 0.0, 1.0) * 6.0;
    if (x < 1.0) {
        return mix(c0, c1, x);
    }
    if (x < 2.0) {
        return mix(c1, c2, x - 1.0);
    }
    if (x < 3.0) {
        return mix(c2, c3, x - 2.0);
    }
    if (x < 4.0) {
        return mix(c3, c4, x - 3.0);
    }
    if (x < 5.0) {
        return mix(c4, c5, x - 4.0);
    }
    return mix(c5, c6, x - 5.0);
}

fn band(t: f32, center: f32, width: f32) -> f32 {
    let d = abs(fract(t - center + 0.5) - 0.5);
    return 1.0 - smoothstep(width * 0.35, width, d);
}

fn noise2(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);

    let a = hash21(i + vec2f(0.0, 0.0));
    let b = hash21(i + vec2f(1.0, 0.0));
    let c = hash21(i + vec2f(0.0, 1.0));
    let d = hash21(i + vec2f(1.0, 1.0));

    let u = f * f * (3.0 - 2.0 * f);
    return mix(a, b, u.x) +
        (c - a) * u.y * (1.0 - u.x) +
        (d - b) * u.x * u.y;
}

fn hash21(p: vec2f) -> f32 {
    var p3 = fract(vec3f(p.x, p.y, p.x) * 0.1031);
    p3 += dot(p3, p3.yzx + vec3f(33.33));
    return fract((p3.x + p3.y) * p3.z);
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = max(1.0, params.a.y);
    var p = position;
    p.x *= w / h;
    return p;
}
