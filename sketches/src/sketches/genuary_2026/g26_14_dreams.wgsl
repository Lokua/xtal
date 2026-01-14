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

    let pos = correct_aspect(position);
    let animated_pos = pos + vec2f(time * 0.02, time * 0.01);

    let voronoi_result = voronoi_boxes(
        animated_pos * scale,
        roundness
    );

    let cloud = fbm(animated_pos * scale, octaves);
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

    let bone_white = vec3f(0.96, 0.96, 0.92);
    let background = vec3f(0.05, 0.05, 0.08);
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
}

fn voronoi_boxes(p: vec2f, roundness: f32) -> VoronoiResult {
    let cell = floor(p);
    let local_p = fract(p);

    var min_dist = 1000.0;
    var second_min = 1000.0;
    var closest_value = 0.0;

    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let neighbor = vec2f(f32(x), f32(y));
            let cell_id = cell + neighbor;

            let rand_offset = vec2f(
                hash(cell_id),
                hash(cell_id + vec2f(127.1, 311.7))
            );
            let point = neighbor + rand_offset;

            let to_point = point - local_p;
            let box_dist = rounded_box_dist(to_point, roundness);

            if box_dist < min_dist {
                second_min = min_dist;
                min_dist = box_dist;
                closest_value = hash(cell_id + vec2f(43.21, 19.17));
            } else if box_dist < second_min {
                second_min = box_dist;
            }
        }
    }

    let edge_dist = smoothstep(0.0, 0.1, second_min - min_dist);

    var result: VoronoiResult;
    result.value = closest_value;
    result.edge_dist = 1.0 - edge_dist;
    return result;
}

fn rounded_box_dist(p: vec2f, roundness: f32) -> f32 {
    let corner_radius = mix(0.0, 0.3, roundness);
    let box_size = vec2f(0.3);
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
    var pos = p;

    for (var i = 0; i < octaves; i++) {
        value += amplitude * noise(pos * frequency);
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

