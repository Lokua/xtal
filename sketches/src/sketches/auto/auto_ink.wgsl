const TAU: f32 = 6.283185307;
const RATE_SCALE: f32 = 0.25;
const BOX_CORNER_ROUNDNESS: f32 = 0.08;
const MAX_STEPS: i32 = 34;
const MAX_DIST: f32 = 12.0;
const SURF_DIST: f32 = 0.0024;
const MAX_CUBES: i32 = 9;

struct VertexInput {
    @location(0) position: vec2f,
}

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
}

struct Params {
    // w, h, beats, _
    a: vec4f,
    // box_count, spread, rotation_rate, blend_k
    b: vec4f,
    // stroke_ink_strength, contour_strength,
    // stroke_flow_strength, background_drop_strength
    c: vec4f,
    // stroke_drop_strength, box_size_offset,
    // moving_box_ratio, moving_box_range
    d: vec4f,
    // moving_rate, elongate_y_three_boxes,
    // elongate_x_three_boxes, elongate_z_three_boxes
    e: vec4f,
    // background_drop_density, domain_warp_amount,
    // domain_warp_scale, zoom
    f: vec4f,
    g: vec4f,
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
fn fs_main(
    @builtin(position) frag_coord: vec4f,
    @location(0) position: vec2f,
) -> @location(0) vec4f {
    let w = params.a.x;
    let h = params.a.y;
    let beats = params.a.z;

    let box_count = params.b.x;
    let spread = params.b.y;
    let rotation_rate = params.b.z;
    let blend_k = params.b.w;

    let stroke_ink_strength = params.c.x;
    let contour_strength = params.c.y;
    let stroke_flow_strength = params.c.z;
    let background_drop_strength = params.c.w;
    let stroke_drop_strength = params.d.x;
    let box_size_offset = params.d.y;
    let moving_box_ratio = params.d.z;
    let moving_box_range = params.d.w;
    let moving_rate = params.e.x;
    let elongate_y_three_boxes = params.e.y;
    let elongate_x_three_boxes = params.e.z;
    let elongate_z_three_boxes = params.e.w;
    let background_drop_density = params.f.x;
    let domain_warp_amount = params.f.y;
    let domain_warp_scale = params.f.z;
    let zoom = max(params.f.w, 0.1);

    let aspect = w / h;
    var uv = position * 0.5 / zoom;
    uv.x *= aspect;

    let cam = vec3f(0.0, 1.0, -3.5);
    let cam_target = vec3f(0.0, 0.16, 0.0);
    let fwd = normalize(cam_target - cam);
    let right = normalize(cross(fwd, vec3f(0.0, 1.0, 0.0)));
    let up = cross(right, fwd);
    let rd = normalize(fwd + uv.x * right + uv.y * up);
    let ro = cam;

    let spin = beats * rotation_rate * TAU * RATE_SCALE;
    let move_phase = beats * moving_rate * TAU * RATE_SCALE;
    let march = ray_march(
        ro,
        rd,
        spin,
        move_phase,
        box_count,
        spread,
        blend_k,
        domain_warp_amount,
        domain_warp_scale,
        box_size_offset,
        moving_box_ratio,
        moving_box_range,
        elongate_y_three_boxes,
        elongate_x_three_boxes,
        elongate_z_three_boxes,
    );
    let hit_dist = march.x;
    let min_dist = march.y;
    let is_hit = hit_dist < MAX_DIST && min_dist < 0.03;
    let hit_pos = ro + rd * hit_dist;

    let paper = vec3f(0.985, 0.975, 0.955);
    let ink = vec3f(0.04, 0.045, 0.05);
    var coverage = 0.0;
    var outside = 1.0;

    if is_hit {
        outside = 0.0;
        let n = calc_normal(
            hit_pos,
            spin,
            move_phase,
            box_count,
            spread,
            blend_k,
            domain_warp_amount,
            domain_warp_scale,
            box_size_offset,
            moving_box_ratio,
            moving_box_range,
            elongate_y_three_boxes,
            elongate_x_three_boxes,
            elongate_z_three_boxes,
        );
        let p_obj = to_object_space(hit_pos, spin);

        let edge_metric = 1.0 - abs(dot(n, -rd));
        let edge_noise = value_noise_2d(
            p_obj.xz * 8.0 + vec2f(4.3, 1.8),
        );
        let contour_threshold = mix(
            0.56 + (edge_noise - 0.5) * 0.03,
            0.26 + (edge_noise - 0.5) * 0.22,
            stroke_flow_strength,
        );
        let contour = step(contour_threshold, edge_metric);

        let edge_band = step(
            mix(0.40, 0.16, stroke_flow_strength),
            edge_metric,
        ) * (1.0 - contour);
        let swash_noise = value_noise_2d(
            p_obj.xz * mix(2.0, 8.5, stroke_flow_strength)
                + p_obj.yx * mix(0.8, 3.0, stroke_flow_strength)
                + vec2f(1.4, -2.2),
        );
        let swash_sparse = step(
            mix(0.97, 0.48, stroke_flow_strength),
            swash_noise,
        );
        let brush_swash = swash_sparse * edge_band;

        var stroke_mask = contour * contour_strength;
        stroke_mask = max(
            stroke_mask,
            brush_swash * stroke_ink_strength * 0.85,
        );
        stroke_mask = clamp(stroke_mask, 0.0, 1.0);

        let stroke_drops = stroke_drop_field(
            p_obj,
            stroke_drop_strength,
        );

        coverage += stroke_mask;
        coverage += stroke_drops
            * stroke_drop_strength
            * (0.35 + 0.65 * stroke_mask);
    }

    let near_outer = 1.0 - step(0.050, min_dist);
    let near_inner = 1.0 - step(0.018, min_dist);
    let contour_ring = max(near_outer - near_inner, 0.0);
    coverage += contour_ring * contour_strength * 0.06;

    let bg_drops = background_drop_field(
        frag_coord.xy,
        background_drop_density,
    );
    coverage += bg_drops * background_drop_strength * outside;

    coverage = clamp(coverage, 0.0, 1.0);

    let color = mix(paper, ink, coverage);
    return vec4f(color, 1.0);
}

fn ray_march(
    ro: vec3f,
    rd: vec3f,
    spin: f32,
    move_phase: f32,
    box_count: f32,
    spread: f32,
    blend_k: f32,
    domain_warp_amount: f32,
    domain_warp_scale: f32,
    box_size_offset: f32,
    moving_box_ratio: f32,
    moving_box_range: f32,
    elongate_y_three_boxes: f32,
    elongate_x_three_boxes: f32,
    elongate_z_three_boxes: f32,
) -> vec2f {
    var t = 0.0;
    var min_dist = 9e8;

    for (var step_i = 0; step_i < MAX_STEPS; step_i++) {
        let p = ro + rd * t;
        let dist = scene_sdf(
            p,
            spin,
            move_phase,
            box_count,
            spread,
            blend_k,
            domain_warp_amount,
            domain_warp_scale,
            box_size_offset,
            moving_box_ratio,
            moving_box_range,
            elongate_y_three_boxes,
            elongate_x_three_boxes,
            elongate_z_three_boxes,
        );
        min_dist = min(min_dist, abs(dist));
        t += dist * 0.88;

        if abs(dist) < SURF_DIST || t > MAX_DIST {
            break;
        }
    }

    return vec2f(t, min_dist);
}

fn calc_normal(
    p: vec3f,
    spin: f32,
    move_phase: f32,
    box_count: f32,
    spread: f32,
    blend_k: f32,
    domain_warp_amount: f32,
    domain_warp_scale: f32,
    box_size_offset: f32,
    moving_box_ratio: f32,
    moving_box_range: f32,
    elongate_y_three_boxes: f32,
    elongate_x_three_boxes: f32,
    elongate_z_three_boxes: f32,
) -> vec3f {
    let e = 0.002;
    let d = scene_sdf(
        p,
        spin,
        move_phase,
        box_count,
        spread,
        blend_k,
        domain_warp_amount,
        domain_warp_scale,
        box_size_offset,
        moving_box_ratio,
        moving_box_range,
        elongate_y_three_boxes,
        elongate_x_three_boxes,
        elongate_z_three_boxes,
    );
    let dx = scene_sdf(
        p - vec3f(e, 0.0, 0.0),
        spin,
        move_phase,
        box_count,
        spread,
        blend_k,
        domain_warp_amount,
        domain_warp_scale,
        box_size_offset,
        moving_box_ratio,
        moving_box_range,
        elongate_y_three_boxes,
        elongate_x_three_boxes,
        elongate_z_three_boxes,
    );
    let dy = scene_sdf(
        p - vec3f(0.0, e, 0.0),
        spin,
        move_phase,
        box_count,
        spread,
        blend_k,
        domain_warp_amount,
        domain_warp_scale,
        box_size_offset,
        moving_box_ratio,
        moving_box_range,
        elongate_y_three_boxes,
        elongate_x_three_boxes,
        elongate_z_three_boxes,
    );
    let dz = scene_sdf(
        p - vec3f(0.0, 0.0, e),
        spin,
        move_phase,
        box_count,
        spread,
        blend_k,
        domain_warp_amount,
        domain_warp_scale,
        box_size_offset,
        moving_box_ratio,
        moving_box_range,
        elongate_y_three_boxes,
        elongate_x_three_boxes,
        elongate_z_three_boxes,
    );
    return normalize(d - vec3f(dx, dy, dz));
}

fn scene_sdf(
    p_world: vec3f,
    spin: f32,
    move_phase: f32,
    box_count: f32,
    spread: f32,
    blend_k: f32,
    domain_warp_amount: f32,
    domain_warp_scale: f32,
    box_size_offset: f32,
    moving_box_ratio: f32,
    moving_box_range: f32,
    elongate_y_three_boxes: f32,
    elongate_x_three_boxes: f32,
    elongate_z_three_boxes: f32,
) -> f32 {
    let p_raw = to_object_space(p_world, spin);
    let p = apply_domain_warp(
        p_raw,
        domain_warp_amount,
        domain_warp_scale,
    );

    let count = clamp(round(box_count), 1.0, f32(MAX_CUBES));
    let k = max(blend_k, 0.0);
    let motion_ratio = clamp(moving_box_ratio, 0.0, 1.0);
    let motion_range = clamp(moving_box_range, 0.0, 1.5);
    let size_offset = clamp(box_size_offset, 0.0, 0.8);
    let elongate_y = clamp(elongate_y_three_boxes, 0.0, 1.0);
    let elongate_x = clamp(elongate_x_three_boxes, 0.0, 1.0);
    let elongate_z = clamp(elongate_z_three_boxes, 0.0, 1.0);
    let moving_count = i32(round(motion_ratio * count));

    var dist = 9e8;
    for (var i = 0; i < MAX_CUBES; i++) {
        if f32(i) >= count {
            break;
        }

        let fi = f32(i);
        var moving = 0.0;
        if motion_rank(i) < moving_count {
            moving = 1.0;
        }
        let move_amp = 0.32 * motion_range;
        let mov = vec3f(
            move_amp * sin(move_phase * 1.0 + fi * 1.7),
            move_amp * 0.78 * cos(move_phase * 1.4 + fi * 2.4),
            move_amp * sin(move_phase * 0.8 + fi * 0.9),
        ) * moving;
        let center = base_center(i) * spread + mov;

        let size_jitter = vec3f(
            hash11(fi * 3.11 + 0.31) * 2.0 - 1.0,
            hash11(fi * 4.57 + 1.73) * 2.0 - 1.0,
            hash11(fi * 6.91 + 2.29) * 2.0 - 1.0,
        );
        let size_scale = max(
            vec3f(0.22),
            vec3f(1.0) + size_jitter * size_offset * 1.4,
        );
        let stretch_x = 1.0 + elongate_x * 3.6 * elongated_box_mask_x(i);
        let stretch_y = 1.0 + elongate_y * 3.6 * elongated_box_mask_y(i);
        let stretch_z = 1.0 + elongate_z * 3.6 * elongated_box_mask_z(i);
        let box_size = base_size(i) * size_scale * vec3f(
            stretch_x,
            stretch_y,
            stretch_z,
        );

        let box_dist = sd_round_box(
            p - center,
            box_size,
            BOX_CORNER_ROUNDNESS,
        );

        if i == 0 {
            dist = box_dist;
        } else if k <= 0.0001 {
            dist = min(dist, box_dist);
        } else {
            dist = smooth_min(dist, box_dist, k);
        }
    }

    return dist;
}

fn sd_round_box(p: vec3f, b: vec3f, r: f32) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3f(0.0)))
        + min(max(q.x, max(q.y, q.z)), 0.0) - r;
}

fn base_center(index: i32) -> vec3f {
    switch index {
        case 0: { return vec3f(-0.56, 0.18, -0.20); }
        case 1: { return vec3f(-0.30, -0.08, 0.40); }
        case 2: { return vec3f(0.06, 0.22, -0.48); }
        case 3: { return vec3f(0.34, 0.06, 0.12); }
        case 4: { return vec3f(0.56, -0.17, 0.34); }
        case 5: { return vec3f(-0.02, -0.28, 0.02); }
        case 6: { return vec3f(-0.44, 0.05, -0.56); }
        case 7: { return vec3f(0.22, 0.30, 0.56); }
        default: { return vec3f(0.50, -0.22, -0.30); }
    }
}

fn base_size(index: i32) -> vec3f {
    switch index {
        case 0: { return vec3f(0.30, 0.19, 0.25); }
        case 1: { return vec3f(0.24, 0.32, 0.20); }
        case 2: { return vec3f(0.26, 0.21, 0.33); }
        case 3: { return vec3f(0.21, 0.26, 0.21); }
        case 4: { return vec3f(0.28, 0.20, 0.25); }
        case 5: { return vec3f(0.22, 0.29, 0.22); }
        case 6: { return vec3f(0.31, 0.24, 0.27); }
        case 7: { return vec3f(0.20, 0.34, 0.19); }
        default: { return vec3f(0.27, 0.22, 0.31); }
    }
}

fn smooth_min(a: f32, b: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}

fn background_drop_field(frag: vec2f, density: f32) -> f32 {
    let grid = frag * 0.020;
    let cell = floor(grid);
    let local = fract(grid);
    let pick_threshold = mix(0.995, 0.35, density);
    var out_drop = 0.0;

    for (var yi = 0; yi < 2; yi++) {
        for (var xi = 0; xi < 2; xi++) {
            let offset = vec2f(f32(xi), f32(yi));
            let cid = cell + offset;
            let pick = hash21(cid + vec2f(7.31, 3.19));
            let center = vec2f(
                hash21(cid + vec2f(2.1, 1.7)),
                hash21(cid + vec2f(9.2, 5.3)),
            );
            let radius = 0.06 + 0.17 * hash21(cid + vec2f(4.6, 8.1));
            let d = length((local - offset) - center);
            let blob = 1.0 - step(radius, d);
            out_drop = max(
                out_drop,
                blob * step(pick_threshold, pick),
            );
        }
    }

    return out_drop;
}

fn stroke_drop_field(p_obj: vec3f, drop_strength: f32) -> f32 {
    let grid = p_obj.xz * 6.2 + p_obj.yx * 1.8;
    let cell = floor(grid);
    let local = fract(grid);
    let strength = clamp(drop_strength, 0.0, 1.0);
    var out_drop = 0.0;

    for (var yi = 0; yi < 2; yi++) {
        for (var xi = 0; xi < 2; xi++) {
            let offset = vec2f(f32(xi), f32(yi));
            let cid = cell + offset;
            let pick = hash21(cid + vec2f(5.2, 1.1));
            let center = vec2f(
                hash21(cid + vec2f(1.7, 8.2)),
                hash21(cid + vec2f(3.4, 2.5)),
            );
            let radius = (0.03 + 0.07 * hash21(cid + vec2f(9.5, 6.3)))
                * mix(0.65, 2.2, strength);
            let d = length((local - offset) - center);
            let blob = 1.0 - step(radius, d);
            out_drop = max(
                out_drop,
                blob * step(mix(0.92, 0.55, strength), pick),
            );
        }
    }

    return out_drop;
}

fn apply_domain_warp(
    p: vec3f,
    warp_amount: f32,
    warp_scale: f32,
) -> vec3f {
    let amt = clamp(warp_amount, 0.0, 1.0) * 0.10;
    let freq = max(warp_scale, 0.001);

    let wx = value_noise_2d(p.xz * freq + vec2f(1.7, -3.2));
    let wy = value_noise_2d(p.yx * freq * 0.9 + vec2f(6.1, 2.4));
    let wz = value_noise_2d(p.zy * freq * 1.1 + vec2f(-4.3, 5.6));
    let warp = (vec3f(wx, wy, wz) - 0.5) * 2.0 * amt;

    return p + warp;
}

fn to_object_space(p_world: vec3f, spin: f32) -> vec3f {
    var p = rotate_y(p_world, spin);
    p = rotate_x(p, 0.34 + spin * 0.27);
    p = rotate_z(p, spin * 0.19);
    return p;
}

fn rotate_x(p: vec3f, a: f32) -> vec3f {
    let c = cos(a);
    let s = sin(a);
    return vec3f(p.x, p.y * c - p.z * s, p.y * s + p.z * c);
}

fn rotate_y(p: vec3f, a: f32) -> vec3f {
    let c = cos(a);
    let s = sin(a);
    return vec3f(p.x * c + p.z * s, p.y, -p.x * s + p.z * c);
}

fn rotate_z(p: vec3f, a: f32) -> vec3f {
    let c = cos(a);
    let s = sin(a);
    return vec3f(p.x * c - p.y * s, p.x * s + p.y * c, p.z);
}

fn value_noise_2d(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);

    let a = hash21(i + vec2f(0.0, 0.0));
    let b = hash21(i + vec2f(1.0, 0.0));
    let c = hash21(i + vec2f(0.0, 1.0));
    let d = hash21(i + vec2f(1.0, 1.0));

    let u = f * f * (3.0 - 2.0 * f);
    return mix(a, b, u.x) + (c - a) * u.y * (1.0 - u.x)
        + (d - b) * u.x * u.y;
}

fn hash21(p: vec2f) -> f32 {
    var q = fract(vec3f(p.xyx) * 0.1031);
    q += dot(q, q.yzx + 33.33);
    return fract((q.x + q.y) * q.z);
}

fn hash11(x: f32) -> f32 {
    return fract(sin(x * 127.1 + 311.7) * 43758.5453);
}

fn motion_rank(index: i32) -> i32 {
    switch index {
        case 0: { return 3; }
        case 1: { return 7; }
        case 2: { return 1; }
        case 3: { return 5; }
        case 4: { return 0; }
        case 5: { return 8; }
        case 6: { return 2; }
        case 7: { return 6; }
        default: { return 4; }
    }
}

fn elongated_box_mask_x(index: i32) -> f32 {
    switch index {
        case 0: { return 1.0; }
        case 3: { return 1.0; }
        case 6: { return 1.0; }
        default: { return 0.0; }
    }
}

fn elongated_box_mask_y(index: i32) -> f32 {
    switch index {
        case 1: { return 1.0; }
        case 4: { return 1.0; }
        case 7: { return 1.0; }
        default: { return 0.0; }
    }
}

fn elongated_box_mask_z(index: i32) -> f32 {
    switch index {
        case 2: { return 1.0; }
        case 5: { return 1.0; }
        case 8: { return 1.0; }
        default: { return 0.0; }
    }
}
