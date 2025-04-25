const TAU: f32 = 6.283185307179586;

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, center_radius, center_strength
    a: vec4f,
    // center_scaling, neonize, corner_radius, corner_strength
    b: vec4f,
    // corner_scaling, corner_offset, invert, dots
    c: vec4f,
    // UNUSED, r, g, b
    d: vec4f,
    // UNUSED, ring_strength, UNUSED, UNUSED
    e: vec4f,
    // angular_variation, lerp, frequency, threshold
    f: vec4f,
    // mix, time, UNUSED, UNUSED
    g: vec4f,
    // UNUSED, UNUSED, UNUSED, UNUSED
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
    let neonize = params.b.y;
    let ring_strength = params.e.y;
    let angular_variation = params.f.x;
    let threshold = params.f.w;
    let mix_value = params.g.x;
    
    let t = params.g.y;
    let aspect = params.a.x / params.a.y;
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
    
    let angle = atan2(total_displacement.y, total_displacement.x) + 
        modulo(t * 0.125, TAU);
    let disp_length = length(total_displacement);
    
    let rings = sin(disp_length * ring_strength);
    let angular_pattern = sin(angle * angular_variation);
    
    
    let threshold_step = step(threshold, rings * angular_pattern);
    let pattern = mix(rings * angular_pattern, threshold_step, mix_value);

    if max_influence < 0.01 {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }

    let base_hue = (
        sin((angle + t * 0.5) * 2.0) + 
        sin((angle + t * 0.6666666) * 3.0)
    ) * neonize + 0.5;
    
    let dist_color = sin(disp_length * 3.0 + t) * 0.5 + 0.5;
    
    let r = pattern * params.d.y * (base_hue * 0.8 + dist_color * 0.2);
    let g = pattern * params.d.z * (1.0 - base_hue * 0.7 + dist_color * 0.3);
    let b = pattern * params.d.w * (sin(angle * 2.0) * 0.4 + dist_color * 0.6);

    return vec4f(r, g, b, 1.0);
}

fn displace(
    point: vec2f, 
    displacer_pos: vec2f, 
    displacer_params: vec4f
) -> vec2f {
    let radius = displacer_params.x;
    let strength = displacer_params.y;
    let scaling_power = displacer_params.z;
    let lerp_value = params.f.y;
    let frequency = params.f.z;

    let distance_from_displacer = mix(
        distance(displacer_pos, point),
        concentric_waves(displacer_pos, point, frequency),
        lerp_value
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
    let aspect = params.a.x / params.a.y;
    
    var pos: vec2f;
    let corner_offset = params.c.y;
    
    switch(index) {
        case 0u: { pos = vec2f(0.0); }
        case 1u: { pos = vec2f(1.0 - corner_offset, 1.0 - corner_offset); }
        case 2u: { pos = vec2f(1.0 - corner_offset, -1.0 + corner_offset); }
        case 3u: { pos = vec2f(-1.0 + corner_offset, -1.0 + corner_offset); }
        case 4u: { pos = vec2f(-1.0 + corner_offset, 1.0 - corner_offset); }
        default: { pos = vec2f(0.0); }
    }
    pos.x *= aspect;
    return pos;
}

fn get_displacer_params(index: u32) -> vec4f {
    let center_params = vec4f(params.a.z, params.a.w, params.b.x, 0.0);
    let corner_params = vec4f(params.b.z, params.b.w, params.c.x, params.c.y);
    
    switch(index) {
        case 0u: { return center_params; }
        case 1u: { return corner_params; }
        case 2u: { return corner_params; }
        case 3u: { return corner_params; }
        case 4u: { return corner_params; }
        default: { return vec4f(0.0); }
    }
}

fn modulo(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}