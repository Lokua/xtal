struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // a: width, height, beats, speed
    a: vec4f,
    // b: box_size, spacing, motion_amp, line_thickness
    b: vec4f,
    // c: rotation_amount, cube_offset, glow, palette_mix
    c: vec4f,
    // d: box_count, zoom, grain, pattern_morph_rate
    d: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

const MAX_BOXES: i32 = 16;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.pos = vert.position;
    return out;
}

@fragment
fn fs_main(@location(0) position: vec2f) -> @location(0) vec4f {
    let pos = correct_aspect(position);
    let beats = params.a.z;
    let speed = max(params.a.w, 0.01);
    let t = beats * speed;

    let box_size = clamp(params.b.x, 0.05, 0.8);
    let spacing = max(params.b.y, 0.15);
    let motion_amp = max(params.b.z, 0.0);
    let line_thickness = clamp(params.b.w, 0.001, box_size * 0.4);

    let rotation_amount = max(params.c.x, 0.0);
    let cube_offset = clamp(params.c.y, 0.0, 0.35);
    let glow = max(params.c.z, 0.0);
    let palette_mix = clamp(params.c.w, 0.0, 1.0);

    let box_count = clamp(round(params.d.x), 1.0, f32(MAX_BOXES));
    let zoom = max(params.d.y, 0.3);
    let grain_amount = clamp(params.d.z, 0.0, 0.2);
    let pattern_morph_rate = clamp(params.d.w, 0.0, 0.5);
    let morph_t = beats * pattern_morph_rate;

    let p = pos * zoom;
    let bg_a = vec3f(0.010, 0.012, 0.020);
    let bg_b = vec3f(0.020, 0.014, 0.034);
    var bg = mix(bg_a, bg_b, smoothstep(-1.0, 1.0, pos.y));

    var color = bg;
    let cols: i32 = 4;
    let rows = (i32(box_count) + cols - 1) / cols;
    let half_cols = f32(cols - 1) * 0.5;
    let half_rows = f32(max(rows - 1, 0)) * 0.5;

    let front_col = mix(
        vec3f(0.24, 0.28, 0.34),
        vec3f(0.72, 0.92, 1.0),
        palette_mix,
    );
    let back_col = mix(
        vec3f(0.15, 0.17, 0.22),
        vec3f(0.36, 0.72, 0.92),
        palette_mix,
    );
    let link_col = mix(
        vec3f(0.20, 0.22, 0.28),
        vec3f(0.40, 0.95, 0.86),
        palette_mix,
    );

    for (var i = 0; i < MAX_BOXES; i = i + 1) {
        if (f32(i) >= box_count) {
            break;
        }

        let fi = f32(i);
        let col = f32(i % cols) - half_cols;
        let row = f32(i / cols) - half_rows;
        let base_center = vec2f(
            col * spacing,
            row * spacing * 0.9,
        );
        let morph_a = sin(morph_t * 0.37 + fi * 0.41);
        let morph_b = cos(morph_t * 0.29 + fi * 0.53);
        let drift_freq_x = 0.7 + fi * 0.09 + morph_a * 0.08;
        let drift_freq_y = 0.9 + fi * 0.11 + morph_b * 0.08;
        let drift_phase_x = fi * 1.3 + morph_b * 0.9;
        let drift_phase_y = fi * 0.8 + morph_a * 0.9;
        let drift = vec2f(
            sin(t * drift_freq_x + drift_phase_x),
            cos(t * drift_freq_y + drift_phase_y),
        ) * motion_amp;
        let center = base_center + drift;
        let angle_freq = 0.5 + fi * 0.07 + morph_a * 0.05;
        let angle_phase = fi * 0.6 + morph_b * 0.4;
        let angle = sin(t * angle_freq + angle_phase) * rotation_amount;
        let q = rot(p - center, angle);
        let q_back = q - vec2f(cube_offset, -cube_offset);

        let d_front = abs(sd_box(q, vec2f(box_size))) - line_thickness;
        let d_back = abs(sd_box(q_back, vec2f(box_size))) - line_thickness;
        let front = 1.0 - smoothstep(0.0, line_thickness * 1.5, d_front);
        let back = 1.0 - smoothstep(0.0, line_thickness * 1.5, d_back);

        let c0 = vec2f(-box_size, -box_size);
        let c1 = vec2f(box_size, -box_size);
        let c2 = vec2f(box_size, box_size);
        let c3 = vec2f(-box_size, box_size);
        let off = vec2f(cube_offset, -cube_offset);
        let link0 = 1.0 - smoothstep(
            0.0,
            line_thickness * 1.5,
            sd_segment(q, c0, c0 + off) - line_thickness * 0.75,
        );
        let link1 = 1.0 - smoothstep(
            0.0,
            line_thickness * 1.5,
            sd_segment(q, c1, c1 + off) - line_thickness * 0.75,
        );
        let link2 = 1.0 - smoothstep(
            0.0,
            line_thickness * 1.5,
            sd_segment(q, c2, c2 + off) - line_thickness * 0.75,
        );
        let link3 = 1.0 - smoothstep(
            0.0,
            line_thickness * 1.5,
            sd_segment(q, c3, c3 + off) - line_thickness * 0.75,
        );
        let links = max(max(link0, link1), max(link2, link3));

        color = mix(color, back_col, back * 0.8);
        color = mix(color, link_col, links * 0.85);
        color = mix(color, front_col, front);
        color += front_col * (front * glow * 0.25);
    }

    let g = (hash12(pos * vec2f(417.0, 293.0) + t * 0.03) - 0.5) * grain_amount;
    color += vec3f(g);
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

fn sd_segment(p: vec2f, a: vec2f, b: vec2f) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h);
}

fn rot(p: vec2f, angle: f32) -> vec2f {
    let c = cos(angle);
    let s = sin(angle);
    return vec2f(
        c * p.x - s * p.y,
        s * p.x + c * p.y,
    );
}

fn sd_box(p: vec2f, b: vec2f) -> f32 {
    let d = abs(p) - b;
    return length(max(d, vec2f(0.0))) + min(max(d.x, d.y), 0.0);
}

fn hash12(p: vec2f) -> f32 {
    let h = dot(p, vec2f(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}
