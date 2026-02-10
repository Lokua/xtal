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
    // d: reserved, sphere_radius, blend_k, reserved
    d: vec4f,
    // e: hue_shift, saturation, contrast, debug_view
    e: vec4f,
    // f: harmonic_amp_1, harmonic_freq_1, harmonic_amp_2, harmonic_freq_2
    f: vec4f,
    // g: harmonic_warp, harmonic_ridge, harmonic_phase, stretch_y
    g: vec4f,
    // h: light_x, light_y, light_z, light_intensity
    h: vec4f,
    // i: shadow_strength, shadow_softness, shadow_legacy_mode, reserved
    i: vec4f,
    // j: rim_strength, rim_power, emissive_strength, spec_power
    j: vec4f,
    // k: reserved, triangle_size, triangle_rotation, reserved
    k: vec4f,
    // l: motion_speed, motion_amount, blend_pulse_amount, blend_pulse_freq
    l: vec4f,
    // m: energy_strength, energy_power, energy_freq, chroma_strength
    m: vec4f,
    // n: satellite_count, satellite_radius, satellite_orbit, satellite_activity
    n: vec4f,
    // o: satellite_speed, satellite_merge, satellite_jitter, satellite_breathe
    o: vec4f,
    // p: flow_amount, flow_scale, strand_strength, strand_thinness
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
const SATELLITE_MAX: i32 = 9;

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
var<private> g_flow_amount: f32;
var<private> g_flow_scale: f32;
var<private> g_strand_strength: f32;
var<private> g_strand_thinness: f32;

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
    let debug_view = i32(params.e.w);
    let light_pos = vec3f(params.h.x, params.h.y, params.h.z);
    let light_intensity = max(params.h.w, 0.0);
    let shadow_strength = clamp(params.i.x, 0.0, 1.0);
    let shadow_softness = max(params.i.y, 0.0001);
    let shadow_legacy_mode = params.i.z > 0.5;
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
    prepare_scene_state(beats);

    let t = ray_march(
        ro,
        rd,
        max_steps,
        surf_eps,
    );
    if (t >= MAX_DIST) {
        return vec4f(bg, 1.0);
    }

    let hit_p = ro + rd * t;
    let n = calc_normal(hit_p);
    let shading_bias = surf_eps * (1.2 + 1.2 * complexity);
    let p = hit_p + n * shading_bias;
    let l = normalize(light_pos - p);
    let light_dist = length(light_pos - p);
    let v = normalize(ro - p);
    let h = normalize(l + v);
    var shadow = 1.0;
    if (shadow_strength > 0.0001) {
        shadow = soft_shadow(
            p,
            l,
            max(shading_bias, surf_eps * 2.0),
            light_dist,
            shadow_softness,
            surf_eps,
            shadow_legacy_mode,
        );
    }
    let shadow_mix = mix(1.0, shadow, shadow_strength);
    let ao = ambient_occlusion(p, n, surf_eps);

    let diff = max(dot(n, l), 0.0) * shadow_mix;
    let spec = pow(max(dot(n, h), 0.0), spec_power) * shadow_mix;
    let fresnel = pow(1.0 - max(dot(n, v), 0.0), 3.0);
    let rim = pow(1.0 - max(dot(n, v), 0.0), rim_power) * rim_strength;
    if (debug_view == 1) {
        return vec4f(n * 0.5 + vec3f(0.5), 1.0);
    }
    if (debug_view == 2) {
        return vec4f(vec3f(shadow_mix), 1.0);
    }
    if (debug_view == 3) {
        return vec4f(vec3f(ao), 1.0);
    }
    if (debug_view == 4) {
        return vec4f(vec3f(diff), 1.0);
    }
    if (debug_view == 5) {
        return vec4f(vec3f(spec), 1.0);
    }
    if (debug_view == 6) {
        return vec4f(vec3f(rim), 1.0);
    }
    if (debug_view == 7) {
        return vec4f(vec3f(fresnel), 1.0);
    }

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
) -> f32 {
    var dist = 0.0;
    for (var i = 0; i < MAX_MARCH_STEPS_CAP; i = i + 1) {
        if (i >= max_steps) {
            break;
        }
        let p = ro + rd * dist;
        let scene_dist = scene_sdf(p);
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

fn prepare_scene_state(beats: f32) {
    g_beats = beats;
    let motion_speed = params.l.x;
    g_motion_amount = params.l.y;
    let shape_phase = params.g.z;

    let blend_pulse_amount = params.l.z;
    let blend_pulse_freq = params.l.w;
    g_sat_count = i32(round(clamp(params.n.x, 0.0, f32(SATELLITE_MAX))));
    g_sat_radius = max(params.n.y, 0.0);
    g_sat_orbit = max(params.n.z, 0.0);
    g_sat_activity = clamp(params.n.w, 0.0, 1.0);
    g_sat_speed = max(params.o.x, 0.0);
    g_sat_merge = clamp(params.o.y, 0.0, 1.0);
    g_sat_jitter = clamp(params.o.z, 0.0, 1.0);
    g_sat_breathe = clamp(params.o.w, 0.0, 1.0);
    g_flow_amount = clamp(params.p.x, 0.0, 1.0);
    g_flow_scale = max(params.p.y, 0.05);
    g_strand_strength = clamp(params.p.z, 0.0, 1.0);
    g_strand_thinness = clamp(params.p.w, 0.0, 1.0);
    g_phase = shape_phase + beats * motion_speed;

    let stretch_amount = params.g.w;
    let harmonic_amp_sum = abs(params.f.x) + abs(params.f.z);
    let ridge = clamp(params.g.y, 0.0, 1.0);
    let harmonic_bound = pow(
        max(harmonic_amp_sum, 0.00001),
        mix(1.0, 0.35, ridge),
    );
    g_blob_radius_bound = max(params.d.y, 0.02) * (
        1.0
            + 0.45 * abs(stretch_amount)
            + harmonic_bound
    );
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
        g_c1 += flow_amt * curl_advection(g_c1 + vec3f(0.7, 1.1, -0.4), flow_t, g_flow_scale);
        g_c2 += flow_amt * curl_advection(g_c2 + vec3f(-0.9, 0.3, 0.8), flow_t + 1.7, g_flow_scale);
        g_c3 += flow_amt * curl_advection(g_c3 + vec3f(0.2, -1.2, 1.4), flow_t + 3.1, g_flow_scale);
    }

    g_blend = max(
        params.d.z + blend_pulse_amount * sin(beats * blend_pulse_freq),
        0.0001,
    );
    g_outer_blend = max(
        0.0001,
        g_blend * (0.35 + 0.65 * clamp(g_motion_amount, 0.0, 1.0)),
    );
}

fn scene_sdf(p: vec3f) -> f32 {
    // Conservative broad-phase bound. If far enough, return early and skip
    // expensive harmonic/satellite evaluation while preserving visual result.
    let d1_bound = length(p - g_c1) - g_blob_radius_bound;
    let d2_bound = length(p - g_c2) - g_blob_radius_bound;
    let d3_bound = length(p - g_c3) - g_blob_radius_bound;
    var scene_bound = min(min(d1_bound, d2_bound), d3_bound) - g_smooth_pad;
    if (g_sat_count > 0 && g_sat_radius > 0.0 && g_sat_orbit > 0.0) {
        let sat_orbit_bound = g_sat_orbit * (1.0 + 0.45 * g_sat_jitter);
        let sat_radius_bound = g_sat_radius * (1.0 + 0.35 * g_sat_breathe);
        let sat_cluster_bound = sat_orbit_bound + sat_radius_bound + g_smooth_pad;
        let sat1_bound = length(p - g_c1) - sat_cluster_bound;
        let sat2_bound = length(p - g_c2) - sat_cluster_bound;
        let sat3_bound = length(p - g_c3) - sat_cluster_bound;
        scene_bound = min(scene_bound, min(min(sat1_bound, sat2_bound), sat3_bound));
    }
    if (scene_bound > 0.35) {
        return scene_bound;
    }

    let d1 = blob_sdf(p, g_c1, g_phase + 0.0);
    let d2 = blob_sdf(p, g_c2, g_phase + 2.1);
    let d3 = blob_sdf(p, g_c3, g_phase + 4.2);

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

            let sat_phase = g_phase * g_sat_speed + f32(i) * 1.947 + f32(lane) * 2.713;
            let radial = g_sat_orbit * (
                1.0 + 0.45 * g_sat_jitter * sin(sat_phase * 1.63 + f32(hub_idx) * 1.9)
            );
            let theta = sat_phase + 0.6 * g_sat_jitter * sin(sat_phase * 0.71 + 1.2);
            let y_amp = radial * (0.25 + 0.45 * g_sat_jitter);
            let y_off = y_amp * sin(sat_phase * 1.29 + f32(hub_idx) * 0.83);
            let orbit_offset = vec3f(cos(theta) * radial, y_off, sin(theta) * radial);
            let inhale = g_sat_activity * (
                0.5 + 0.5 * sin(sat_phase * 1.91 + g_beats * 0.37)
            );
            let sat_center = mix(hub + orbit_offset, hub, inhale);
            let sat_r = g_sat_radius * (
                1.0 + 0.35 * g_sat_breathe * sin(sat_phase * 2.07 + g_beats * 0.91)
            );
            let sat_d = length(p - sat_center) - max(sat_r, 0.01);
            let sat_blend = sat_blend_base * (0.7 + 0.3 * inhale);
            scene = smin(scene, sat_d, sat_blend);

            // Mucus-like strand: thin, sticky connection as satellites pull away.
            if (g_strand_strength > 0.0001) {
                let away = clamp(1.0 - inhale, 0.0, 1.0);
                let strand_gate = smoothstep(0.05, 0.95, away);
                let strand_r = g_sat_radius
                    * mix(0.26, 0.07, g_strand_thinness)
                    * (0.35 + 0.65 * away);
                let strand_d = sd_capsule(p, hub, sat_center, max(strand_r, 0.002));
                let strand_blend = strand_base * strand_gate;
                scene = smin(scene, strand_d, max(0.0001, strand_blend));
            }
        }
    }

    return scene;
}

fn blob_sdf(p: vec3f, center: vec3f, phase: f32) -> f32 {
    let sphere_radius = params.d.y;
    let rel = p - center;
    let dir = safe_normalize(rel);

    let stretch_amount = params.g.w;
    let stretch_term = 0.45 * stretch_amount
        * (dir.y * dir.y - 0.5 * (dir.x * dir.x + dir.z * dir.z));
    let bump = harmonic_field(dir, phase);
    let r_mod = sphere_radius * (
        1.0
            + stretch_term
            + bump
    );

    return length(rel) - max(r_mod, 0.02);
}

fn calc_normal(p: vec3f) -> vec3f {
    let complexity = shape_complexity();
    let e = mix(NORMAL_EPS, NORMAL_EPS * 2.5, complexity);
    let nx = scene_sdf(p + vec3f(e, 0.0, 0.0))
        - scene_sdf(p - vec3f(e, 0.0, 0.0));
    let ny = scene_sdf(p + vec3f(0.0, e, 0.0))
        - scene_sdf(p - vec3f(0.0, e, 0.0));
    let nz = scene_sdf(p + vec3f(0.0, 0.0, e))
        - scene_sdf(p - vec3f(0.0, 0.0, e));
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

fn harmonic_field(dir: vec3f, phase: f32) -> f32 {
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
    return field;
}

fn ridge_shape(x: f32, ridge: f32) -> f32 {
    let power = mix(1.0, 0.35, ridge);
    return sign(x) * pow(max(abs(x), 0.00001), power);
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

fn soft_shadow(
    ro: vec3f,
    rd: vec3f,
    min_t: f32,
    max_t: f32,
    softness: f32,
    surf_eps: f32,
    legacy_mode: bool,
) -> f32 {
    var result = 1.0;
    var t = min_t;
    var prev_h = 1.0;
    for (var i = 0; i < MAX_SHADOW_STEPS; i = i + 1) {
        if (legacy_mode) {
            let h_raw = scene_sdf(ro + rd * t);
            if (h_raw < surf_eps * 0.4) {
                return 0.0;
            }
            let h = mix(h_raw, prev_h, 0.22);
            if (h < surf_eps * 0.6) {
                break;
            }
            result = min(result, softness * h / max(t, 0.02));
            prev_h = h;
            t += clamp(h * 0.75, surf_eps * 0.3, 0.18);
        } else {
            let h = scene_sdf(ro + rd * t);
            if (h < surf_eps * 0.8) {
                break;
            }
            result = min(result, softness * h / max(t, 0.02));
            // Slightly denser stepping reduces contour-like bands on rippled SDFs.
            t += clamp(h * 0.6, surf_eps * 0.4, 0.12);
        }
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
) -> f32 {
    var occ = 0.0;
    var scale = 1.0;
    let ao_bias = surf_eps * 2.0;
    for (var i = 1; i <= MAX_AO_SAMPLES; i = i + 1) {
        let h = AO_STEP * f32(i);
        let d = scene_sdf(p + n * (h + ao_bias));
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
    let stretch = abs(params.g.w) * 0.35;
    let strands = params.p.z * 0.18;
    return clamp(h1 + h2 + ridge + warp + stretch + strands, 0.0, 1.0);
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
