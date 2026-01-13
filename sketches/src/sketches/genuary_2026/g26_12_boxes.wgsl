struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, beats, depth
    a: vec4f,
    // boxify, cell_size, num_pinches, invert
    b: vec4f,
    // falloff, grain_amount, bridge_density, bg_noise_scale
    c: vec4f,
    // bg_noise_brightness, contrast, saturation, hue_shift
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
    let depth = params.a.w;
    let quant = params.b.x;
    let cell_size = params.b.y;
    let num_pinches = params.b.z;
    let invert = params.b.w;
    let falloff = params.c.x;
    let grain_amount = params.c.y;
    let bridge_density = params.c.z;
    let bg_noise_scale = params.c.w;
    let bg_noise_brightness = params.d.x;
    let contrast = params.d.y;
    let saturation = params.d.z;
    let hue_shift = params.d.w;

    let pos = correct_aspect(position);

    // Create multiple pinch points
    let pinch_pos = fract(pos * num_pinches);
    let pinch_center = pinch_pos - 0.5;

    // Create repeating layers going into "distance"
    let dist = length(pinch_center);
    let z = 1.0 / (dist + depth);

    // Normalize to keep zoom constant
    let base_z = 1.0 / (dist + 0.3);
    let normalized_z = z / base_z;

    // Quantize the scale to create stepped/blocky distortion
    let quant_steps = mix(100.0, 5.0, quant);
    let quantized_z = floor(normalized_z * quant_steps) / quant_steps;
    let final_z = mix(normalized_z, quantized_z, quant);

    let scale = final_z * 3.0;

    // Create box grid that scales with depth
    let p = pos * scale;
    let cell = fract(p);

    // Draw outer box edges
    let edge = 0.05;
    let box_x = step(edge, cell.x) * step(cell.x, 1.0 - edge);
    let box_y = step(edge, cell.y) * step(cell.y, 1.0 - edge);
    let outer_box = 1.0 - (box_x * box_y);

    // Draw inner box rotated 180 degrees
    // Rotate cell around center by flipping both axes
    let centered = cell - 0.5;
    let rotated = -centered + 0.5;

    // Make inner box smaller
    let inner_scale = 0.5;
    let inner_edge = edge / inner_scale;
    let inner_mask = step(cell_size, cell.x) *
        step(cell.x, 1.0 - cell_size) *
        step(cell_size, cell.y) *
        step(cell.y, 1.0 - cell_size);

    let inner_cell = (rotated - 0.5) / inner_scale + 0.5;
    let inner_x = step(inner_edge, inner_cell.x) *
        step(inner_cell.x, 1.0 - inner_edge);
    let inner_y = step(inner_edge, inner_cell.y) *
        step(inner_cell.y, 1.0 - inner_edge);
    let inner_box = (1.0 - (inner_x * inner_y)) * inner_mask;

    // Draw connecting lines between inner and outer corners
    let line_width = 0.02;

    // Distance to diagonal lines connecting corners
    let to_corner = abs(cell.x - cell.y);
    let to_anti_corner = abs(cell.x - (1.0 - cell.y));

    // Only draw lines in regions between inner and outer boxes
    let in_corner_region = (cell.x < cell_size || cell.x > 1.0 - cell_size) ||
        (cell.y < cell_size || cell.y > 1.0 - cell_size);

    let diagonal1 = step(to_corner, line_width) *
        f32(in_corner_region);
    let diagonal2 = step(to_anti_corner, line_width) *
        f32(in_corner_region);
    let connecting_lines = max(diagonal1, diagonal2);

    // Add bridge lines - thin connecting lines at grid boundaries
    let grid_id = floor(p);
    let bridge_threshold = bridge_density * 0.5;
    let show_bridge = hash(grid_id) < bridge_threshold;

    let on_h_edge = abs(cell.y - 0.5) < 0.02;
    let on_v_edge = abs(cell.x - 0.5) < 0.02;
    let bridges = (on_h_edge || on_v_edge) && show_bridge;

    let box = max(max(outer_box, inner_box), connecting_lines);
    let box_with_bridges = max(box, f32(bridges));

    // Toggle inversion
    let final_box = mix(box_with_bridges, 1.0 - box_with_bridges, invert);

    // Vary color and brightness using box distance (not radial)
    let box_dist = max(abs(pinch_center.x), abs(pinch_center.y));
    let min_brightness = 0.1;
    let depth_factor = clamp(box_dist * falloff, 0.0, 1.0);
    let brightness = mix(1.0, min_brightness, depth_factor);

    // Mix between coral (center) and copper (edges)
    // Lean more toward coral
    let coral = vec3f(1.0, 0.5, 0.5);
    let copper = vec3f(1.0, 0.5, 0.35);
    let color_mix = pow(depth_factor, 2.0);
    var base_color = mix(coral, copper, color_mix);

    // Apply hue shift
    let hsv = rgb_to_hsv(base_color);
    let shifted_hsv = vec3f(fract(hsv.x + hue_shift), hsv.y, hsv.z);
    base_color = hsv_to_rgb(shifted_hsv);

    // Add grain/noise texture
    let grain = hash(position * 10000.0) * grain_amount;
    let grainy_brightness = brightness + grain - grain_amount * 0.5;

    // Background noise for negative space
    let bg_noise = fbm(position * bg_noise_scale, 5);
    let bg_color = vec3f(bg_noise * bg_noise_brightness);

    // Mix between noisy background and colored boxes
    var color = mix(bg_color, base_color * grainy_brightness,
        final_box);

    // Apply contrast
    color = (color - 0.5) * contrast + 0.5;

    // Apply distance-based saturation (edge = more saturated)
    let dist_sat = max(abs(pinch_center.x), abs(pinch_center.y));
    let sat_factor = mix(0.0, saturation, dist_sat * 2.0);
    let luminance = dot(color, vec3f(0.299, 0.587, 0.114));
    color = mix(vec3f(luminance), color, sat_factor);

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

fn hash(p: vec2f) -> f32 {
    var p3 = fract(vec3f(p.xyx) * 0.13);
    p3 += dot(p3, p3.yzx + 3.333);
    return fract((p3.x + p3.y) * p3.z);
}

fn noise(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);

    let a = hash(i);
    let b = hash(i + vec2f(1.0, 0.0));
    let c = hash(i + vec2f(0.0, 1.0));
    let d = hash(i + vec2f(1.0, 1.0));

    let u = f * f * (3.0 - 2.0 * f);

    return mix(a, b, u.x) + (c - a) * u.y * (1.0 - u.x) +
        (d - b) * u.x * u.y;
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

fn rgb_to_hsv(c: vec3f) -> vec3f {
    let k = vec4f(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    let p = mix(
        vec4f(c.bg, k.wz),
        vec4f(c.gb, k.xy),
        step(c.b, c.g)
    );
    let q = mix(
        vec4f(p.xyw, c.r),
        vec4f(c.r, p.yzx),
        step(p.x, c.r)
    );
    let d = q.x - min(q.w, q.y);
    let e = 1.0e-10;
    return vec3f(
        abs(q.z + (q.w - q.y) / (6.0 * d + e)),
        d / (q.x + e),
        q.x
    );
}

fn hsv_to_rgb(c: vec3f) -> vec3f {
    let k = vec4f(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + k.xyz) * 6.0 - k.www);
    return c.z * mix(k.xxx, clamp(p - k.xxx, vec3f(0.0), vec3f(1.0)), c.y);
}

