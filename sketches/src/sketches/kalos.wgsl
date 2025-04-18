struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    resolution: vec4f,

    show_center: f32,
    show_corners: f32,
    radius: f32,
    strength: f32,

    corner_radius: f32,
    corner_strength: f32,
    scaling_power: f32,
    auto_hue_shift: f32,

    r: f32,
    g: f32,
    b: f32,
    offset: f32,

    ring_strength: f32,
    angular_variation: f32,
    threshold: f32,
    mix: f32,

    alg: f32,
    j: f32,
    k: f32,
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
        var displacement = vec2f(0.0);
        if i == 0u && params.show_center == 1.0 {
            displacement = displace(
                pos, 
                displacer_pos, 
                params.radius, 
                params.strength
            );
        } else if i > 0u && params.show_corners == 1.0 {
            displacement = displace(
                pos, 
                displacer_pos, 
                params.corner_radius, 
                params.corner_strength
            );

        }
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

    if params.auto_hue_shift == 1.0 {
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
    } else {
        return vec4f(
            pattern * params.r,
            pattern * params.g ,
            pattern * params.b,
            1.0
        ); 
    }
}

fn displace(
    point: vec2f, 
    displacer_pos: vec2f, 
    radius: f32, 
    strength: f32
) -> vec2f {
    var d = 0.0;
    if params.alg == 0.0 {
        d = distance(displacer_pos, point);
    } else if params.alg == 1.0 {
        d = concentric_waves(displacer_pos, point, params.j * 20.0);
    } else if params.alg == 2.0 {
        d = moire(displacer_pos, point, params.j) * params.k * 100.0;
    }

    let proximity = 1.0 - d / (radius * 2.0);
    let distance_factor = max(proximity, 0.0);
    let angle = atan2(point.y - displacer_pos.y, point.x - displacer_pos.x);
    let force = strength * pow(distance_factor, params.scaling_power);

    return vec2f(cos(angle) * force, sin(angle) * force);
}

fn concentric_waves(p1: vec2f, p2: vec2f, frequency: f32) -> f32 {
    let distance = length(p2 - p1);
    return abs(sin(distance * frequency)) * exp(-distance * 0.01);
}

fn moire(p1: vec2f, p2: vec2f, scale: f32) -> f32 {
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    return cos(dx / scale) * cos(dy / scale);
}

fn get_displacer_position(index: u32) -> vec2f {
    let max = 1.0 - params.offset;
    let min = -1.0 + params.offset;

    switch(index) {
        // Center
        case 0u: { return vec2f(0.0); }
        // Top right
        case 1u: { return vec2f(max, min); }
        // Top left 
        case 2u: { return vec2f(min, max); }
        // Bottom right 
        case 3u: { return vec2f(max); }
        // Bottom left 
        case 4u: { return vec2f(min); }
        // Default case required
        default: { return vec2f(0.0); }
    }
}