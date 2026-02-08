struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    a: vec4f,
    b: vec4f,
    c: vec4f,
    d: vec4f,
    e: vec4f,
    f: vec4f,
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
    let time = params.a.z;
    let scale = params.b.x;
    let octaves = i32(params.b.y);
    let brightness = params.b.z;
    let contrast = params.b.w;
    let contour_levels = params.c.x;
    let contour_smoothness = params.c.y;
    let depth_strength = params.c.z;
    let roundness = params.c.w;
    let scroll_down = params.d.x > 0.5;
    let scroll_speed = params.d.y;
    let extrude_amount = params.d.z;
    let extrude_frequency = params.d.w;
    let gap_amount = params.e.x;
    let wave_enabled = params.e.y > 0.5;
    let wave_speed = params.e.z;
    let wave_scale = params.e.w;
    let wave_invert = params.f.x > 0.5;

    let bone_white = vec3f(0.96, 0.96, 0.92);
    let background = vec3f(0.05, 0.05, 0.08);
    let gap_background = vec3f(0.96, 0.96, 1.0);

    let pos = correct_aspect(position);
    let scroll_dir = select(-1.0, 1.0, scroll_down);
    let scroll_phase = scroll_dir * time * scroll_speed;

    var local_scale = scale;
    let wave_active = wave_enabled && abs(wave_scale) > 0.0001;
    if wave_active {
        let movement = vec2f(
            sin(time * wave_speed * 0.2) * 3.0,
            select(1.0, -1.0, wave_invert) * time * wave_speed * 0.5
        );
        let noise_pos = (pos + vec2f(0.0, scroll_phase)) * 0.8 + movement;

        let n = fbm(noise_pos, 3);
        let wave_raw = clamp((n - 0.35) * 3.0, 0.0, 1.0);
        let shaped = pow(wave_raw, 3.0);

        let n2 = fbm(noise_pos * 3.7 + vec2f(9.4, 7.1), 2);
        let modulated = shaped * (0.5 + n2 * 0.5);
        let capped = clamp(modulated, 0.0, 0.6);
        let wave_strength = 8.0;

        local_scale = mix(
            scale,
            scale * (1.0 + wave_scale * wave_strength),
            capped
        );
    }

    // Decouple advection from animated scale to keep a stable fall speed/direction.
    // This multiplier sets visual speed in pattern space without tying it to scale.
    let pattern_scroll = vec2f(0.0, scroll_phase * 12.0);
    let pattern_pos = pos * local_scale + pattern_scroll;

    let voronoi_result = voronoi_boxes(
        pattern_pos,
        roundness,
        time,
        extrude_amount,
        extrude_frequency,
        gap_amount
    );

    if voronoi_result.is_gap {
        return vec4f(gap_background, 1.0);
    }

    let cloud = fbm(pattern_pos, octaves);
    let shaped = mix(
        cloud,
        voronoi_result.value,
        0.7
    );

    let quantized = posterize(
        shaped,
        contour_levels,
        contour_smoothness
    );

    let depth_edges = voronoi_result.edge_dist * depth_strength;

    var final_value = clamp(quantized - depth_edges, 0.0, 1.0);
    final_value = pow(final_value, contrast);
    final_value = final_value * brightness;

    let color = mix(background, bone_white, final_value);

    return vec4f(color, 1.0);
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = params.a.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}

struct VoronoiResult {
    value: f32,
    edge_dist: f32,
    is_gap: bool,
}

fn voronoi_boxes(
    p: vec2f,
    roundness: f32,
    time: f32,
    extrude_amount: f32,
    extrude_frequency: f32,
    gap_amount: f32
) -> VoronoiResult {
    let cell = floor(p);
    let local_p = fract(p);
    let animate_extrude = extrude_amount > 0.0001;
    let extrude_phase = time * extrude_frequency;
    let corner_radius = mix(0.0, 0.3, roundness);

    var min_dist = 1000.0;
    var second_min = 1000.0;
    var closest_value = 0.0;
    var closest_cell_id = vec2f(0.0);

    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let neighbor = vec2f(f32(x), f32(y));
            let cell_id = cell + neighbor;

            // Derive a stable 2D jitter from a single hash to cut hash calls.
            let jitter_seed = hash(cell_id + vec2f(127.1, 311.7));
            let rand_offset = fract(vec2f(
                jitter_seed * 13.37 + 0.17,
                jitter_seed * 91.73 + 0.83
            ));
            let point = neighbor + rand_offset;

            let cell_hash = hash(cell_id + vec2f(43.21, 19.17));
            var size_mod = 1.0;
            if animate_extrude {
                let pulse_offset = cell_hash * 6.28318;
                let pulse = sin(extrude_phase + pulse_offset);
                let pulse_01 = pulse * 0.5 + 0.5;
                size_mod = 1.0 - (pulse_01 * extrude_amount);
            }

            let to_point = point - local_p;
            let box_dist = rounded_box_dist(
                to_point,
                corner_radius,
                size_mod
            );

            if box_dist < min_dist {
                second_min = min_dist;
                min_dist = box_dist;
                closest_value = cell_hash;
                closest_cell_id = cell_id;
            } else if box_dist < second_min {
                second_min = box_dist;
            }
        }
    }

    let gap_hash = hash(closest_cell_id + vec2f(77.7, 55.5));
    let is_gap = gap_hash < gap_amount;

    let edge_dist = smoothstep(0.0, 0.1, second_min - min_dist);

    var result: VoronoiResult;
    result.value = closest_value;
    result.edge_dist = 1.0 - edge_dist;
    result.is_gap = is_gap;
    return result;
}

fn rounded_box_dist(
    p: vec2f,
    corner_radius: f32,
    size_mod: f32
) -> f32 {
    let box_size = vec2f(0.3) * size_mod;
    let q = abs(p) - box_size + corner_radius;
    let dist = length(max(q, vec2f(0.0))) +
        min(max(q.x, q.y), 0.0) - corner_radius;
    return dist;
}

fn posterize(value: f32, levels: f32, smoothness: f32) -> f32 {
    let stepped = floor(value * levels) / levels;
    return mix(stepped, value, smoothness);
}

fn create_depth_map(p: vec2f, base_value: f32, strength: f32) -> f32 {
    let offset = 0.01;
    let right = fbm(p + vec2f(offset, 0.0), 3);
    let up = fbm(p + vec2f(0.0, offset), 3);
    let left = fbm(p - vec2f(offset, 0.0), 3);
    let down = fbm(p - vec2f(0.0, offset), 3);

    let dx = abs(right - left);
    let dy = abs(up - down);
    let gradient = sqrt(dx * dx + dy * dy);

    return gradient * strength * 0.1;
}

fn fbm(p: vec2f, octaves: i32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;

    for (var i = 0; i < octaves; i++) {
        value += amplitude * noise(p * frequency);
        frequency *= 2.0;
        amplitude *= 0.5;
    }

    return value;
}

fn noise(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);

    let a = hash(i);
    let b = hash(i + vec2f(1.0, 0.0));
    let c = hash(i + vec2f(0.0, 1.0));
    let d = hash(i + vec2f(1.0, 1.0));

    let u = f * f * (3.0 - 2.0 * f);

    return mix(a, b, u.x) +
        (c - a) * u.y * (1.0 - u.x) +
        (d - b) * u.x * u.y;
}

fn hash(p: vec2f) -> f32 {
    var p3 = fract(vec3f(p.xyx) * 0.13);
    p3 += dot(p3, p3.yzx + 3.333);
    return fract((p3.x + p3.y) * p3.z);
}
