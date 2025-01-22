const PI: f32 = 3.14159265359;
const TAU: f32 = 6.283185307179586;
const PHI: f32 = 1.61803398875;

// const NEAR_COLOR = vec3f(0.96, 0.93, 0.86); // warm beige
// const MID_COLOR = vec3f(0.82, 0.77, 0.68); // medium beige
// const FAR_COLOR = vec3f(0.62, 0.58, 0.52); // darker beige
const NEAR_COLOR = vec3f(0.98, 0.94, 0.85); // very light warm beige
const MID_COLOR = vec3f(0.75, 0.65, 0.55);  // distinctly darker beige
const FAR_COLOR = vec3f(0.45, 0.40, 0.35);  // much darker beige

const FULLSCREEN_TRIANGLE_VERTS = array<vec2f, 3>(
    vec2f(-1.0, -3.0),
    vec2f( 3.0,  1.0),
    vec2f(-1.0,  1.0)
);

struct ColoredPosition {
    pos: vec2f,
    color: vec3f,
};

struct DistortionResult {
    offset: vec2f,
    distance: f32,
};

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

    // point_size, col_freq, width, distortion
    c: vec4f,

    // clip_start, clip_grade, unused, row_freq
    d: vec4f,

    // stripe_step, stripe_mix, stripe_amp, stripe_freq
    e: vec4f,

    // unused, circle_radius, circle_phase, wave_amp
    f: vec4f,

    // center_count, center_spread, center_falloff, circle_force
    g: vec4f,

    // stripe_min, stripe_phase, harmonic_influence, stripe_max
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
    let col_freq = params.c.y;
    let width = params.c.z;
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
    
    let colored_pos = get_pos(
        t, 
        line_idx, 
        col_freq * (1.0 + 0.2 * sin(spiral_angle)), 
        width,
        spiral_factor
    );
    var adjusted_pos = colored_pos.pos;

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

    adjusted_pos = colored_pos.pos + 
        rotated_dir * 
        noise * 
        (1.0 + 0.3 * combined_harmonic);

    let w = params.resolution.x;
    let h = params.resolution.y;
    let aspect = w / h;
    adjusted_pos.x /= aspect;

    let modulation_factor = 1.0 + harmonic_influence * combined_harmonic;
    let dynamic_point_size = point_size * modulation_factor;

    var final_pos = adjusted_pos + 
        get_corner_offset(corner_index, dynamic_point_size);
    final_pos = clamp(final_pos, vec2f(-1.0), vec2f(1.0));

    var out: VertexOutput;
    out.pos = vec4f(final_pos, 0.0, 1.0);
    let alpha = 0.1 * modulation_factor;
    out.point_color = vec4f(colored_pos.color, alpha);
    out.uv = (final_pos.xy + 1.0) * 0.5;
    return out;
}

@fragment
fn fs_main(
    @builtin(position) pos: vec4f,
    @location(0) point_color: vec4f,
    @location(1) uv: vec2f,
) -> @location(0) vec4f {
    let invert = params.d.z;
    let near_black = vec4f(vec3f(0.05), 1.0);
    
    var color = select(near_black, point_color, point_color.a > 0.0);
    if invert == 1.0 {
        return vec4f(1.0 - color.r, 1.0 - color.g, 1.0 - color.b, color.a);
    }
    
    return color;
}

fn get_pos(
    t: f32, 
    line_idx: f32, 
    col_freq: f32,
    width: f32,
    spiral_factor: f32,
) -> ColoredPosition {
    let distortion = params.c.w;
    let time = params.d.y;
    let row_freq = params.d.w;
    let wave_amp = params.f.w;
    let center_count = params.g.x;  
    let center_spread = params.g.y; 
    let center_falloff = params.g.z;
    let circle_radius = params.f.y;
    let circle_phase = params.f.z;
    let circle_force = params.g.w;
    let n_lines = params.b.w;
    let clip_start = params.d.x;
    let clip_grade = params.d.y;
    
    // Get base position and apply wave distortion
    var pos = get_base_grid_pos(t, line_idx, n_lines, width);
    pos = apply_wave_distortion(pos, col_freq, row_freq, width, wave_amp);
    
    // Calculate and apply total distortion
    let distortion_result = calculate_total_distortion(
        pos, width, center_count, center_spread, 
        circle_radius, circle_force, center_falloff, distortion
    );
    
    let raw_offset = distortion_result.offset * width;
    pos += vec2f(
        soft_clip(raw_offset.x, clip_start, clip_grade),
        soft_clip(raw_offset.y, clip_start, clip_grade)
    );
    
    // Apply final modulation
    let angle = atan2(pos.y, pos.x);
    let final_r = length(pos);
    let r_modulated = apply_stripe_modulation(final_r, angle);
    pos *= r_modulated / max(final_r, 0.0001);
    
    // Calculate color based on distance to nearest center
    let normalized_dist = distortion_result.distance;
    let mid_range = smoothstep(0.1, 0.3, normalized_dist);
    let far_range = smoothstep(0.3, 0.5, normalized_dist);

    let near_to_mid = mix(NEAR_COLOR, MID_COLOR, mid_range);
    let final_color = mix(near_to_mid, FAR_COLOR, far_range);
    
    return ColoredPosition(pos, final_color);
}

fn get_base_grid_pos(t: f32, line_idx: f32, n_lines: f32, width: f32) -> vec2f {
    let x = (t * 2.0 - 1.0) * width;
    let y = ((line_idx / (n_lines - 1.0)) * 2.0 - 1.0) * width;
    return vec2f(x, y);
}

fn apply_wave_distortion(
    pos: vec2f, 
    col_freq: f32, 
    row_freq: f32, 
    width: f32, 
    wave_amp: f32
) -> vec2f {
    let x_freq = max(col_freq, 0.1);
    let y_freq = max(row_freq, 0.1);
    let x_wave = sin(pos.x * x_freq) * width * wave_amp;
    let y_wave = sin(pos.y * y_freq) * width * wave_amp;
    
    return vec2f(
        pos.x + y_wave,
        pos.y + x_wave
    );
}

fn calculate_distortion_force(
    normalized_dist: f32, 
    base_force: f32,
    radius: f32,
    dist: f32
) -> f32 {
    let ripple_freq = 3.0 * PI * normalized_dist;
    let ripple = sin(ripple_freq) * cos(ripple_freq * 0.5);
    
    let harmonic_dist = sin(normalized_dist * PI * 2.0) * 
        cos(normalized_dist * PI * 4.0) * 
        sin(normalized_dist * PI * 1.5);
    
    return base_force * (1.0 + harmonic_dist + ripple * 0.5) * 
        radius / (dist + radius * 0.1);
}

fn get_distortion_direction(delta: vec2f, normalized_dist: f32) -> vec2f {
    let rotation_angle = normalized_dist * PI * 2.0;
    let rotated_dir = vec2f(
        delta.x * cos(rotation_angle) - delta.y * sin(rotation_angle),
        delta.x * sin(rotation_angle) + delta.y * cos(rotation_angle)
    );
    
    return mix(
        normalize(delta),
        normalize(rotated_dir),
        sin(normalized_dist * PI) * 0.5 + 0.5
    );
}

fn calculate_total_distortion(
    pos: vec2f,
    width: f32,
    center_count: f32,
    center_spread: f32,
    circle_radius: f32,
    circle_force: f32,
    center_falloff: f32,
    distortion: f32
) -> DistortionResult {
    var total_distortion = vec2f(0.0);
    var min_dist = 99999.9;  // Track closest center
    
    for (var i = 0.0; i < center_count; i += 1.0) {
        let center_pos = get_grid_position(i, center_count, center_spread);
        let delta = pos - center_pos;
        let dist = length(delta);
        
        if (dist == 0.0) { continue; }
        
        min_dist = min(min_dist, dist);
        
        let radius = width * circle_radius;
        let normalized_dist = dist / (width * 2.0);
        let base_force = distortion * circle_force;
        
        let force = calculate_distortion_force(
            normalized_dist, 
            base_force, 
            radius, 
            dist
        );
        let falloff = exp(-pow(normalized_dist, 1.5) * center_falloff);
        let direction = get_distortion_direction(delta, normalized_dist);
        
        total_distortion += direction * force * falloff;
    }
    
    return DistortionResult(
        total_distortion,
        min_dist / (width * 2.0)
    );
}

fn get_grid_position(index: f32, count: f32, width: f32) -> vec2f {
    let grid_size = ceil(sqrt(count));
    let row = floor(index / grid_size);
    let col = index % grid_size;
    
    let cell_size = 2.0 / (grid_size);
    let offset = (grid_size - 1.0) * 0.5;
    
    let x = (col - offset) * cell_size * width;
    let y = (row - offset) * cell_size * width;
    
    return vec2f(x, y);
}

fn apply_stripe_modulation(radius: f32, pos_angle: f32) -> f32 {
    let stripe_step = params.e.x;
    let stripe_mix = params.e.y;
    let stripe_amp = params.e.z;
    let stripe_freq = params.e.w;
    let stripe_phase = params.h.y;
    let stripe_min = params.h.x;
    let stripe_max = params.h.w;
    let normalized_phase = stripe_phase * stripe_freq;
    let stripe_input = sin(stripe_freq * pos_angle + normalized_phase);
    let stripe1 = step(stripe_step, stripe_input); 
    let stripe2 = smoothstep(stripe_min, stripe_max, stripe_input); 
    let stripe = mix(stripe1, stripe2, stripe_mix);
    let modulation = 1.0 + stripe_amp * (2.0 * stripe - 1.0);
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

fn soft_clip(x: f32, clip_start: f32, softness: f32) -> f32 {
    if (abs(x) < clip_start) { return x; }
    let overshoot = abs(x) - clip_start;
    let soft_limit = softness * (1.0 - exp(-overshoot / softness));
    return sign(x) * (clip_start + soft_limit);
}

// fn soft_clip(x: f32, clip_start: f32, softness: f32) -> f32 {
//     if (abs(x) < clip_start) { return x; }
//     let overshoot = abs(x) - clip_start;
//     let normalized = overshoot / softness;
//     let soft_limit = softness * tanh(normalized);
//     return sign(x) * (clip_start + soft_limit);
// }

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
