struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    resolution: vec4f,

    // center, top-right, bottom-right, bottom-left, top-left
    // [radius, strength, scale, unused]
    d_0: vec4f,
    d_1: vec4f,
    d_2: vec4f,
    d_3: vec4f,
    d_4: vec4f,

    radius: f32,
    strength: f32,
    scaling_power: f32,
    r: f32,
    g: f32,
    b: f32,
    offset: f32,
    ring_strength: f32,
    ring_harmonics: f32,
    ring_harm_amt: f32,
    angular_variation: f32,
    lerp: f32,
    frequency: f32,
    threshold: f32,
    mix: f32,
    time: f32,
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
    let aspect = params.resolution.x / params.resolution.y;
    var pos = position;
    pos.x *= aspect;

    var total_displacement = vec2f(0.0);
    var max_influence = 0.0;

    for (var i = 0u; i < 5u; i++) {
        let displacer_pos = get_displacer_position(i);
        let displacer_params = get_displacer_params(i);
        let displacement = displace(pos, displacer_pos, displacer_params);
        total_displacement += displacement;
        max_influence = max(max_influence, length(displacement));
    }
    
    let angle = atan2(total_displacement.y, total_displacement.x);
    let disp_length = length(total_displacement);
    let rings = sin(disp_length * params.ring_strength);
    let angular_pattern = sin(angle * params.angular_variation);
    let threshold = step(params.threshold, rings * angular_pattern);
    let pattern = mix(rings * angular_pattern, threshold, params.mix);

    if max_influence < 0.01 {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }

    let hue_shift = 
        (
            sin((angle + params.time * 0.3) * 2.0) + 
            sin((angle + params.time * 0.62) * 3.0)
        ) * 0.25 + 0.5;

    return vec4f(
        pattern * params.r * hue_shift,
        pattern * params.g * (1.0 - hue_shift),
        pattern * params.b,
        1.0
    );
}

fn displace(
    point: vec2f, 
    displacer_pos: vec2f, 
    displacer_params: vec4f
) -> vec2f {
    let radius = displacer_params.x;
    let strength = displacer_params.y;
    let scaling_power = displacer_params.z;

    let distance_from_displacer = mix(
        distance(displacer_pos, point),
        concentric_waves(displacer_pos, point, params.frequency),
        params.lerp
    );

    if distance_from_displacer == 0.0 {
        return vec2f(0.0);
    }

    let proximity = 1.0 - distance_from_displacer / (radius * 2.0);
    let distance_factor = max(proximity, 0.0);
    let angle = atan2(point.y - displacer_pos.y, point.x - displacer_pos.x);
    let force = strength * pow(distance_factor, scaling_power);

    return vec2f(cos(angle) * force, sin(angle) * force);
}

fn concentric_waves(p1: vec2f, p2: vec2f, frequency: f32) -> f32 {
    let distance = length(p2 - p1);
    return abs(sin(distance * frequency)) * exp(-distance * 0.01);
}

fn wave_interference(p1: vec2f, p2: vec2f, frequency: f32) -> f32 {
    let source2 = p1 + vec2f(50.0, 50.0);
    let d1 = distance(p1, p2);
    let d2 = distance(source2, p2);
    return sin(d1 * frequency) + sin(d2 * frequency) * 2.0;
}

fn get_displacer_position(index: u32) -> vec2f {
    let aspect = params.resolution.x / params.resolution.y;
    var pos: vec2f;
    switch(index) {
        // Center stays at origin
        case 0u: { pos = vec2f(0.0); }
        // Top right: start at (1,1) and offset inward
        case 1u: { pos = vec2f(1.0 - params.d_1.w, 1.0 - params.d_1.w); }
        // Bottom right: start at (1,-1) and offset inward 
        case 2u: { pos = vec2f(1.0 - params.d_2.w, -1.0 + params.d_2.w); }
        // Bottom left: start at (-1,-1) and offset inward
        case 3u: { pos = vec2f(-1.0 + params.d_3.w, -1.0 + params.d_3.w); }
        // Top left: start at (-1,1) and offset inward
        case 4u: { pos = vec2f(-1.0 + params.d_4.w, 1.0 - params.d_4.w); }
        // Default case required
        default: { pos = vec2f(0.0); }
    }
    pos.x *= aspect;
    return pos;
}

fn get_displacer_params(index: u32) -> vec4f {
    switch(index) {
        // Center
        case 0u: { return params.d_0; }
        // Top right
        case 1u: { return params.d_1; }
        // Bottom right 
        case 2u: { return params.d_2; }
        // Bottom left 
        case 3u: { return params.d_3; }
        // Top left 
        case 4u: { return params.d_4; }
        // Default case required
        default: { return vec4f(0.0); }
    }
}

fn multi_lerp_3(values: array<f32, 3>, t: f32) -> f32 {
    let n_segments = 2u; // Hardcoded for 3 values
    let scaled_t = t * f32(n_segments);
    let index = u32(floor(scaled_t));
    let segment_t = scaled_t - f32(index);

    // Handle interpolation manually with explicit branches
    if index == 0u {
        return mix(values[0], values[1], segment_t);
    } 
    if index == 1u {
        return mix(values[1], values[2], segment_t);
    }

    // Handle edge case where t == 1.0
    return values[2];
}