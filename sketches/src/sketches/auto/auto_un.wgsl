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
    // c: cam_distance, cam_height, focal_len, fog_density
    c: vec4f,
    // d: sphere_offset, sphere_radius, blend_k, wobble_amount
    d: vec4f,
    // e: hue_shift, saturation, contrast, reserved
    e: vec4f,
    // f: harmonic_amp_1, harmonic_freq_1, harmonic_amp_2, harmonic_freq_2
    f: vec4f,
    // g: harmonic_warp, harmonic_ridge, harmonic_phase, reserved
    g: vec4f,
    // h: light_x, light_y, light_z, light_intensity
    h: vec4f,
    // i: shadow_strength, shadow_softness, ao_strength, ao_step
    i: vec4f,
    // j: rim_strength, rim_power, emissive_strength, spec_power
    j: vec4f,
    // k-l: reserved
    k: vec4f,
    l: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

const MAX_MARCH_STEPS_CAP: i32 = 256;
const MAX_SHADOW_STEPS: i32 = 64;
const MAX_AO_SAMPLES: i32 = 6;
const MAX_DIST: f32 = 30.0;
const SURF_DIST: f32 = 0.0012;
const NORMAL_EPS: f32 = 0.0012;

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
    let color_mix = clamp(params.a.w, 0.0, 1.0);
    let hue_shift = params.e.x;
    let saturation = max(params.e.y, 0.0);
    let contrast = max(params.e.z, 0.0);
    let light_pos = vec3f(params.h.x, params.h.y, params.h.z);
    let light_intensity = max(params.h.w, 0.0);
    let shadow_strength = clamp(params.i.x, 0.0, 1.0);
    let shadow_softness = max(params.i.y, 0.0001);
    let ao_strength = max(params.i.z, 0.0);
    let ao_step = max(params.i.w, 0.0001);
    let rim_strength = max(params.j.x, 0.0);
    let rim_power = max(params.j.y, 0.0001);
    let emissive_strength = max(params.j.z, 0.0);
    let spec_power = max(params.j.w, 1.0);

    let max_steps = i32(round(clamp(
        params.b.x,
        1.0,
        f32(MAX_MARCH_STEPS_CAP),
    )));

    let cam_dist = max(params.c.x, 0.1);
    let cam_height = params.c.y;
    let focal_len = max(params.c.z, 0.01);
    let fog_density = max(params.c.w, 0.000001);

    let ro = vec3f(0.0, cam_height, -cam_dist);
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
    );
    if (t >= MAX_DIST) {
        return vec4f(bg, 1.0);
    }

    let p = ro + rd * t;
    let n = calc_normal(p);
    let l = normalize(light_pos - p);
    let light_dist = length(light_pos - p);
    let v = normalize(ro - p);
    let h = normalize(l + v);
    let shadow = soft_shadow(
        p + n * SURF_DIST * 3.0,
        l,
        0.01,
        light_dist,
        shadow_softness,
    );
    let shadow_mix = mix(1.0, shadow, shadow_strength);
    let ao = ambient_occlusion(p, n, ao_step, ao_strength);

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

    let fog = exp(-fog_density * t * t);
    color = mix(bg, color, fog);
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
) -> f32 {
    var dist = 0.0;
    for (var i = 0; i < MAX_MARCH_STEPS_CAP; i = i + 1) {
        if (i >= max_steps) {
            break;
        }
        let p = ro + rd * dist;
        let scene_dist = scene_sdf(p);
        dist += scene_dist;
        if (scene_dist < SURF_DIST || dist >= MAX_DIST) {
            break;
        }
    }
    return dist;
}

fn scene_sdf(p: vec3f) -> f32 {
    let sphere_offset = params.d.x;
    let sphere_radius = params.d.y;
    let blend_k = max(params.d.z, 0.0001);
    let wobble_amt = params.d.w;
    let q = p;

    let c1 = vec3f(
        -sphere_offset,
        0.35 * wobble_amt,
        0.0,
    );
    let r1 = sphere_radius + 0.15 * wobble_amt;
    let rel1 = q - c1;
    let dir1 = safe_normalize(rel1);
    let r1_mod = max(
        r1 + sphere_radius * harmonic_displacement(dir1, params.g.z),
        0.02,
    );
    let d1 = length(rel1) - r1_mod;

    let c2 = vec3f(
        sphere_offset,
        -0.35 * wobble_amt,
        0.0,
    );
    let r2 = sphere_radius - 0.15 * wobble_amt;
    let rel2 = q - c2;
    let dir2 = safe_normalize(rel2);
    let r2_mod = max(
        r2 + sphere_radius * harmonic_displacement(dir2, -params.g.z),
        0.02,
    );
    let d2 = length(rel2) - r2_mod;

    return smin(d1, d2, blend_k);
}

fn calc_normal(p: vec3f) -> vec3f {
    let e = NORMAL_EPS;
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

fn harmonic_displacement(dir: vec3f, phase: f32) -> f32 {
    let amp1 = params.f.x;
    let freq1 = max(params.f.y, 0.0);
    let amp2 = params.f.z;
    let freq2 = max(params.f.w, 0.0);
    let warp = params.g.x;
    let ridge = clamp(params.g.y, 0.0, 1.0);

    let theta = atan2(dir.z, dir.x);
    let phi = acos(clamp(dir.y, -1.0, 1.0));

    var h1 = sin(theta * freq1 + warp * dir.y + phase)
        * cos(phi * (0.5 * freq1 + 1.0));
    var h2 = sin(theta * freq2 - phi * (0.7 * freq2 + 1.0) - phase)
        * cos(phi * 0.5 + warp * dir.x);

    h1 = ridge_shape(h1, ridge);
    h2 = ridge_shape(h2, ridge);

    return amp1 * h1 + amp2 * h2;
}

fn ridge_shape(x: f32, ridge: f32) -> f32 {
    let power = mix(1.0, 0.35, ridge);
    return sign(x) * pow(max(abs(x), 0.00001), power);
}

fn soft_shadow(
    ro: vec3f,
    rd: vec3f,
    min_t: f32,
    max_t: f32,
    softness: f32,
) -> f32 {
    var result = 1.0;
    var t = min_t;
    for (var i = 0; i < MAX_SHADOW_STEPS; i = i + 1) {
        let h = scene_sdf(ro + rd * t);
        if (h < SURF_DIST) {
            return 0.0;
        }
        result = min(result, softness * h / t);
        t += clamp(h, 0.01, 0.25);
        if (t > max_t) {
            break;
        }
    }
    return clamp(result, 0.0, 1.0);
}

fn ambient_occlusion(
    p: vec3f,
    n: vec3f,
    step_size: f32,
    strength: f32,
) -> f32 {
    var occ = 0.0;
    var scale = 1.0;
    for (var i = 1; i <= MAX_AO_SAMPLES; i = i + 1) {
        let h = step_size * f32(i);
        let d = scene_sdf(p + n * h);
        occ += max(h - d, 0.0) * scale;
        scale *= 0.75;
    }
    return clamp(1.0 - occ * strength, 0.0, 1.0);
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
