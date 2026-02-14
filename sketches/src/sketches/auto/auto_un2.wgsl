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
    // b: reserved, reserved, reserved, reserved
    b: vec4f,
    // c: cam_distance, cam_y_angle, focal_len, fog_density
    c: vec4f,
    // d: reserved, sphere_radius, blend_k, reserved
    d: vec4f,
    // e: hue_shift, saturation, contrast, reserved
    e: vec4f,
    // f: reserved, reserved, reserved, reserved
    f: vec4f,
    // g: reserved, reserved, reserved, reserved
    g: vec4f,
    // h: bg_bottom_r, bg_bottom_g, bg_bottom_b, reserved
    h: vec4f,
    // i: bg_top_r, bg_top_g, bg_top_b, reserved
    i: vec4f,
    // j: rim_strength, rim_power, emissive_strength, reserved
    j: vec4f,
    // k: reserved, triangle_size, triangle_rotation, reserved
    k: vec4f,
    // l: motion_speed, motion_amount, blend_pulse_amount, blend_pulse_freq
    l: vec4f,
    // m: reserved, reserved, reserved, reserved
    m: vec4f,
    // n: satellite_count, satellite_radius, satellite_orbit, reserved
    n: vec4f,
    // o: satellite_speed, reserved, satellite_jitter, satellite_breathe
    o: vec4f,
    // p: flow_amount, flow_scale, reserved, reserved
    p: vec4f,
    // q: topology_amount, reserved, reserved, topology_split
    q: vec4f,
    // r: reserved, topology_drive, reserved, reserved
    r: vec4f,
    // s: reserved, reserved, reserved, reserved
    s: vec4f,
    // t: reserved, reserved, reserved, reserved
    t: vec4f,
    // u: reserved, reserved, reserved, reserved
    u: vec4f,
    v: vec4f,
    w: vec4f,
    x: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

const MAX_MARCH_STEPS: i32 = 64;
const MAX_DIST: f32 = 30.0;
const SURF_DIST: f32 = 0.0012;
const NORMAL_EPS: f32 = 0.0012;
const MARCH_SAFETY: f32 = 0.82;
const ORBIT_AMOUNT: f32 = 0.2;
const CENTER_LIFT: f32 = 0.12;
const SATELLITE_MAX: i32 = 9;
const SATELLITE_ACTIVITY_FIXED: f32 = 0.5;
const SATELLITE_MERGE_FIXED: f32 = 1.0;
const STRAND_STRENGTH_FIXED: f32 = 0.7;
const STRAND_THINNESS_FIXED: f32 = 0.4;

var<private> g_beats: f32;
var<private> g_phase: f32;
var<private> g_motion_amount: f32;
var<private> g_blend: f32;
var<private> g_outer_blend: f32;
var<private> g_c1: vec3f;
var<private> g_c2: vec3f;
var<private> g_c3: vec3f;
var<private> g_blob_radius_bound: f32;
var<private> g_smooth_pad: f32;
var<private> g_sat_count: i32;
var<private> g_sat_radius: f32;
var<private> g_sat_orbit: f32;
var<private> g_sat_activity: f32;
var<private> g_sat_speed: f32;
var<private> g_sat_merge: f32;
var<private> g_sat_jitter: f32;
var<private> g_sat_breathe: f32;
var<private> g_sat_cluster_bound: f32;
var<private> g_flow_amount: f32;
var<private> g_flow_scale: f32;
var<private> g_strand_strength: f32;
var<private> g_strand_thinness: f32;
var<private> g_topology_amount: f32;
var<private> g_topology_strength1: f32;
var<private> g_topology_strength2: f32;
var<private> g_topology_strength3: f32;
var<private> g_topology_blend: f32;
var<private> g_topology_split1: vec3f;
var<private> g_topology_split2: vec3f;
var<private> g_topology_split3: vec3f;
var<private> g_topology_bound: f32;
var<private> g_scene_bound_radius: f32;

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
    let color_mix = params.a.w;
    let hue_shift = params.e.x;
    let saturation = max(params.e.y, 0.0);
    let contrast = max(params.e.z, 0.0);
    let bg_bottom = clamp(params.h.xyz, vec3f(0.0), vec3f(1.0));
    let bg_top = clamp(params.i.xyz, vec3f(0.0), vec3f(1.0));
    let rim_strength = max(params.j.x, 0.0);
    let rim_power = max(params.j.y, 0.0001);
    let emissive_strength = max(params.j.z, 0.0);
    let complexity = shape_complexity();
    let surf_eps = mix(SURF_DIST, SURF_DIST * 2.2, complexity);

    let cam_dist = max(params.c.x, 0.1);
    let cam_y_angle = params.c.y;
    let focal_len = max(params.c.z, 0.01);
    let fog_density = max(params.c.w, 0.000001);

    let cam_orbit_angle = cam_y_angle;
    let ro = rotate_xz(vec3f(0.0, 0.0, -cam_dist), cam_orbit_angle);
    let ta = vec3f(0.0, 0.0, 0.0);

    let ww = normalize(ta - ro);
    let uu = normalize(cross(vec3f(0.0, 1.0, 0.0), ww));
    let vv = cross(ww, uu);
    let rd = normalize(uv.x * uu + uv.y * vv + focal_len * ww);

    let bg = mix(
        bg_bottom,
        bg_top,
        clamp(uv.y * 0.5 + 0.5, 0.0, 1.0),
    );
    prepare_scene_state(beats);

    let t = ray_march(ro, rd, surf_eps);
    if (t >= MAX_DIST) {
        return vec4f(bg, 1.0);
    }

    let hit_p = ro + rd * t;
    let n = calc_normal(hit_p);
    let shading_bias = surf_eps * (1.2 + 1.2 * complexity);
    let p = hit_p + n * shading_bias;
    let v = normalize(ro - p);
    let rim = pow(1.0 - max(dot(n, v), 0.0), rim_power) * rim_strength;

    var color = vec3f(0.0);
    let rim_color = mix(
        vec3f(0.35, 0.6, 1.0),
        vec3f(1.0, 0.5, 0.25),
        color_mix,
    );
    color += rim_color * rim * emissive_strength;

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
    surf_eps: f32,
) -> f32 {
    let hit = ray_sphere_interval(ro, rd, g_scene_bound_radius);
    if (hit.y < 0.0 || hit.x > MAX_DIST) {
        return MAX_DIST;
    }

    var dist = max(hit.x, 0.0);
    let march_end = min(hit.y + 0.15, MAX_DIST);
    var hit_found = false;
    for (var i = 0; i < MAX_MARCH_STEPS; i = i + 1) {
        let p = ro + rd * dist;
        let scene_dist = scene_sdf(p);
        if (scene_dist < surf_eps) {
            hit_found = true;
            break;
        }
        dist += scene_dist * MARCH_SAFETY;
        if (dist >= march_end) {
            break;
        }
    }
    if (!hit_found) {
        return MAX_DIST;
    }
    return dist;
}

fn ray_sphere_interval(ro: vec3f, rd: vec3f, radius: f32) -> vec2f {
    let b = dot(ro, rd);
    let c = dot(ro, ro) - radius * radius;
    let h = b * b - c;
    if (h < 0.0) {
        return vec2f(MAX_DIST + 1.0, -1.0);
    }
    let s = sqrt(h);
    let t0 = -b - s;
    let t1 = -b + s;
    return vec2f(t0, t1);
}

fn prepare_scene_state(beats: f32) {
    g_beats = beats;
    let motion_speed = params.l.x;
    g_motion_amount = params.l.y;

    let blend_pulse_amount = params.l.z;
    let blend_pulse_freq = params.l.w;
    g_sat_count = i32(round(params.n.x));
    g_sat_radius = params.n.y;
    g_sat_orbit = params.n.z;
    g_sat_activity = SATELLITE_ACTIVITY_FIXED;
    g_sat_speed = params.o.x;
    g_sat_merge = SATELLITE_MERGE_FIXED;
    g_sat_jitter = params.o.z;
    g_sat_breathe = params.o.w;
    g_flow_amount = params.p.x;
    g_flow_scale = params.p.y;
    g_strand_strength = STRAND_STRENGTH_FIXED;
    g_strand_thinness = STRAND_THINNESS_FIXED;
    let topology_amount = params.q.x;
    let topology_split = params.q.w;
    let topology_drive = params.r.y;
    g_topology_amount = topology_amount * topology_drive;
    g_topology_strength1 = 0.0;
    g_topology_strength2 = 0.0;
    g_topology_strength3 = 0.0;
    g_topology_blend = 0.0001;
    g_topology_split1 = vec3f(0.0);
    g_topology_split2 = vec3f(0.0);
    g_topology_split3 = vec3f(0.0);
    g_topology_bound = 0.0;
    g_sat_cluster_bound = 0.0;
    g_phase = beats * motion_speed;

    g_blob_radius_bound = max(params.d.y, 0.02);
    let blend_max = max(params.d.z + abs(blend_pulse_amount), 0.0001);
    g_smooth_pad = 0.25 * blend_max;

    let tri_size = max(params.k.y, 0.0001);
    let tri_rot = params.k.z;
    let lift = CENTER_LIFT * g_motion_amount * sin(beats * 0.41);

    g_c1 = vec3f(cos(tri_rot), sin(tri_rot), 0.0) * tri_size;
    g_c2 = vec3f(
        cos(tri_rot + 2.0943951),
        sin(tri_rot + 2.0943951),
        0.0,
    ) * tri_size;
    g_c3 = vec3f(
        cos(tri_rot + 4.1887902),
        sin(tri_rot + 4.1887902),
        0.0,
    ) * tri_size;

    let tri_breath = 1.0 + 0.12 * g_motion_amount * sin(g_phase * 0.67);
    g_c1 *= tri_breath;
    g_c2 *= tri_breath;
    g_c3 *= tri_breath;
    g_c3.y += lift;

    let orbit = ORBIT_AMOUNT * g_motion_amount * sin(beats * 0.33);
    g_c1 = rotate_xz(g_c1, orbit);
    g_c2 = rotate_xz(g_c2, orbit);
    g_c3 = rotate_xz(g_c3, orbit);

    let drift = g_motion_amount * vec3f(
        0.32 * sin(g_phase * 0.83),
        0.18 * cos(g_phase * 0.71),
        0.26 * sin(g_phase * 0.57),
    );
    g_c1 += drift + g_motion_amount * vec3f(
        0.16 * sin(g_phase + 0.0),
        0.11 * cos(g_phase * 1.17 + 0.0),
        0.12 * sin(g_phase * 1.31 + 0.0),
    );
    g_c2 += drift + g_motion_amount * vec3f(
        0.16 * sin(g_phase + 2.0943951),
        0.11 * cos(g_phase * 1.17 + 2.0943951),
        0.12 * sin(g_phase * 1.31 + 2.0943951),
    );
    g_c3 += drift + g_motion_amount * vec3f(
        0.16 * sin(g_phase + 4.1887902),
        0.11 * cos(g_phase * 1.17 + 4.1887902),
        0.12 * sin(g_phase * 1.31 + 4.1887902),
    );

    if (g_flow_amount > 0.0001) {
        let flow_t = beats * (0.31 + motion_speed * 0.69);
        let flow_amt = 0.35 * g_flow_amount;
        g_c1 += flow_amt * curl_advection(
            g_c1 + vec3f(0.7, 1.1, -0.4),
            flow_t,
            g_flow_scale,
        );
        g_c2 += flow_amt * curl_advection(
            g_c2 + vec3f(-0.9, 0.3, 0.8),
            flow_t + 1.7,
            g_flow_scale,
        );
        g_c3 += flow_amt * curl_advection(
            g_c3 + vec3f(0.2, -1.2, 1.4),
            flow_t + 3.1,
            g_flow_scale,
        );
    }

    g_blend = max(
        params.d.z + blend_pulse_amount * sin(beats * blend_pulse_freq),
        0.0001,
    );
    g_outer_blend = max(
        0.0001,
        g_blend * (0.35 + 0.65 * g_motion_amount),
    );

    if (g_topology_amount > 0.0001) {
        g_topology_strength1 = g_topology_amount;
        g_topology_strength2 = g_topology_amount;
        g_topology_strength3 = g_topology_amount;

        let split_base = max(params.d.y, 0.02)
            * mix(0.14, 3.0, topology_split);
        let split1 = split_base * g_topology_strength1;
        let split2 = split_base * g_topology_strength2;
        let split3 = split_base * g_topology_strength3;

        let dir1 = safe_normalize(g_c1 + vec3f(0.0, 0.0, 0.001));
        let dir2 = safe_normalize(g_c2 + vec3f(0.0, 0.0, 0.001));
        let dir3 = safe_normalize(g_c3 + vec3f(0.0, 0.0, 0.001));

        g_topology_split1 = dir1 * split1;
        g_topology_split2 = dir2 * split2;
        g_topology_split3 = dir3 * split3;
        g_topology_blend = max(
            0.0001,
            g_blend * mix(0.35, 0.72, topology_split),
        );

        let max_strength = max(
            g_topology_strength1,
            max(g_topology_strength2, g_topology_strength3),
        );
        g_topology_bound = split_base * (0.75 + 0.5 * max_strength);
    }

    let c1_len = length(g_c1);
    let c2_len = length(g_c2);
    let c3_len = length(g_c3);
    let max_center_len = max(c1_len, max(c2_len, c3_len));
    let blob_bound = g_blob_radius_bound + g_topology_bound + g_smooth_pad;
    var max_local_bound = blob_bound;
    if (g_sat_count > 0 && g_sat_radius > 0.0 && g_sat_orbit > 0.0) {
        let sat_orbit_bound = g_sat_orbit * (1.0 + 0.45 * g_sat_jitter);
        let sat_radius_bound = g_sat_radius * (1.0 + 0.35 * g_sat_breathe);
        g_sat_cluster_bound = sat_orbit_bound + sat_radius_bound + g_smooth_pad;
        max_local_bound = max(max_local_bound, g_sat_cluster_bound);
    }
    g_scene_bound_radius = max_center_len + max_local_bound + 0.25;
}

fn scene_sdf(p: vec3f) -> f32 {
    // Conservative broad-phase bound. If far enough, return early and skip
    // expensive harmonic/satellite evaluation while preserving visual result.
    let d1_bound = length(p - g_c1)
        - (g_blob_radius_bound + g_topology_bound);
    let d2_bound = length(p - g_c2)
        - (g_blob_radius_bound + g_topology_bound);
    let d3_bound = length(p - g_c3)
        - (g_blob_radius_bound + g_topology_bound);
    var scene_bound = min(min(d1_bound, d2_bound), d3_bound) - g_smooth_pad;
    if (g_sat_cluster_bound > 0.0) {
        let sat1_bound = length(p - g_c1) - g_sat_cluster_bound;
        let sat2_bound = length(p - g_c2) - g_sat_cluster_bound;
        let sat3_bound = length(p - g_c3) - g_sat_cluster_bound;
        scene_bound = min(
            scene_bound,
            min(min(sat1_bound, sat2_bound), sat3_bound),
        );
    }
    if (scene_bound > 0.35) {
        return scene_bound;
    }

    let d1_base = blob_sdf(p, g_c1, g_phase + 0.0);
    let d2_base = blob_sdf(p, g_c2, g_phase + 2.1);
    let d3_base = blob_sdf(p, g_c3, g_phase + 4.2);
    var d1 = d1_base;
    var d2 = d2_base;
    var d3 = d3_base;
    if (g_topology_strength1 > 0.0001) {
        let d1_split = split_blob_sdf(
            p,
            g_c1,
            g_phase + 0.0,
            g_topology_split1,
            g_topology_strength1,
        );
        d1 = mix(d1_base, d1_split, g_topology_strength1);
    }
    if (g_topology_strength2 > 0.0001) {
        let d2_split = split_blob_sdf(
            p,
            g_c2,
            g_phase + 2.1,
            g_topology_split2,
            g_topology_strength2,
        );
        d2 = mix(d2_base, d2_split, g_topology_strength2);
    }
    if (g_topology_strength3 > 0.0001) {
        let d3_split = split_blob_sdf(
            p,
            g_c3,
            g_phase + 4.2,
            g_topology_split3,
            g_topology_strength3,
        );
        d3 = mix(d3_base, d3_split, g_topology_strength3);
    }

    // True 3-way smooth union so all blobs can merge as a single mass.
    let d12 = smin(d1, d2, g_outer_blend);
    let d13 = smin(d1, d3, g_blend);
    let d23 = smin(d2, d3, g_blend);
    let d123 = smin(d12, d13, g_blend);
    var scene = smin(d123, d23, g_blend);

    // Satellite metaballs: orbiting droplets that periodically dive in/out.
    if (g_sat_count > 0 && g_sat_radius > 0.0 && g_sat_orbit > 0.0) {
        let sat_blend_base = max(0.0001, g_blend * (0.32 + 0.68 * g_sat_merge));
        let strand_base = sat_blend_base * (0.25 + 0.9 * g_strand_strength);
        for (var i = 0; i < SATELLITE_MAX; i = i + 1) {
            if (i >= g_sat_count) {
                break;
            }
            let hub_idx = i % 3;
            let lane = i / 3;
            var hub = g_c1;
            if (hub_idx == 1) {
                hub = g_c2;
            } else if (hub_idx == 2) {
                hub = g_c3;
            }

            let sat_phase = g_phase * g_sat_speed
                + f32(i) * 1.947
                + f32(lane) * 2.713;
            let radial = g_sat_orbit * (
                1.0
                    + 0.45
                        * g_sat_jitter
                        * sin(sat_phase * 1.63 + f32(hub_idx) * 1.9)
            );
            let theta = sat_phase
                + 0.6 * g_sat_jitter * sin(sat_phase * 0.71 + 1.2);
            let y_amp = radial * (0.25 + 0.45 * g_sat_jitter);
            let y_off = y_amp * sin(sat_phase * 1.29 + f32(hub_idx) * 0.83);
            let orbit_offset = vec3f(
                cos(theta) * radial,
                y_off,
                sin(theta) * radial,
            );
            let inhale = g_sat_activity * (
                0.5 + 0.5 * sin(sat_phase * 1.91 + g_beats * 0.37)
            );
            let sat_center = mix(hub + orbit_offset, hub, inhale);
            let sat_r = g_sat_radius
                * (1.0
                    + 0.35
                        * g_sat_breathe
                        * sin(sat_phase * 2.07 + g_beats * 0.91));
            let sat_d = length(p - sat_center) - max(sat_r, 0.01);
            let sat_blend = sat_blend_base * (0.7 + 0.3 * inhale);
            scene = smin(scene, sat_d, sat_blend);

            // Mucus-like strand: thin sticky connection.
            if (g_strand_strength > 0.0001) {
                let away = 1.0 - inhale;
                let strand_gate = smoothstep(0.05, 0.95, away);
                let strand_r = g_sat_radius
                    * mix(0.26, 0.07, g_strand_thinness)
                    * (0.35 + 0.65 * away);
                let strand_d = sd_capsule(
                    p,
                    hub,
                    sat_center,
                    max(strand_r, 0.002),
                );
                let strand_blend = strand_base * strand_gate;
                scene = smin(scene, strand_d, max(0.0001, strand_blend));
            }
        }
    }

    return scene;
}

fn split_blob_sdf(
    p: vec3f,
    center: vec3f,
    phase: f32,
    split: vec3f,
    strength: f32,
) -> f32 {
    let s = strength;
    let radius_scale = mix(1.0, 0.78, s);
    let split_a = blob_sdf_scaled(
        p,
        center + split,
        phase + 0.13,
        radius_scale,
    );
    let split_b = blob_sdf_scaled(
        p,
        center - split,
        phase - 0.17,
        radius_scale,
    );
    let neck_blend = max(0.0001, g_topology_blend * mix(1.0, 0.45, s));
    return smin(split_a, split_b, neck_blend);
}

fn blob_sdf(p: vec3f, center: vec3f, phase: f32) -> f32 {
    return blob_sdf_scaled(p, center, phase, 1.0);
}

fn blob_sdf_scaled(
    p: vec3f,
    center: vec3f,
    phase: f32,
    radius_scale: f32,
) -> f32 {
    let sphere_radius = params.d.y;
    let rel = p - center;
    let _ = phase;
    let r_mod = sphere_radius * max(radius_scale, 0.05);

    return length(rel) - max(r_mod, 0.02);
}

fn calc_normal(p: vec3f) -> vec3f {
    let complexity = shape_complexity();
    let e = mix(NORMAL_EPS, NORMAL_EPS * 2.5, complexity);
    let k1 = vec3f(1.0, -1.0, -1.0);
    let k2 = vec3f(-1.0, -1.0, 1.0);
    let k3 = vec3f(-1.0, 1.0, -1.0);
    let k4 = vec3f(1.0, 1.0, 1.0);
    let n = k1 * scene_sdf(p + k1 * e)
        + k2 * scene_sdf(p + k2 * e)
        + k3 * scene_sdf(p + k3 * e)
        + k4 * scene_sdf(p + k4 * e);
    return safe_normalize(n);
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

fn rotate_xz(v: vec3f, angle: f32) -> vec3f {
    let c = cos(angle);
    let s = sin(angle);
    return vec3f(c * v.x - s * v.z, v.y, s * v.x + c * v.z);
}

fn flow_field(p: vec3f, time: f32, scale: f32) -> vec3f {
    let q = p * scale;
    return vec3f(
        sin(q.y + time * 0.71) - cos(q.z * 1.17 - time * 0.37),
        sin(q.z + time * 0.53) - cos(q.x * 1.31 + time * 0.19),
        sin(q.x + time * 0.89) - cos(q.y * 1.11 - time * 0.43),
    );
}

fn curl_advection(p: vec3f, time: f32, scale: f32) -> vec3f {
    let e = 0.11;
    let ex = vec3f(e, 0.0, 0.0);
    let ey = vec3f(0.0, e, 0.0);
    let ez = vec3f(0.0, 0.0, e);
    let fx1 = flow_field(p + ex, time, scale);
    let fx2 = flow_field(p - ex, time, scale);
    let fy1 = flow_field(p + ey, time, scale);
    let fy2 = flow_field(p - ey, time, scale);
    let fz1 = flow_field(p + ez, time, scale);
    let fz2 = flow_field(p - ez, time, scale);
    let d_fz_dy = (fy1.z - fy2.z) / (2.0 * e);
    let d_fy_dz = (fz1.y - fz2.y) / (2.0 * e);
    let d_fx_dz = (fz1.x - fz2.x) / (2.0 * e);
    let d_fz_dx = (fx1.z - fx2.z) / (2.0 * e);
    let d_fy_dx = (fx1.y - fx2.y) / (2.0 * e);
    let d_fx_dy = (fy1.x - fy2.x) / (2.0 * e);
    let curl = vec3f(
        d_fz_dy - d_fy_dz,
        d_fx_dz - d_fz_dx,
        d_fy_dx - d_fx_dy,
    );
    return safe_normalize(curl);
}

fn sd_capsule(p: vec3f, a: vec3f, b: vec3f, r: f32) -> f32 {
    let ab = b - a;
    let ap = p - a;
    let denom = max(dot(ab, ab), 0.000001);
    let h = clamp(dot(ap, ab) / denom, 0.0, 1.0);
    return length(ap - ab * h) - r;
}

fn shape_complexity() -> f32 {
    let strands = STRAND_STRENGTH_FIXED * 0.18;
    let topology = params.q.x * 0.2;
    return strands + topology;
}

fn tone_map_filmic(color: vec3f) -> vec3f {
    let x = max(color - vec3f(0.004), vec3f(0.0));
    return (x * (6.2 * x + vec3f(0.5)))
        / (x * (6.2 * x + vec3f(1.7)) + vec3f(0.06));
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
    return c.z * mix(
        vec3f(1.0),
        clamp(p - vec3f(1.0), vec3f(0.0), vec3f(1.0)),
        c.y,
    );
}
