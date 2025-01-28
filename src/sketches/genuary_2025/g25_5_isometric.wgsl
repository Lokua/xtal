const PI: f32 = 3.14159265359;

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, ..unused
    resolution: vec4f,
    // size, height, n_triangles, spacing
    a: vec4f,
};

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
    let size = mix(0.1, 0.4, params.a.x);
    let height = mix(0.1, 0.3, params.a.y);
    let n_triangles = (params.a.z + 1.0) * 10.0;
    let spacing_param = params.a.w + 1.0;

    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    p = p * 2.0;

    // Define colors with more contrast
    let base_color = vec3f(0.92);  // Slightly darker off-white
    let highlight_color = vec3f(1.0);
    
    let spacing = size * spacing_param;
    // Adjust vertical spacing to match the full triangle height
    let vertical_spacing = size * 1.5; // This matches the full triangle height (from point to base)
    let n_rows = i32(floor(n_triangles / 2.0));
    
    var total_shape = 0.0;
    var total_edges = 0.0;

    let triangle_count = i32(floor(n_triangles));
    let start_x = -spacing * (f32(triangle_count) / 2.0) + spacing * 0.5;
    let start_y = vertical_spacing * f32(n_rows);

    for (var row = 0; row < n_rows * 2; row++) {
        let row_offset = f32(row) * vertical_spacing;
        let is_odd_row = row % 2 == 1;
        let x_shift = select(0.0, spacing * 0.5, is_odd_row);
        
        for (var i = 0; i < triangle_count; i++) {
            let x_pos = start_x + spacing * f32(i) + x_shift;
            let y_pos = start_y - row_offset;
            let offset = vec2f(x_pos, y_pos);
            
            let shape_result = draw_isometric_triangle(
                p - offset, 
                size, 
                height, 
                is_odd_row
            );
            
            // Keep the original blending behavior
            total_shape = max(total_shape, shape_result.visibility);
            total_edges = max(total_edges, shape_result.edges);
        }
    }

    let shaded = mix(
        base_color * 0.95,
        highlight_color,
        total_shape
    );

    let color = shaded * (1.0 - total_edges);
    return vec4f(color, 1.0);
}

struct ShapeResult {
    visibility: f32,
    edges: f32,
};

fn draw_isometric_triangle(p: vec2f, size: f32, height: f32, flip: bool) -> ShapeResult {
    var top_left = vec2f(-size, -size * 0.5);
    var top_right = vec2f(size, -size * 0.5);
    var top_front = vec2f(0.0, size);

    if (flip) {
        // Rotate 180 degrees and adjust position for interlocking
        top_left = vec2f(-size, size * 0.5);
        top_right = vec2f(size, size * 0.5);
        top_front = vec2f(0.0, -size);
    }

    let bottom_left = top_left + vec2f(0.0, height) * select(1.0, -1.0, flip);
    let bottom_right = top_right + vec2f(0.0, height) * select(1.0, -1.0, flip);
    let bottom_front = top_front + vec2f(0.0, height) * select(1.0, -1.0, flip);

    let line_width = 0.01;

    // Draw edges
    let edge1 = smoothstep(line_width, 0.0, line_distance(p, top_left, top_right));
    let edge2 = smoothstep(line_width, 0.0, line_distance(p, top_right, top_front));
    let edge3 = smoothstep(line_width, 0.0, line_distance(p, top_front, top_left));
    let edge4 = smoothstep(line_width, 0.0, line_distance(p, top_left, bottom_left));
    let edge5 = smoothstep(line_width, 0.0, line_distance(p, top_right, bottom_right));
    let edge6 = smoothstep(line_width, 0.0, line_distance(p, top_front, bottom_front));
    let edge7 = smoothstep(line_width, 0.0, line_distance(p, bottom_front, bottom_right));
    let edge8 = smoothstep(line_width, 0.0, line_distance(p, bottom_front, bottom_left));

    let edges = min(1.0, edge1 + edge2 + edge3 + edge4 + edge5 + edge6 + edge7 + edge8);

    let front_face = is_in_triangle(p, top_front, top_right, top_left);
    let right_face_1 = is_in_triangle(p, top_front, bottom_right, top_right);
    let right_face_2 = is_in_triangle(p, top_front, bottom_front, bottom_right);
    let right_face = max(right_face_1, right_face_2);
    let left_face_1 = is_in_triangle(p, top_front, bottom_left, top_left);
    let left_face_2 = is_in_triangle(p, top_front, bottom_front, bottom_left);
    let left_face = max(left_face_1, left_face_2);

    let visibility = select(
        front_face * 0.7 + right_face * 0.5 + left_face * 0.3,
        front_face * 0.3 + right_face * 0.5 + left_face * 0.7,
        flip
    );

    return ShapeResult(visibility, edges);
}

fn line_distance(p: vec2f, a: vec2f, b: vec2f) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let den = dot(ba, ba);
    if den == 0.0 {
        return length(pa);
    }
    let h = clamp(dot(pa, ba) / den, 0.0, 1.0);
    let dist = length(pa - ba * h);
    return max(0.0, dist);
}

fn is_in_triangle(p: vec2f, a: vec2f, b: vec2f, c: vec2f) -> f32 {
    let v0 = c - a;
    let v1 = b - a;
    let v2 = p - a;

    let dot00 = dot(v0, v0);
    let dot01 = dot(v0, v1);
    let dot02 = dot(v0, v2);
    let dot11 = dot(v1, v1);
    let dot12 = dot(v1, v2);

    let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);
    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;

    return f32(u >= 0.0 && v >= 0.0 && u + v <= 1.0);
}