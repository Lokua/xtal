const DEFAULT_DISTANCE_ALG: f32 = 0.0;
const CONCENTRIC_WAVES_ALG: f32 = 1.0;
const MOIRE_ALG: f32 = 2.0;

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, bg_alpha, show_center
    a: vec4f,
    // show_corners, radius, strength, corner_radius
    b: vec4f,
    // corner_strength, scaling_power, offset, ring_strength
    c: vec4f,
    // angular_variation, alg, j, k
    d: vec4f,
    // auto_hue_shift, r, g, b
    e: vec4f,
    // threshold, mix, time, comp_shift
    f: vec4f,
    // init_x, init_y, color_bands, unused
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
    let w = params.a.x;
    let h = params.a.y;
    let bg_alpha = params.a.z;
    let show_center = params.a.w;
    let show_corners = params.b.x;
    let radius = params.b.y;
    var strength = params.b.z;
    let corner_radius = params.b.w;
    let corner_strength = params.c.x;
    let scaling_power = params.c.y;
    let offset = params.c.z;
    let ring_strength = params.c.w;
    let angular_variation = params.d.x;
    let alg = params.d.y;
    let j = params.d.z;
    let k = params.d.w;
    let auto_hue_shift = params.e.x;
    let r = params.e.y;
    let g = params.e.z;
    let b = params.e.w;
    let threshold = params.f.x;
    let mix_value = params.f.y;
    let time = params.f.z;
    let comp_shift = params.f.w;
    let init_x = params.g.x;
    let init_y = params.g.y;
    let color_bands = params.g.z;
    
    let aspect = w / h;
    var pos = position;
    pos.x *= aspect;

    var total_displacement = vec2f(init_x, init_y);
    var max_influence = 0.0;

    for (var i = 0u; i < 5u; i++) {
        let displacer_pos = get_displacer_position(i);
        var displacement = vec2f(0.0);
        if i == 0u && show_center == 1.0 {
            displacement = displace(
                pos, 
                displacer_pos, 
                radius, 
                strength
            );
        } else if i > 0u && show_corners == 1.0 {
            displacement = displace(
                pos, 
                displacer_pos, 
                corner_radius, 
                corner_strength
            );
        }
        total_displacement += displacement;
        max_influence = max(max_influence, length(displacement));
    }
    
    let angle = atan2(total_displacement.y, total_displacement.x);
    let disp_length = length(total_displacement);
    let rings = sin(disp_length * ring_strength);
    let angular_pattern = sin(angle * angular_variation);
    let threshold_pattern = step(threshold, rings * angular_pattern);
    let pattern = mix(rings * angular_pattern, threshold_pattern, mix_value);
    
    if max_influence < 0.01 {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }

    if auto_hue_shift == 1.0 {
        let hue_shift = sin((angle + time) * color_bands) + 2.0 * 0.5;
        let compliment_shift = (hue_shift + comp_shift) % 1.0;
        
        return vec4f(
            clamp(pattern * r * hue_shift, 0.0, 1.0),
            clamp(pattern * g * compliment_shift, 0.0, 1.0),
            clamp(pattern * b * (1.0 - hue_shift), 0.0, 1.0),
            bg_alpha
        );
    } else {
        return vec4f(
            pattern * r,
            pattern * g,
            pattern * b,
            bg_alpha
        ); 
    }
}

fn displace(
    point: vec2f, 
    displacer_pos: vec2f, 
    radius: f32, 
    strength: f32
) -> vec2f {
    let alg = params.d.y;
    let j = params.d.z;
    let k = params.d.w;
    let scaling_power = params.c.y;
    
    var d = 0.0;

    if alg == DEFAULT_DISTANCE_ALG {
        d = distance(displacer_pos, point);
    } else if alg == CONCENTRIC_WAVES_ALG {
        d = concentric_waves(displacer_pos, point, j);
    } else if alg == MOIRE_ALG {
        d = moire(displacer_pos, point, j) * k * 100.0;
    }

    let proximity = 1.0 - d / (radius * 2.0);
    let distance_factor = max(proximity, 0.0);
    let angle = atan2(point.y - displacer_pos.y, point.x - displacer_pos.x);
    let force = strength * pow(distance_factor, scaling_power);

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
    let offset = params.c.z;
    
    let max = 1.0 - offset;
    let min = -1.0 + offset;

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