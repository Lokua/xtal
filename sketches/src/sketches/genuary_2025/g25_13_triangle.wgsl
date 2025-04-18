const PRIMARY_LEVEL_COLOR: vec4f = vec4f(0.95, 0.95, 0.95, 1.0);
const SECOND_LEVEL_COLOR: vec4f = vec4f(0.1, 0.1, 0.6, 1.0);
const THIRD_LEVEL_COLOR: vec4f = vec4f(1.0, 0.4, 0.2, 1.0);
const FOURTH_LEVEL_COLOR: vec4f = vec4f(0.0, 0.3, 0.2, 1.0);
const BACKGROUND_COLOR: vec4f = vec4f(0.0, 0.0, 0.0, 1.0);

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
    // primary_iterations, second_iterations, third_iterations, fourth_iterations
    a: vec4f,
    // scale, y_offset, unused, unused          
    b: vec4f,
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
    let iterations = vec4i(params.a);  // All four iteration counts
    let scale = params.b.x;
    let y_offset = params.b.y;

    let aspect = params.resolution.x / params.resolution.y;
    var p = position;
    p.x *= aspect;
    
    p = p * scale;
    p.y = p.y + y_offset;

    // First try fourth level (deepest)
    var result = get_pattern_at_level(p, iterations, 3);
    if (result > 0.0) {
        return FOURTH_LEVEL_COLOR;
    }

    // Then third level
    result = get_pattern_at_level(p, iterations, 2);
    if (result > 0.0) {
        return THIRD_LEVEL_COLOR;
    }

    // Then second level
    result = get_pattern_at_level(p, iterations, 1);
    if (result > 0.0) {
        return SECOND_LEVEL_COLOR;
    }

    // Finally primary level
    result = get_pattern_at_level(p, iterations, 0);
    if (result > 0.0) {
        return PRIMARY_LEVEL_COLOR;
    }

    return BACKGROUND_COLOR;
}

fn get_pattern_at_level(p: vec2f, iterations: vec4i, level: i32) -> f32 {
    // if (iterations[level] <= 0) { return 0.0; }

    var current_p = p;
    let top = vec2f(0.0, 1.0);
    let left = vec2f(-0.866, -0.5);
    let right = vec2f(0.866, -0.5);

    // First check if point is in main triangle
    if (!in_triangle(current_p, top, left, right)) {
        return 0.0;
    }

    // Only do early return for iterations=0 on levels > 0
    if (level > 0 && iterations[level] <= 0) {
        return 0.0;
    }

    // For levels > 0, we need to be in the correct "hole" level
    for (var l = 0; l < level; l++) {
        var found_hole = false;
        for (var i = 0; i < iterations[l]; i++) {
            let mid_top = (top + right) * 0.5;
            let mid_left = (left + top) * 0.5;
            let mid_right = (right + left) * 0.5;

            if (in_triangle(current_p, mid_top, mid_left, mid_right)) {
                let center = (mid_top + mid_left + mid_right) / 3.0;
                // Rotate 180°
                current_p = center + (center - current_p); 
                // Scale
                current_p = (current_p - center) * 2.0 + center; 
                found_hole = true;
                break;
            }

            if (in_triangle(current_p, top, mid_left, mid_top)) {
                current_p = (current_p - top) * 2.0 + top;
            } else if (in_triangle(current_p, mid_left, left, mid_right)) {
                current_p = (current_p - left) * 2.0 + left;
            } else {
                current_p = (current_p - right) * 2.0 + right;
            }
        }
        if (!found_hole) { return 0.0; }
    }

    // Now check the actual pattern at this level
    return sierpinski(current_p, iterations[level]);
}

fn sierpinski(p: vec2f, iterations: i32) -> f32 {
    let top = vec2f(0.0, 1.0);
    let left = vec2f(-0.866, -0.5);
    let right = vec2f(0.866, -0.5);
    
    if (!in_triangle(p, top, left, right)) {
        return 0.0;
    }
    
    var current_p = p;
    for (var i = 0; i < iterations; i++) {
        let mid_top = (top + right) * 0.5;
        let mid_left = (left + top) * 0.5;
        let mid_right = (right + left) * 0.5;
        
        if (in_triangle(current_p, mid_top, mid_left, mid_right)) {
            return 0.0;
        }
        
        if (in_triangle(current_p, top, mid_left, mid_top)) {
            current_p = (current_p - top) * 2.0 + top;
        } else if (in_triangle(current_p, mid_left, left, mid_right)) {
            current_p = (current_p - left) * 2.0 + left;
        } else {
            current_p = (current_p - right) * 2.0 + right;
        }
    }
    
    return 1.0;
}

fn in_triangle(p: vec2f, a: vec2f, b: vec2f, c: vec2f) -> bool {
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
    
    return (u >= 0.0) && (v >= 0.0) && (u + v <= 1.0);
}