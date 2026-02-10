struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // a: width, height, beats, color_mix
    a: vec4f,
    // b: march_steps, reserved, reserved, reserved
    b: vec4f,
    // c: cam_distance, cam_y_rotation, focal_len, fog_density
    c: vec4f,
    // d: sphere_offset, sphere_radius, blend_k, reserved
    d: vec4f,
    // e: hue_shift, saturation, contrast, reserved
    e: vec4f,
    // f: harmonic_amp_1, harmonic_freq_1, harmonic_amp_2, harmonic_freq_2
    f: vec4f,
    // g: harmonic_warp, harmonic_ridge, harmonic_phase, stretch_y
    g: vec4f,
    // h: light_x, light_y, light_z, light_intensity
    h: vec4f,
    // i: shadow_strength, shadow_softness, reserved, reserved
    i: vec4f,
    // j: rim_strength, rim_power, emissive_strength, spec_power
    j: vec4f,
    // k: reserved, triangle_size, triangle_rotation, reserved
    k: vec4f,
    // l: motion_speed, motion_amount, blend_pulse_amount, blend_pulse_freq
    l: vec4f,
    // m: energy_strength, energy_power, energy_freq, chroma_strength
    m: vec4f,
    // n: reserved, reserved, reserved, reserved
    n: vec4f,
    // o: bump_amp, bump_freq, bump_sharpness, twist_amount
    o: vec4f,
    // p: stretch_amount, reserved, reserved, reserved
    p: vec4f,
    q: vec4f,
    r: vec4f,
    s: vec4f,
    t: vec4f,
    u: vec4f,
    v: vec4f,
    w: vec4f,
    x: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

const MAX_MARCH_STEPS_CAP: i32 = 256;
const MAX_SHADOW_STEPS: i32 = 64;
const MAX_AO_SAMPLES: i32 = 6;
const MAX_DIST: f32 = 30.0;
const SURF_DIST: f32 = 0.0012;
const NORMAL_EPS: f32 = 0.0012;
const MARCH_SAFETY: f32 = 0.82;
const ORBIT_AMOUNT: f32 = 0.2;
const CENTER_LIFT: f32 = 0.12;
const AO_STRENGTH: f32 = 1.25;
const AO_STEP: f32 = 0.02;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.pos = vert.position;
    return out;
}

@fragment
fn fs_main(@location(0) position: vec2f) -> @location(0) vec4f {
    let uv = correct_aspect(position);
    let beats = params.a.z;
    let color_mix = clamp(params.a.w, 0.0, 1.0);
    let hue_shift = params.e.x;
    let saturation = max(params.e.y, 0.0);
    let contrast = max(params.e.z, 0.0);
    let light_pos = vec3f(params.h.x, params.h.y, params.h.z);
    let light_intensity = max(params.h.w, 0.0);
    let shadow_strength = clamp(params.i.x, 0.0, 1.0);
    let shadow_softness = max(params.i.y, 0.0001);
    let rim_strength = max(params.j.x, 0.0);
    let rim_power = max(params.j.y, 0.0001);
    let emissive_strength = max(params.j.z, 0.0);
    let spec_power = max(params.j.w, 1.0);
    let energy_strength = max(params.m.x, 0.0);
    let energy_power = max(params.m.y, 0.0001);
    let energy_freq = max(params.m.z, 0.0);
    let chroma_strength = max(params.m.w, 0.0);
    let complexity = shape_complexity();
    let surf_eps = mix(SURF_DIST, SURF_DIST * 2.2, complexity);

    let max_steps = i32(round(clamp(
        params.b.x,
        1.0,
        f32(MAX_MARCH_STEPS_CAP),
    )));

    let cam_dist = max(params.c.x, 0.1);
    let cam_y_rotation = params.c.y;
    let focal_len = max(params.c.z, 0.01);
    let fog_density = max(params.c.w, 0.000001);

    let cam_orbit_angle = beats * cam_y_rotation;
    let ro = rotate_xz(vec3f(0.0, 0.0, -cam_dist), cam_orbit_angle);
    let ta = vec3f(0.0, 0.0, 0.0);

    let ww = normalize(ta - ro);
    let uu = normalize(cross(vec3f(0.0, 1.0, 0.0), ww));
    let vv = cross(ww, uu);
    let rd = normalize(uv.x * uu + uv.y * vv + focal_len * ww);

    let bg_bottom = mix(vec3f(0.005, 0.006, 0.010), vec3f(0.010, 0.006, 0.004), color_mix);
    let bg_top = mix(vec3f(0.020, 0.014, 0.028), vec3f(0.024, 0.014, 0.008), color_mix);
    let bg = mix(
        bg_bottom,
        bg_top,
        clamp(uv.y * 0.5 + 0.5, 0.0, 1.0),
    );

    let t = ray_march(
        ro,
        rd,
        max_steps,
        surf_eps,
        beats,
    );
    if (t >= MAX_DIST) {
        return vec4f(bg, 1.0);
    }

    let hit_p = ro + rd * t;
    let n = calc_normal(hit_p, beats);
    let shading_bias = surf_eps * (1.2 + 1.2 * complexity);
    let p = hit_p + n * shading_bias;
    let l = normalize(light_pos - p);
    let light_dist = length(light_pos - p);
    let v = normalize(ro - p);
    let h = normalize(l + v);
    let shadow = soft_shadow(
        p,
        l,
        max(shading_bias, surf_eps * 2.0),
        light_dist,
        shadow_softness,
        surf_eps,
        beats,
    );
    let shadow_mix = mix(1.0, shadow, shadow_strength);
    let ao = ambient_occlusion(p, n, surf_eps, beats);

    let diff = max(dot(n, l), 0.0) * shadow_mix;
    let spec = pow(max(dot(n, h), 0.0), spec_power) * shadow_mix;
    let fresnel = pow(1.0 - max(dot(n, v), 0.0), 3.0);
    let rim = pow(1.0 - max(dot(n, v), 0.0), rim_power) * rim_strength;

    let base = mix(vec3f(0.18, 0.72, 0.98), vec3f(0.98, 0.46, 0.22), color_mix);
    var color = base * (0.12 + 0.88 * diff);
    color *= ao * light_intensity;
    color += vec3f(spec) * 0.85 * light_intensity;
    color += vec3f(0.9, 0.3, 1.0) * fresnel * 0.35;
    color += mix(vec3f(0.35, 0.6, 1.0), vec3f(1.0, 0.5, 0.25), color_mix)
        * rim
        * emissive_strength;
    let energy_edge = pow(1.0 - max(dot(n, v), 0.0), energy_power);
    let energy_flow = 0.5 + 0.5 * sin(
        (p.y + 0.7 * p.z) * energy_freq + beats * 0.6,
    );
    let energy_color = mix(
        vec3f(0.2, 0.85, 1.0),
        vec3f(1.0, 0.3, 0.8),
        energy_flow,
    );
    color += energy_color * energy_edge * energy_strength;
    let chroma = chroma_strength * energy_edge;
    color += vec3f(chroma, -0.2 * chroma, 0.3 * chroma);

    let fog = exp(-fog_density * t * t);
    color = mix(bg, color, fog);
    color = tone_map_filmic(color);
    color = apply_color_grade(color, hue_shift, saturation, contrast);
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

fn ray_march(
    ro: vec3f,
    rd: vec3f,
    max_steps: i32,
    surf_eps: f32,
    beats: f32,
) -> f32 {
    var dist = 0.0;
    for (var i = 0; i < MAX_MARCH_STEPS_CAP; i = i + 1) {
        if (i >= max_steps) {
            break;
        }
        let p = ro + rd * dist;
        let scene_dist = scene_sdf(p, beats);
        if (scene_dist < surf_eps) {
            break;
        }
        dist += scene_dist * MARCH_SAFETY;
        if (dist >= MAX_DIST) {
            break;
        }
    }
    return dist;
}

fn scene_sdf(p: vec3f, beats: f32) -> f32 {
    let motion_speed = params.l.x;
    let motion_amount = params.l.y;
    let shape_phase = params.g.z;

    let blend_pulse_amount = params.l.z;
    let blend_pulse_freq = params.l.w;
    let phase = shape_phase + beats * motion_speed;
    let tri_size = max(params.k.y, 0.0001);
    let tri_rot = params.k.z;
    let lift = CENTER_LIFT * motion_amount * sin(beats * 0.41);

    var c1 = vec3f(cos(tri_rot), sin(tri_rot), 0.0) * tri_size;
    var c2 = vec3f(
        cos(tri_rot + 2.0943951),
        sin(tri_rot + 2.0943951),
        0.0,
    ) * tri_size;
    var c3 = vec3f(
        cos(tri_rot + 4.1887902),
        sin(tri_rot + 4.1887902),
        0.0,
    ) * tri_size;

    let tri_breath = 1.0 + 0.12 * motion_amount * sin(phase * 0.67);
    c1 *= tri_breath;
    c2 *= tri_breath;
    c3 *= tri_breath;

    c3.y += lift;

    let orbit = ORBIT_AMOUNT * motion_amount * sin(beats * 0.33);
    c1 = rotate_xz(c1, orbit);
    c2 = rotate_xz(c2, orbit);
    c3 = rotate_xz(c3, orbit);

    let drift = motion_amount * vec3f(
        0.32 * sin(phase * 0.83),
        0.18 * cos(phase * 0.71),
        0.26 * sin(phase * 0.57),
    );
    c1 += drift + motion_amount * vec3f(
        0.16 * sin(phase + 0.0),
        0.11 * cos(phase * 1.17 + 0.0),
        0.12 * sin(phase * 1.31 + 0.0),
    );
    c2 += drift + motion_amount * vec3f(
        0.16 * sin(phase + 2.0943951),
        0.11 * cos(phase * 1.17 + 2.0943951),
        0.12 * sin(phase * 1.31 + 2.0943951),
    );
    c3 += drift + motion_amount * vec3f(
        0.16 * sin(phase + 4.1887902),
        0.11 * cos(phase * 1.17 + 4.1887902),
        0.12 * sin(phase * 1.31 + 4.1887902),
    );

    let blend = max(
        params.d.z + blend_pulse_amount * sin(beats * blend_pulse_freq),
        0.0001,
    );

    let d1 = blob_sdf(p, c1, phase + 0.0);
    let d2 = blob_sdf(p, c2, phase + 2.1);
    let d3 = blob_sdf(p, c3, phase + 4.2);

    // True 3-way smooth union so all blobs can merge as a single mass.
    let outer_blend = max(
        0.0001,
        blend * (0.35 + 0.65 * clamp(motion_amount, 0.0, 1.0)),
    );
    let d12 = smin(d1, d2, outer_blend);
    let d13 = smin(d1, d3, blend);
    let d23 = smin(d2, d3, blend);
    let d123 = smin(d12, d13, blend);
    return smin(d123, d23, blend);
}

fn blob_sdf(p: vec3f, center: vec3f, phase: f32) -> f32 {
    let sphere_radius = params.d.y;
    let rel = p - center;
    let dir = safe_normalize(rel);

    let stretch_amount = params.g.w + params.p.x;
    let stretch_term = 0.45 * stretch_amount
        * (dir.y * dir.y - 0.5 * (dir.x * dir.x + dir.z * dir.z));
    let bump = bump_field(dir, phase);
    let r_mod = sphere_radius * (
        1.0
            + stretch_term
            + bump
    );

    return length(rel) - max(r_mod, 0.02);
}

fn calc_normal(p: vec3f, beats: f32) -> vec3f {
    let complexity = shape_complexity();
    let e = mix(NORMAL_EPS, NORMAL_EPS * 2.5, complexity);
    let nx = scene_sdf(p + vec3f(e, 0.0, 0.0), beats)
        - scene_sdf(p - vec3f(e, 0.0, 0.0), beats);
    let ny = scene_sdf(p + vec3f(0.0, e, 0.0), beats)
        - scene_sdf(p - vec3f(0.0, e, 0.0), beats);
    let nz = scene_sdf(p + vec3f(0.0, 0.0, e), beats)
        - scene_sdf(p - vec3f(0.0, 0.0, e), beats);
    return normalize(vec3f(nx, ny, nz));
}

fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}

fn safe_normalize(v: vec3f) -> vec3f {
    let len = length(v);
    if (len < 0.00001) {
        return vec3f(0.0, 1.0, 0.0);
    }
    return v / len;
}

fn bump_field(dir: vec3f, phase: f32) -> f32 {
    let amp1 = params.f.x;
    let freq1 = max(params.f.y, 0.0);
    let amp2 = params.f.z;
    let freq2 = max(params.f.w, 0.0);
    let warp = params.g.x;
    let ridge = clamp(params.g.y, 0.0, 1.0);

    let h1 = sin((dir.x + 0.31 * dir.y) * (freq1 + 0.0001) + phase + warp * dir.z);
    let h2 = sin(
        (dir.y - 0.27 * dir.z) * (freq2 * 0.87 + 1.3)
            - phase * 0.7
            + warp * dir.x,
    ) * sin(
        (dir.z + 0.23 * dir.x) * (freq2 * 1.13 + 2.1)
            + phase * 0.5
            - warp * dir.y,
    );

    var field = amp1 * h1 + amp2 * h2;
    field = ridge_shape(field, ridge);

    let bump_amp = params.o.x;
    let bump_freq = max(params.o.y, 0.0);
    let bump_sharp = clamp(params.o.z, 0.0, 1.0);
    let twist = params.o.w;
    var bump = sin((dir.x + twist * dir.y) * bump_freq + phase)
        * sin(
            (dir.z - twist * dir.x) * (bump_freq * 1.37 + 1.0)
                - phase * 0.6,
        );
    bump = ridge_shape(bump, bump_sharp);
    field += bump_amp * bump;
    return field;
}

fn ridge_shape(x: f32, ridge: f32) -> f32 {
    let power = mix(1.0, 0.35, ridge);
    return sign(x) * pow(max(abs(x), 0.00001), power);
}

fn rotate2d(v: vec2f, angle: f32) -> vec2f {
    let c = cos(angle);
    let s = sin(angle);
    return vec2f(c * v.x - s * v.y, s * v.x + c * v.y);
}

fn rotate_xz(v: vec3f, angle: f32) -> vec3f {
    let c = cos(angle);
    let s = sin(angle);
    return vec3f(c * v.x - s * v.z, v.y, s * v.x + c * v.z);
}

fn soft_shadow(
    ro: vec3f,
    rd: vec3f,
    min_t: f32,
    max_t: f32,
    softness: f32,
    surf_eps: f32,
    beats: f32,
) -> f32 {
    var result = 1.0;
    var t = min_t;
    for (var i = 0; i < MAX_SHADOW_STEPS; i = i + 1) {
        let h = scene_sdf(ro + rd * t, beats);
        if (h < surf_eps * 0.6) {
            break;
        }
        result = min(result, softness * h / max(t, 0.01));
        t += clamp(h, surf_eps * 0.35, 0.22);
        if (t > max_t) {
            break;
        }
    }
    return clamp(result, 0.0, 1.0);
}

fn ambient_occlusion(
    p: vec3f,
    n: vec3f,
    surf_eps: f32,
    beats: f32,
) -> f32 {
    var occ = 0.0;
    var scale = 1.0;
    let ao_bias = surf_eps * 2.0;
    for (var i = 1; i <= MAX_AO_SAMPLES; i = i + 1) {
        let h = AO_STEP * f32(i);
        let d = scene_sdf(p + n * (h + ao_bias), beats);
        occ += max(h - d, 0.0) * scale;
        scale *= 0.75;
    }
    return clamp(exp(-occ * AO_STRENGTH * 1.1), 0.0, 1.0);
}

fn shape_complexity() -> f32 {
    let h1 = abs(params.f.x) * (0.15 + 0.05 * params.f.y);
    let h2 = abs(params.f.z) * (0.15 + 0.05 * params.f.w);
    let ridge = params.g.y * 0.25;
    let warp = abs(params.g.x) * 0.08;
    let stretch = abs(params.g.w + params.p.x) * 0.35;
    let bump = abs(params.o.x) * (0.2 + 0.05 * params.o.y);
    let twist = abs(params.o.w) * 0.12;
    return clamp(h1 + h2 + ridge + warp + stretch + bump + twist, 0.0, 1.0);
}

fn tone_map_filmic(color: vec3f) -> vec3f {
    let x = max(color - vec3f(0.004), vec3f(0.0));
    return (x * (6.2 * x + vec3f(0.5))) / (x * (6.2 * x + vec3f(1.7)) + vec3f(0.06));
}

fn apply_color_grade(
    color: vec3f,
    hue_shift: f32,
    saturation: f32,
    contrast: f32,
) -> vec3f {
    var hsv = rgb_to_hsv(max(color, vec3f(0.0)));
    hsv.x = fract(hsv.x + hue_shift);
    hsv.y = clamp(hsv.y * saturation, 0.0, 1.0);
    var graded = hsv_to_rgb(hsv);
    graded = (graded - vec3f(0.5)) * contrast + vec3f(0.5);
    return clamp(graded, vec3f(0.0), vec3f(1.0));
}

fn rgb_to_hsv(c: vec3f) -> vec3f {
    let k = vec4f(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    let p = mix(
        vec4f(c.bg, k.wz),
        vec4f(c.gb, k.xy),
        select(0.0, 1.0, c.b < c.g),
    );
    let q = mix(
        vec4f(p.xyw, c.r),
        vec4f(c.r, p.yzx),
        select(0.0, 1.0, p.x < c.r),
    );
    let d = q.x - min(q.w, q.y);
    let e = 1.0e-10;
    return vec3f(
        abs(q.z + (q.w - q.y) / (6.0 * d + e)),
        d / (q.x + e),
        q.x,
    );
}

fn hsv_to_rgb(c: vec3f) -> vec3f {
    let p = abs(fract(c.xxx + vec3f(0.0, 2.0 / 3.0, 1.0 / 3.0)) * 6.0
        - vec3f(3.0));
    return c.z * mix(vec3f(1.0), clamp(p - vec3f(1.0), vec3f(0.0), vec3f(1.0)), c.y);
}
