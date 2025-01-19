const PI: f32 = 3.14159265359;
const TAU: f32 = 6.283185307179586;

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    resolution: vec4f,
    
    // wave1_frequency, wave1_angle, wave2_frequency, wave2_angle
    a: vec4f,
    
    // wave1_phase, wave2_phase, wave1_y_influence, wave2_y_influence
    b: vec4f,
    
    // unused, type_mix, unused, checkerboard
    c: vec4f,

    // curve_freq_x, curve_freq_y, wave_distort, smoothing
    d: vec4f,

    // unused
    e: vec4f,
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
    let w = params.resolution.x;
    let h = params.resolution.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;

    let wave1_frequency = params.a.x;
    let wave1_angle = params.a.y;
    let wave1_phase = params.b.x;
    let wave1_y_influence = params.b.z;
    let wave1_amp = params.e.x;

    let wave2_frequency = params.a.z;
    let wave2_angle = params.a.w;
    let wave2_phase = params.b.y;
    let wave2_y_influence = params.b.w;
    let wave2_amp = params.e.y;

    let smoothing = params.d.w;
    let checkerboard = params.c.w;

    let wave1 = calculate_wave(
        p,
        wave1_frequency,
        wave1_angle, 
        wave1_phase,
        wave1_y_influence,
        wave1_amp
   );

    let wave2 = calculate_wave(
        p,
        wave2_frequency,
        wave2_angle,
        wave2_phase,
        wave2_y_influence,
        wave2_amp
   );

    let half_smooth = smoothing * 0.5;
    let square1 = smoothstep(0.5 - half_smooth, 0.5 + half_smooth, wave1);
    let square2 = smoothstep(0.5 - half_smooth, 0.5 + half_smooth, wave2);

    let pattern_a = square1 * square2;
    let pattern_b = abs(square1 - square2) / (square1 + square2 + 0.1);
    var value = select(pattern_a, pattern_b, checkerboard == 1.0);
    
    return vec4f(value);
}

fn calculate_wave(
   p: vec2f,
   frequency: f32,
   angle: f32,
   phase: f32,
   y_influence: f32,
   amp: f32
) -> f32 {
    let curve_freq_x = params.d.x;
    let curve_freq_y = params.d.y;
    let wave_distort = params.d.z;
    let type_mix = params.c.y;

    let rot = mat2x2f(
        cos(angle * TAU), -sin(angle * TAU),
        sin(angle * TAU), cos(angle * TAU)
    );
    let rotated_p = rot * p;

    let curve = 
        sin(rotated_p.y * y_influence * curve_freq_y * 10.0) * 
        cos(rotated_p.x * y_influence * curve_freq_x * 10.0);

    let freq = 1.0 + frequency * 10.0;
    let wave_x = freq * (rotated_p.x + curve * wave_distort * amp) + phase;
    let base = fract(wave_x);
    let harmonic = fract(3.0 * wave_x + curve * wave_distort * 1.5);
    return mix(base, harmonic, type_mix);
}
