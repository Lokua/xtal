const TAU: f32 = 6.283185307179586;

struct VertexInput {
    @location(0) position: vec2f,
}

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
}

struct Params {
    // w, h, beats, wave1_frequency
    a: vec4f,

    // wave1_angle, wave2_frequency, wave2_angle, wave1_amp
    b: vec4f,

    // animate_wave1_phase, wave1_phase, wave1_phase_auto, wave1_y_influence
    c: vec4f,

    // animate_wave2_phase, wave2_phase, wave2_phase_auto, wave2_y_influence
    d: vec4f,

    // wave2_amp, checkerboard, type_mix, curve_freq_x
    e: vec4f,

    // curve_freq_y, wave_distort, smoothing, white_hue
    f: vec4f,

    // white_saturation, unused, unused, unused
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
    let w = max(params.a.x, 1.0);
    let h = max(params.a.y, 1.0);
    let aspect = w / h;
    var p = position;
    p.x *= aspect;

    let wave1_frequency = params.a.w;
    let wave1_angle = params.b.x;
    let wave1_amp = params.b.w;
    let animate_wave1_phase = bool(params.c.x);
    let wave1_phase = select(params.c.y, params.c.z, animate_wave1_phase);
    let wave1_y_influence = params.c.w;

    let wave2_frequency = params.b.y;
    let wave2_angle = params.b.z;
    let wave2_amp = params.e.x;
    let animate_wave2_phase = bool(params.d.x);
    let wave2_phase = select(params.d.y, params.d.z, animate_wave2_phase);
    let wave2_y_influence = params.d.w;

    let checkerboard = bool(params.e.y);
    let type_mix = params.e.z;
    let curve_freq_x = params.e.w;
    let curve_freq_y = params.f.x;
    let wave_distort = params.f.y;
    let smoothing = params.f.z;
    let white_hue = params.f.w;
    let white_saturation = params.g.x;

    let wave1 = calculate_wave(
        p,
        wave1_frequency,
        wave1_angle,
        wave1_phase,
        wave1_y_influence,
        wave1_amp,
        type_mix,
        curve_freq_x,
        curve_freq_y,
        wave_distort,
    );

    let wave2 = calculate_wave(
        p,
        wave2_frequency,
        wave2_angle,
        wave2_phase,
        wave2_y_influence,
        wave2_amp,
        type_mix,
        curve_freq_x,
        curve_freq_y,
        wave_distort,
    );

    let half_smooth = smoothing * 0.5;
    let square1 = smoothstep(0.5 - half_smooth, 0.5 + half_smooth, wave1);
    let square2 = smoothstep(0.5 - half_smooth, 0.5 + half_smooth, wave2);

    let pattern_a = square1 * square2;
    let pattern_b = abs(square1 - square2) / (square1 + square2 + 0.1);
    let value = select(pattern_a, pattern_b, checkerboard);
    let tint = hsv_to_rgb(vec3f(white_hue, white_saturation, 1.0));
    let base = vec3f(value);
    let white_mask = smoothstep(0.7, 1.0, value);
    let color = mix(base, base * tint, white_mask);

    return vec4f(color, 1.0);
}

fn calculate_wave(
    p: vec2f,
    frequency: f32,
    angle: f32,
    phase: f32,
    y_influence: f32,
    amp: f32,
    type_mix: f32,
    curve_freq_x: f32,
    curve_freq_y: f32,
    wave_distort: f32,
) -> f32 {
    let rot = mat2x2<f32>(
        cos(angle * TAU),
        -sin(angle * TAU),
        sin(angle * TAU),
        cos(angle * TAU),
    );
    let rotated_p = rot * p;

    let curve = sin(rotated_p.y * y_influence * curve_freq_y * 10.0)
        * cos(rotated_p.x * y_influence * curve_freq_x * 10.0);

    let freq = 1.0 + frequency * 10.0;
    let wave_x = freq * (rotated_p.x + curve * wave_distort * amp) + phase;
    let base = fract(wave_x);
    let harmonic = fract(3.0 * wave_x + curve * wave_distort * 1.5);
    return mix(base, harmonic, type_mix);
}

fn hsv_to_rgb(hsv: vec3f) -> vec3f {
    let h = fract(hsv.x) * 6.0;
    let s = clamp(hsv.y, 0.0, 1.0);
    let v = max(hsv.z, 0.0);

    let i = floor(h);
    let f = h - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));

    if (i < 1.0) {
        return vec3f(v, t, p);
    }
    if (i < 2.0) {
        return vec3f(q, v, p);
    }
    if (i < 3.0) {
        return vec3f(p, v, t);
    }
    if (i < 4.0) {
        return vec3f(p, q, v);
    }
    if (i < 5.0) {
        return vec3f(t, p, v);
    }
    return vec3f(v, p, q);
}
