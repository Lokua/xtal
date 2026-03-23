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
    // bands_count, gradient_mix, ...
    f: vec4f,
    // parallax_mix, warp_amount, ...
    g: vec4f,
    // pool_strength, pool_size, pool_spin_rate, pool_angularity
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

    let parallax_mix = clamp(params.g.x, 0.0, 1.0);
    let warp_amount = max(0.0, params.g.y);
    let pool_strength = max(0.0, params.h.x);
    let pool_size = max(0.01, params.h.y);
    let pool_spin_rate = params.h.z;
    let pool_angularity = clamp(params.h.w, 0.0, 1.0);

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
        pool_strength,
        pool_size,
        pool_spin_rate,
        pool_angularity,
    );
    let far_signal = field_value(
        p * 0.72 + vec2f(sin(t * 0.08), cos(t * 0.06)) * 0.24,
        t * 0.63,
        flow_scale * 0.56 * detail,
        field_twist * 0.55,
        warp_amount * 0.78,
        far_drift,
        pool_strength * 0.8,
        pool_size * 1.2,
        pool_spin_rate * 0.9,
        pool_angularity,
    );
    let scalar = mix(near_signal, far_signal, parallax_mix);

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
    var color = sample_palette(palette, palette_t, palette_shift);

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
    pool_strength: f32,
    pool_size: f32,
    pool_spin_rate: f32,
    pool_angularity: f32,
) -> f32 {
    var q = p * scale;
    q += local_agent_motion(
        q,
        t,
        drift,
        twist,
        pool_strength,
        pool_size,
        pool_spin_rate,
        pool_angularity,
    );

    let wx = noise2(q * 0.63 + vec2f(t * 0.11, -t * 0.07));
    let wy = noise2(
        q * 0.67 + vec2f(-t * 0.09, t * 0.13) + vec2f(19.2, 4.7),
    );
    q += (vec2f(wx, wy) - 0.5) * warp * 1.45;

    let broad = noise2(q * 0.35 + vec2f(-2.4, 1.7));
    let base = fbm2(q * 0.55);
    let swirl = 0.5 + 0.5 * sin(
        (q.x * 0.42 - q.y * 0.37) +
            t * 0.09 +
            twist * (broad - 0.5) * 2.2,
    );
    let undulation = 0.5 + 0.5 * cos((q.x + q.y) * 0.28 + t * 0.06);

    let signal = base * 0.52 + broad * 0.18 + swirl * 0.2 + undulation * 0.15;
    return clamp(signal, 0.0, 1.0);
}

fn local_agent_motion(
    p: vec2f,
    t: f32,
    motion_bias: vec2f,
    twist: f32,
    pool_strength: f32,
    pool_size: f32,
    pool_spin_rate: f32,
    pool_angularity: f32,
) -> vec2f {
    let grid = p * 1.4;
    let cell = floor(grid);
    let f = fract(grid);
    let u = f * f * (3.0 - 2.0 * f);

    let m00 = cell_motion(cell + vec2f(0.0, 0.0), t, motion_bias, twist);
    let m10 = cell_motion(cell + vec2f(1.0, 0.0), t, motion_bias, twist);
    let m01 = cell_motion(cell + vec2f(0.0, 1.0), t, motion_bias, twist);
    let m11 = cell_motion(cell + vec2f(1.0, 1.0), t, motion_bias, twist);

    let mx0 = mix(m00, m10, u.x);
    let mx1 = mix(m01, m11, u.x);
    let local = mix(mx0, mx1, u.y);
    let pools = pool_force(
        p,
        t,
        motion_bias,
        pool_size,
        pool_spin_rate,
        pool_angularity,
    ) * pool_strength;
    return local + pools;
}

fn cell_motion(
    cell: vec2f,
    t: f32,
    motion_bias: vec2f,
    twist: f32,
) -> vec2f {
    let seed_a = hash21(cell + vec2f(17.1, 5.3));
    let seed_b = hash21(cell + vec2f(-9.7, 23.4));

    let ang_a = seed_a * TAU;
    let ang_b = seed_b * TAU;

    let dir_a = vec2f(cos(ang_a), sin(ang_a));
    let dir_b = vec2f(cos(ang_b), sin(ang_b));

    let phase_a = t * mix(0.15, 0.55, seed_a) + seed_b * TAU;
    let phase_b = t * mix(0.12, 0.48, seed_b) + seed_a * TAU;

    let amp_a = mix(0.08, 0.42, seed_a);
    let amp_b = mix(0.06, 0.34, seed_b);

    let local = dir_a * sin(phase_a) * amp_a + dir_b * cos(phase_b) * amp_b;
    let curlish = vec2f(local.y, -local.x) * (twist * 0.12);

    return local + curlish + motion_bias * 0.15;
}

fn pool_force(
    p: vec2f,
    t: f32,
    motion_bias: vec2f,
    pool_size: f32,
    pool_spin_rate: f32,
    pool_angularity: f32,
) -> vec2f {
    let c0 = vec2f(sin(t * 0.11), cos(t * 0.09)) * 1.10;
    let c1 = vec2f(cos(t * 0.07 + 1.7), sin(t * 0.13 - 0.3)) * 0.95 +
        vec2f(-0.42, 0.24);
    let c2 = vec2f(sin(t * 0.17 - 2.4), cos(t * 0.05 + 0.8)) * 1.25 +
        vec2f(0.36, -0.18);

    let f0 = pool_contrib(
        p,
        c0,
        t + 0.3,
        pool_size,
        pool_spin_rate,
        pool_angularity,
    );
    let f1 = pool_contrib(
        p,
        c1,
        t + 2.1,
        pool_size * 0.9,
        pool_spin_rate * -0.8,
        pool_angularity,
    );
    let f2 = pool_contrib(
        p,
        c2,
        t - 1.4,
        pool_size * 1.1,
        pool_spin_rate * 0.65,
        pool_angularity,
    );

    return f0 + f1 * 0.9 + f2 * 0.75 + motion_bias * 0.06;
}

fn pool_contrib(
    p: vec2f,
    center: vec2f,
    t: f32,
    pool_size: f32,
    pool_spin_rate: f32,
    pool_angularity: f32,
) -> vec2f {
    let delta = p - center;
    let d = max(length(delta), 0.0001);
    let radius = 0.22 + pool_size * 0.72;
    let falloff = exp(-pow(d / radius, 2.0));

    let radial = -delta / d;
    let tangent = vec2f(-radial.y, radial.x);
    let spin_phase = t * pool_spin_rate;
    let tangent_spun = rotate2(tangent, spin_phase);
    let base_dir = tangent_spun +
        radial * (0.35 + 0.25 * sin(t * 0.43));
    let dir = normalize(base_dir + vec2f(0.0001, 0.0));
    let snapped_dir = snapped_direction(dir, mix(4.0, 9.0, pool_angularity));
    let final_dir = mix(dir, snapped_dir, pool_angularity);

    let pulse = 0.75 + 0.25 * sin(t * 0.57 + d * 2.7);
    return final_dir * falloff * pulse;
}

fn snapped_direction(dir: vec2f, segments: f32) -> vec2f {
    let a = atan2(dir.y, dir.x);
    let step = TAU / max(1.0, segments);
    let snapped = floor(a / step + 0.5) * step;
    return vec2f(cos(snapped), sin(snapped));
}

fn rotate2(v: vec2f, a: f32) -> vec2f {
    let s = sin(a);
    let c = cos(a);
    return vec2f(c * v.x - s * v.y, s * v.x + c * v.y);
}

fn fbm2(p: vec2f) -> f32 {
    var sum = 0.0;
    var amp = 0.55;
    var freq = 1.0;

    for (var i = 0; i < 4; i++) {
        sum += noise2(p * freq) * amp;
        freq *= 1.85;
        amp *= 0.55;
    }

    return clamp(sum / 1.1083, 0.0, 1.0);
}

fn sample_palette(mode: i32, t_in: f32, shift: f32) -> vec3f {
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
        let a = gradient4(
            t,
            vec3f(0.045, 0.018, 0.020),
            vec3f(0.300, 0.090, 0.120),
            vec3f(0.865, 0.332, 0.130),
            vec3f(0.995, 0.845, 0.470),
        );
        let b = gradient4(
            t,
            vec3f(0.020, 0.010, 0.060),
            vec3f(0.360, 0.060, 0.360),
            vec3f(0.940, 0.320, 0.120),
            vec3f(1.000, 0.960, 0.700),
        );
        return mix(a, b, variant);
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
    let h = dot(p, vec2f(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = max(1.0, params.a.y);
    var p = position;
    p.x *= w / h;
    return p;
}
