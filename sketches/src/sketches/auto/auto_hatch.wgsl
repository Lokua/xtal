// Based on https://www.shadertoy.com/view/MsKfRw

const MAX_STEPS: i32 = 56;
const MAX_DIST: f32 = 20.0;
const SURF_DIST: f32 = 0.0012;
const NOISE_OCTAVES: i32 = 3;
const AO_STEPS: i32 = 1;

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, beats, _
    a: vec4f,
    // morph, rot_speed, cam_z, light_angle
    b: vec4f,
    // hatch_density, hatch_layers, hatch_strength,
    // hatch_angle_spread
    c: vec4f,
    // _, _, paper_grain, brightness
    d: vec4f,
    // include_ao, _, noise_amp, noise_freq
    e: vec4f,
    // arm_layout, limb_rotation_rate, limb_rotation_range, _
    f: vec4f,
    g: vec4f,
    h: vec4f,
    i: vec4f,
    j: vec4f,
    k: vec4f,
    l: vec4f,
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
    let t = params.a.z;
    let morph = params.b.x;
    let arm_layout = params.f.x;
    let rot_speed = params.b.y;
    let cam_z = params.b.z;
    let light_angle = params.b.w;
    let hatch_density = params.c.x;
    let hatch_layers = i32(params.c.y);
    let hatch_strength = params.c.z;
    let angle_spread = params.c.w;
    let paper_grain = params.d.z;
    let brightness_ctrl = params.d.w;
    let disable_ao = params.e.x > 0.5;
    let noise_amp = params.e.z;
    let noise_freq = params.e.w;

    let aspect = w / h;
    var uv = position * 0.5;
    uv.x *= aspect;

    let ro = vec3f(0.0, 0.0, cam_z);
    let rd = normalize(vec3f(uv, 1.0));

    let light = normalize(vec3f(
        cos(light_angle),
        0.6,
        sin(light_angle),
    ));

    // Raymarch in an aspect-aware bounds sphere.
    let morph_smooth = smoothstep(0.0, 1.0, morph);
    let bound_radius = mix(
        1.1,
        2.95 + noise_amp * 0.35,
        morph_smooth,
    );
    let bounds = ray_sphere_bounds(ro, rd, bound_radius);
    var hit_dist = MAX_DIST + 1.0;
    var has_hit = false;
    if bounds.y > bounds.x {
        let result = ray_march(
            ro, rd, t, rot_speed, morph, arm_layout,
            noise_amp, noise_freq, aspect,
            bounds.x, min(bounds.y, MAX_DIST),
        );
        hit_dist = result.x;
        has_hit = result.y > 0.5;
    }
    let hit_pos = ro + rd * hit_dist;

    var scene_brightness = 1.0;

    if has_hit {
        let n = calc_normal(
            hit_pos, t, rot_speed, morph, arm_layout,
            noise_amp, noise_freq, aspect,
        );
        let diff = max(dot(n, light), 0.0);
        let amb = 0.15;
        var ao = 1.0;
        if !disable_ao {
            ao = calc_ao(
                hit_pos, n, t, rot_speed, morph,
                arm_layout, aspect,
            );
        }
        scene_brightness = (diff * 0.85 + amb) * ao;
    }

    scene_brightness = clamp(
        scene_brightness * (0.5 + brightness_ctrl),
        0.0,
        1.0,
    );

    let fc = frag_coord.xy;
    var color = 1.0;

    // Cross-hatching (responds to scene brightness)
    if has_hit {
        var hatch = 0.0;
        var hatch_max = 0.0;
        var count = 0.0;

        for (var idx = 0; idx < 8; idx++) {
            if idx >= hatch_layers {
                break;
            }
            let br = scene_brightness * 1.7;
            let fi = f32(idx);
            let ang = -0.5 - angle_spread * fi * fi;
            let ca = cos(ang);
            let sa = sin(ang);
            let uvh = vec2f(
                ca * fc.x - sa * fc.y,
                sa * fc.x + ca * fc.y,
            ) * vec2f(hatch_density, 1.0) * 1.3;

            let row_jitter = (hash21(vec2f(
                floor(uvh.y * 0.35) + fi * 11.0,
                fi * 7.0,
            )) - 0.5) * 0.35;
            let stripe = abs(fract(uvh.x + row_jitter) - 0.5);
            let rh = smoothstep(0.12, 0.42, stripe);
            let grain = abs(hash21(
                uvh * vec2f(0.37, 0.09)
                    + vec2f(fi * 17.0, fi * 3.0),
            ) - 0.5);
            let h_val = 1.0
                - smoothstep(0.5, 1.5, rh + br)
                - 0.3 * grain;
            hatch += h_val;
            hatch_max = max(hatch_max, h_val);
            count += 1.0;

            if fi > (1.0 - br) * f32(hatch_layers)
                && idx >= 2 {
                break;
            }
        }

        let hatch_val = clamp(
            mix(
                hatch / max(count, 1.0),
                hatch_max,
                0.5,
            ),
            0.0,
            1.0,
        );
        color *= 1.0 - hatch_val * hatch_strength;
    }

    // Soften contrast
    color = 1.0 - ((1.0 - color) * 0.7);

    // Paper texture on the entire frame.
    if paper_grain > 0.001 {
        let paper_r = hash21(fc * 1.1)
            - hash21(fc * 1.1 + vec2f(1.5, -1.5));
        color *= 1.0 + paper_grain * paper_r;
    }

    color = clamp(color, 0.0, 1.0);
    return vec4f(vec3f(color), 1.0);
}

// ----------------------------------------------------------------
//  Scene SDF
// ----------------------------------------------------------------

fn smooth_min(a: f32, b: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}

fn sd_capsule(p: vec3f, a: vec3f, b: vec3f, r: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h) - r;
}

fn blob_dir(i: i32) -> vec3f {
    switch i {
        case 0 { return normalize(vec3f(1.0, 0.2, 0.0)); }
        case 1 { return normalize(vec3f(-1.0, -0.1, 0.1)); }
        case 2 { return normalize(vec3f(0.2, 1.0, 0.3)); }
        case 3 { return normalize(vec3f(-0.1, -1.0, -0.2)); }
        case 4 { return normalize(vec3f(0.0, 0.3, 1.0)); }
        case 5 { return normalize(vec3f(0.1, -0.2, -1.0)); }
        case 6 { return normalize(vec3f(0.8, 0.5, 0.4)); }
        default { return normalize(vec3f(-0.7, 0.4, -0.6)); }
    }
}

fn blob_dir_alt(i: i32) -> vec3f {
    switch i {
        case 0 { return normalize(vec3f(0.8, 0.45, 0.1)); }
        case 1 { return normalize(vec3f(-0.9, 0.2, 0.35)); }
        case 2 { return normalize(vec3f(0.35, 0.85, -0.2)); }
        case 3 { return normalize(vec3f(-0.4, -0.9, 0.15)); }
        case 4 { return normalize(vec3f(0.2, -0.15, 0.95)); }
        case 5 { return normalize(vec3f(-0.05, 0.25, -0.95)); }
        case 6 { return normalize(vec3f(0.95, -0.1, 0.3)); }
        default { return normalize(vec3f(-0.75, 0.55, -0.35)); }
    }
}

fn arm_seed(i: i32) -> f32 {
    switch i {
        case 0 { return 0.12; }
        case 1 { return 0.76; }
        case 2 { return 0.43; }
        case 3 { return 0.91; }
        case 4 { return 0.28; }
        case 5 { return 0.64; }
        case 6 { return 0.35; }
        default { return 0.83; }
    }
}

fn exploded_cluster_sdf(
    p: vec3f,
    t: f32,
    morph: f32,
    arm_layout: f32,
    aspect: f32,
) -> f32 {
    let limb_rotation_rate = params.f.y;
    let limb_rotation_range = max(params.f.z, 0.0);
    let m = smoothstep(0.0, 1.0, morph);
    let shape_mix = arm_layout;
    let core_radius = mix(0.8, mix(0.22, 0.19, shape_mix), m);
    var d = length(p) - core_radius;
    if m < 0.01 {
        return d;
    }
    let spread = mix(
        0.0,
        1.45 + 0.55 * aspect + 0.25 * shape_mix,
        m,
    );
    let blend_k = mix(0.42, mix(0.12, 0.16, shape_mix), m);
    let strand_base = mix(0.36, mix(0.05, 0.045, shape_mix), m);

    // Base motion: limb anchors travel between two nearby
    // directions A<->B on a small arc.
    let travel_phase = t * limb_rotation_rate * 6.28318530718;
    let root_radius = core_radius * 0.92;

    for (var i = 0; i < 8; i++) {
        let seed = arm_seed(i);
        let axis_a = normalize(
            mix(blob_dir(i), blob_dir_alt(i), shape_mix),
        );
        let tangent_a = cross(vec3f(0.0, 1.0, 0.0), axis_a);
        let tangent_b = cross(vec3f(1.0, 0.0, 0.0), axis_a);
        let tangent_mix = smoothstep(0.85, 0.98, abs(axis_a.y));
        let tangent = normalize(
            mix(tangent_a, tangent_b, tangent_mix),
        );
        let step_ang = (0.22 + 0.18 * seed) * limb_rotation_range;
        let axis_b = normalize(vec3f(
            axis_a.x * cos(step_ang) + tangent.x * sin(step_ang),
            axis_a.y * cos(step_ang) + tangent.y * sin(step_ang),
            axis_a.z * cos(step_ang) + tangent.z * sin(step_ang),
        ));
        let phase = travel_phase + seed * 6.28318530718;
        let walk_t = 0.5 + 0.5 * sin(phase);
        let axis_walk = normalize(mix(axis_a, axis_b, walk_t));
        let axis_stretch = vec3f(
            axis_walk.x * (1.0 + (aspect - 1.0) * 0.75),
            axis_walk.y,
            axis_walk.z,
        );
        let dir = normalize(axis_stretch);
        let dist = spread * (0.72 + 0.34 * seed);
        let root = dir * root_radius;
        let center = dir * dist;
        let blob_radius = mix(0.18, 0.16, shape_mix)
            + 0.09 * seed;
        let blob = length(p - center) - blob_radius;

        d = smooth_min(d, blob, blend_k);

        let strand_radius = strand_base * (0.8 + 0.35 * (1.0 - seed));
        let strand = sd_capsule(
            p,
            root,
            center * 0.92,
            strand_radius,
        );
        d = smooth_min(d, strand, blend_k * 0.8);
    }

    return d;
}

fn scene_sdf(
    p: vec3f,
    t: f32,
    rot_speed: f32,
    morph: f32,
    arm_layout: f32,
    noise_amp: f32,
    noise_freq: f32,
    aspect: f32,
) -> f32 {
    let rp = scene_space(p, t, rot_speed);
    return scene_sdf_core(rp, t, morph, arm_layout, aspect)
        + scene_noise_displacement(
            rp, t, morph, noise_amp, noise_freq,
        );
}

fn ray_march(
    ro: vec3f,
    rd: vec3f,
    t: f32,
    rot_speed: f32,
    morph: f32,
    arm_layout: f32,
    noise_amp: f32,
    noise_freq: f32,
    aspect: f32,
    min_dist: f32,
    max_dist: f32,
) -> vec2f {
    var d = max(min_dist, 0.0);
    let near_band = 0.22 + noise_amp * 0.35;
    var hit = 0.0;
    for (var i = 0; i < MAX_STEPS; i++) {
        if d > max_dist {
            break;
        }
        let p = ro + rd * d;
        let rp = scene_space(p, t, rot_speed);
        let ds_core = scene_sdf_core(
            rp, t, morph, arm_layout, aspect,
        );
        var ds = ds_core;
        if ds_core < near_band {
            ds += scene_noise_displacement(
                rp, t, morph, noise_amp, noise_freq,
            );
        }
        d += ds * 0.85;
        if abs(ds) < SURF_DIST {
            hit = 1.0;
            break;
        }
        if d > MAX_DIST {
            break;
        }
    }
    if hit < 0.5 {
        d = MAX_DIST + 1.0;
    }
    return vec2f(d, hit);
}

fn ray_sphere_bounds(ro: vec3f, rd: vec3f, r: f32) -> vec2f {
    let b = dot(ro, rd);
    let c = dot(ro, ro) - r * r;
    let h = b * b - c;
    if h < 0.0 {
        return vec2f(1.0, -1.0);
    }
    let s = sqrt(h);
    let t0 = -b - s;
    let t1 = -b + s;
    return vec2f(max(t0, 0.0), t1);
}

fn scene_space(p: vec3f, t: f32, rot_speed: f32) -> vec3f {
    return rotate_y(
        rotate_x(p, t * rot_speed * 0.7),
        t * rot_speed,
    );
}

fn scene_sdf_core(
    rp: vec3f,
    t: f32,
    morph: f32,
    arm_layout: f32,
    aspect: f32,
) -> f32 {
    let m = smoothstep(0.0, 1.0, morph);
    let sphere = length(rp) - 0.8;
    let cluster = exploded_cluster_sdf(
        rp, t, morph, arm_layout, aspect,
    );
    return mix(sphere, cluster, m);
}

fn scene_noise_displacement(
    rp: vec3f,
    _t: f32,
    morph: f32,
    noise_amp: f32,
    noise_freq: f32,
) -> f32 {
    if noise_amp <= 0.0001 {
        return 0.0;
    }
    let m = smoothstep(0.0, 1.0, morph);
    let np = rp * noise_freq;
    let n = fbm3(np);
    return (n - 0.5) * noise_amp * mix(0.45, 1.0, m);
}

fn calc_normal(
    p: vec3f,
    t: f32,
    rot_speed: f32,
    morph: f32,
    arm_layout: f32,
    noise_amp: f32,
    noise_freq: f32,
    aspect: f32,
) -> vec3f {
    let e = vec2f(0.001, 0.0);
    let d = scene_sdf(
        p, t, rot_speed, morph, arm_layout,
        noise_amp, noise_freq, aspect,
    );
    let n = vec3f(
        scene_sdf(
            p + e.xyy, t, rot_speed, morph,
            arm_layout,
            noise_amp, noise_freq, aspect,
        ) - d,
        scene_sdf(
            p + e.yxy, t, rot_speed, morph,
            arm_layout,
            noise_amp, noise_freq, aspect,
        ) - d,
        scene_sdf(
            p + e.yyx, t, rot_speed, morph,
            arm_layout,
            noise_amp, noise_freq, aspect,
        ) - d,
    );
    return normalize(n);
}

fn calc_ao(
    p: vec3f,
    n: vec3f,
    t: f32,
    rot_speed: f32,
    morph: f32,
    arm_layout: f32,
    aspect: f32,
) -> f32 {
    var occ = 0.0;
    var w = 1.0;
    for (var i = 0; i < AO_STEPS; i++) {
        let h = 0.01 + 0.12 * f32(i) / 3.0;
        let ps = scene_space(p + n * h, t, rot_speed);
        let d = scene_sdf_core(
            ps, t, morph, arm_layout, aspect,
        );
        occ += (h - d) * w;
        w *= 0.85;
    }
    return clamp(1.0 - 1.5 * occ, 0.0, 1.0);
}

// ----------------------------------------------------------------
//  Noise (3D value noise + FBM)
// ----------------------------------------------------------------

fn hash31(p: vec3f) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
}

fn noise3(p: vec3f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(
            mix(
                hash31(i + vec3f(0., 0., 0.)),
                hash31(i + vec3f(1., 0., 0.)),
                u.x,
            ),
            mix(
                hash31(i + vec3f(0., 1., 0.)),
                hash31(i + vec3f(1., 1., 0.)),
                u.x,
            ),
            u.y,
        ),
        mix(
            mix(
                hash31(i + vec3f(0., 0., 1.)),
                hash31(i + vec3f(1., 0., 1.)),
                u.x,
            ),
            mix(
                hash31(i + vec3f(0., 1., 1.)),
                hash31(i + vec3f(1., 1., 1.)),
                u.x,
            ),
            u.y,
        ),
        u.z,
    );
}

fn fbm3(p: vec3f) -> f32 {
    var value = 0.0;
    var amp = 0.5;
    var freq = 1.0;
    var q = p;
    for (var i = 0; i < NOISE_OCTAVES; i++) {
        value += amp * noise3(q * freq);
        amp *= 0.5;
        freq *= 2.0;
        q = vec3f(
            q.y * 1.08 + q.x * 0.2,
            q.z * 0.92 - q.y * 0.15,
            q.x * 1.05 + q.z * 0.1,
        );
    }
    return value;
}

// ----------------------------------------------------------------
//  Helpers
// ----------------------------------------------------------------

fn hash21(p: vec2f) -> f32 {
    var p3 = fract(vec3f(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn rotate_x(p: vec3f, a: f32) -> vec3f {
    let c = cos(a);
    let s = sin(a);
    return vec3f(
        p.x,
        c * p.y - s * p.z,
        s * p.y + c * p.z,
    );
}

fn rotate_y(p: vec3f, a: f32) -> vec3f {
    let c = cos(a);
    let s = sin(a);
    return vec3f(
        c * p.x + s * p.z,
        p.y,
        -s * p.x + c * p.z,
    );
}
