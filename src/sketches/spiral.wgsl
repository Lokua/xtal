const PI: f32 = 3.14159265359;
const TAU: f32 = 6.283185307179586;
const PHI: f32 = 1.61803398875;

const FULLSCREEN_TRIANGLE_VERTS = array<vec2f, 3>(
    vec2f(-1.0, -3.0),
    vec2f( 3.0,  1.0),
    vec2f(-1.0,  1.0)
);

struct VertexOutput {
    @builtin(position) pos: vec4f,
    @location(0) point_color: vec4f,
    @location(1) uv: vec2f,
}

struct Params {
    // w, h, ..unused
    resolution: vec4f,

    // ax, ay, bx, by
    a: vec4f,

    // points_per_segment, noise_scale, angle_variation, n_lines
    b: vec4f,

    // point_size, circle_r_min, circle_r_max, unused
    c: vec4f,

    // bg_brightness, time, invert, animate_angle_offset
    d: vec4f,

    // wave_amp, wave_freq, stripe_amp, stripe_freq
    e: vec4f,

    // animate_bg, steep_amp, steep_freq, steepness
    f: vec4f,

    // quant_amp, quant_freq, quant_phase, steep_phase
    g: vec4f,

    // wave_phase, stripe_phase, harmonic_influence, unused
    h: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@vertex
fn vs_main(@builtin(vertex_index) vidx: u32) -> VertexOutput {
        // Use first 3 vertices for background
    if (vidx < 3u) {
        // Full-screen triangle vertices
        var pos = FULLSCREEN_TRIANGLE_VERTS;
        var out: VertexOutput;
        out.pos = vec4f(pos[vidx], 0.0, 1.0);
        // Use uv for noise sampling if needed
        out.uv = (pos[vidx] + 1.0) * 0.5;
        // Background doesn’t need a specific point_color
        out.point_color = vec4f(0.0);
        return out;
    }

    // Adjust index for spiral vertices: subtract background vertex count
    let vert_index = vidx - 3u;

    let points_per_segment = params.b.x;
    let noise_scale = params.b.y;
    let angle_variation = params.b.z;
    let n_lines = params.b.w;
    let point_size = params.c.x;
    let circle_r_min = params.c.y;
    let circle_r_max = params.c.z;
    let harmonic_influence = params.h.z;

    let total_points_per_pass = u32(n_lines * points_per_segment);
    let point_index = (vert_index / 6u) % total_points_per_pass;
    let corner_index = vert_index % 6u;
    let line_idx = floor(f32(point_index) / points_per_segment);
    let point_in_line = f32(point_index) % points_per_segment;
    let t = point_in_line / (points_per_segment - 1.0);

    // Distribute lines evenly in vertical space
    let step = 1.8 / (n_lines - 1.0);
    let offset = (n_lines - 1.0) * 0.5;
    let y_pos = (line_idx - offset) * step;

    let base_freq = TAU;
    // Use line_idx directly for phase to ensure unique offset per line
    let phase_offset = line_idx * 0.1;
    
    let harmonic1 = sin(t * base_freq + phase_offset);
    let harmonic2 = sin(t * base_freq + phase_offset * 2.0) * 0.5;
    let harmonic3 = sin(t * base_freq + phase_offset * 3.0) * 0.3;
    
    let combined_harmonic = harmonic1 + harmonic2 + harmonic3;

    let noise_seed = point_index + 1u;
    let noise = random_normal(noise_seed, 1.0) * 
        noise_scale * 
        (1.0 + abs(combined_harmonic));

    let spiral_factor = line_idx / n_lines;
    let spiral_angle = t * TAU + spiral_factor * TAU;
    
    let circle_pos = get_circle_pos(
        t, 
        line_idx, 
        n_lines, 
        circle_r_min * (1.0 + 0.2 * sin(spiral_angle)), 
        circle_r_max,
        spiral_factor
    );

    let angle = random_normal(point_index, angle_variation) + 
        spiral_angle * 0.5 + 
        combined_harmonic * 0.3;

    let ref_a = vec2f(params.a.x, y_pos);
    let ref_b = vec2f(params.a.z, y_pos);
    let line_dir = normalize(ref_b - ref_a);
    let perp_dir = vec2f(-line_dir.y, line_dir.x);
    
    let rotated_dir = vec2f(
        perp_dir.x * cos(angle) - perp_dir.y * sin(angle),
        perp_dir.x * sin(angle) + perp_dir.y * cos(angle)
    );

    var adjusted_pos = circle_pos + 
        rotated_dir * 
        noise * 
        (1.0 + 0.3 * combined_harmonic);

    let w = params.resolution.x;
    let h = params.resolution.y;
    let aspect = w / h;
    adjusted_pos.x /= aspect;

    let modulation_factor = 1.0 + harmonic_influence * combined_harmonic;
    let dynamic_point_size = point_size * modulation_factor;

    let final_pos = adjusted_pos + 
        get_corner_offset(corner_index, dynamic_point_size);

    var out: VertexOutput;
    out.pos = vec4f(final_pos, 0.0, 1.0);
    
    let alpha = 0.1 * modulation_factor;
    out.point_color = vec4f(vec3f(0.0), alpha);
    out.uv = (final_pos.xy + 1.0) * 0.5;
    return out;
}

@fragment
fn fs_main(
    @builtin(position) pos: vec4f,
    @location(0) point_color: vec4f,
    @location(1) uv: vec2f,
) -> @location(0) vec4f {
    let bg_brightness = params.d.x;
    let time = params.d.y;
    let invert = params.d.z;
    let animate_bg = params.f.x;

    let pixel_pos = vec2u(floor(pos.xy));
    var time_seed = 0u;
    if animate_bg == 1.0 { 
        time_seed = u32(time * 1000.0);
    }
    let noise_seed = pixel_pos.x + pixel_pos.y * 1000u + time_seed;
    
    let fine_noise = rand_pcg(noise_seed);
    let very_fine_noise = rand_pcg(noise_seed * 31u + 17u);
    let combined_noise = mix(fine_noise, very_fine_noise, 0.5);
    
    let brightness = combined_noise * bg_brightness;
    let background_color = vec4f(vec3f(brightness), 1.0);

    let color = select(background_color, point_color, point_color.a > 0.0);
    if invert == 1.0 {
        return vec4f(1.0 - color.r, 1.0 - color.g, 1.0 - color.b, color.a);
    }
    
    return color;
}

fn get_circle_pos(
    t: f32, 
    line_idx: f32, 
    n_lines: f32, 
    min_r: f32, 
    max_r: f32,
    spiral_factor: f32,
) -> vec2f {
    let offset_mult = params.c.w;
    let time = params.d.y;
    let animate_angle_offset = params.d.w;

    let radius_factor = line_idx / n_lines;
    let actual_min = min(min_r, max_r);
    let actual_max = max(min_r, max_r);
    
    // Keep the radius interpolation direction-aware
    let invert_factor = select(radius_factor, 1.0 - radius_factor, min_r > max_r);
    let radius = mix(actual_min, actual_max, invert_factor);
    
    // Maintain spiral direction but adjust the phase
    let direction = select(1.0, -1.0, min_r > max_r);
    var angle_offset: f32;
    if animate_angle_offset == 1.0 {
        let time_factor = time * 10.0;
        angle_offset = direction * 
            pow(radius_factor, PHI) * TAU * offset_mult + 
            sin(time_factor + radius_factor * 5.0) * 0.5;

    } else {
        angle_offset = direction * pow(radius_factor, PHI) * TAU * offset_mult;
    }
    let pos_angle = t * TAU + angle_offset;

    var r_modulated = apply_wave_modulation(radius, pos_angle);
    // Next 3 are all variations on the same "slice" idea;
    r_modulated = apply_stripe_modulation(r_modulated, pos_angle);
    r_modulated = apply_steep_modulation(r_modulated, pos_angle);
    r_modulated = apply_quant_modulation(r_modulated, pos_angle);

    return vec2f(
        cos(pos_angle) * r_modulated,
        sin(pos_angle) * r_modulated
    );
}

fn apply_wave_modulation(radius: f32, pos_angle: f32) -> f32 {
    let wave_amp = params.e.x;
    let wave_freq = params.e.y;
    let wave_phase = params.h.x;
    let normalized_phase = wave_phase * wave_freq;
    let modulation = 1.0 + wave_amp * 
        sin(wave_freq * pos_angle + normalized_phase);
    return radius * modulation;
}

fn apply_stripe_modulation(radius: f32, pos_angle: f32) -> f32 {
    let stripe_amp = params.e.z;
    let stripe_freq = params.e.w;
    let stripe_phase = params.h.y;
    let normalized_phase = stripe_phase * stripe_freq;
    let stripe = step(0.0, sin(stripe_freq * pos_angle + normalized_phase)); 
    let modulation = 1.0 + stripe_amp * (2.0 * stripe - 1.0);
    return radius * modulation;
}

fn apply_steep_modulation(radius: f32, pos_angle: f32) -> f32 {
    let steep_amp = params.f.y;
    let steep_freq = params.f.z;
    let steepness = params.f.w; 
    let steep_phase = params.g.w;
    let normalized_phase = steep_phase * steep_freq;
    let base_signal = sin(steep_freq * pos_angle + normalized_phase);
    let slice_value = tanh(steepness * base_signal);
    let modulation = 1.0 + steep_amp * slice_value;
    return radius * modulation;
}

fn apply_quant_modulation(radius: f32, pos_angle: f32) -> f32 {
    let quant_amp = params.g.x;
    let quant_freq = params.g.y;
    let quant_phase = params.g.z;
    let tau_by_freq = TAU / quant_freq;
    let quantized_angle = floor(pos_angle / tau_by_freq) * tau_by_freq;
    let modulation = 1.0 + quant_amp * sin(quantized_angle + quant_phase);
    return radius * modulation;
}

fn get_corner_offset(index: u32, point_size: f32) -> vec2f {
    let s = point_size;
    switch (index) {
        case 0u: { return vec2f(-s, -s); }
        case 1u: { return vec2f(-s,  s); }
        case 2u: { return vec2f( s,  s); }
        case 3u: { return vec2f(-s, -s); }
        case 4u: { return vec2f( s,  s); }
        case 5u: { return vec2f( s, -s); }
        default: { return vec2f(0.0); }
    }
}

fn rand_pcg(seed: u32) -> f32 {
    var state = seed * 747796405u + 2891336453u;
    var word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    var result = (word >> 22u) ^ word;
    return f32(result) / 4294967295.0;
}

fn random_normal(seed: u32, std_dev: f32) -> f32 {
    let u1 = rand_pcg(seed);
    let u2 = rand_pcg(seed + 1u);
    let mag = sqrt(-2.0 * log(u1));
    let z0 = mag * cos(2.0 * PI * u2);
    return std_dev * z0;
}