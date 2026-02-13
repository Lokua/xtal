// crosshatch_dev.wgsl
//
// Pencil cross-hatching post-process over a raymarched scene.
// Technique adapted from flockaroo (Shadertoy).
// https://www.shadertoy.com/view/MsKfRw

const MAX_STEPS: i32 = 80;
const MAX_DIST: f32 = 20.0;
const SURF_DIST: f32 = 0.001;
const NOISE_OCTAVES: i32 = 4;

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
    // show_shape, _, noise_amp, noise_freq
    e: vec4f,
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
    let rot_speed = params.b.y;
    let cam_z = params.b.z;
    let light_angle = params.b.w;
    let hatch_density = params.c.x;
    let hatch_layers = i32(params.c.y);
    let hatch_strength = params.c.z;
    let angle_spread = params.c.w;
    let paper_grain = params.d.z;
    let brightness_ctrl = params.d.w;
    let show_shape = params.e.x > 0.5;
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

    // Raymarch
    let result = ray_march(
        ro, rd, t, rot_speed, morph,
        noise_amp, noise_freq, aspect,
    );
    let hit_dist = result.x;
    let hit_pos = ro + rd * hit_dist;

    var scene_brightness = 1.0;

    if hit_dist < MAX_DIST {
        let n = calc_normal(
            hit_pos, t, rot_speed, morph,
            noise_amp, noise_freq, aspect,
        );
        let diff = max(dot(n, light), 0.0);
        let amb = 0.15;
        let ao = calc_ao(
            hit_pos, n, t, rot_speed, morph,
            noise_amp, noise_freq, aspect,
        );
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
    if show_shape {
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

            let rh = hash21(uvh);
            let h_val = 1.0
                - smoothstep(0.5, 1.5, rh + br)
                - 0.3 * abs(
                    hash21(fc * 0.7) - 0.5
                );
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

    // Paper texture (fully bypassed at grain = 0)
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

fn exploded_cluster_sdf(
    p: vec3f,
    t: f32,
    morph: f32,
    aspect: f32,
) -> f32 {
    let m = smoothstep(0.0, 1.0, morph);
    let core_radius = mix(0.8, 0.22, m);
    var d = length(p) - core_radius;
    let spread = mix(0.0, 1.45 + 0.55 * aspect, m);
    let blend_k = mix(0.42, 0.12, m);
    let strand_base = mix(0.36, 0.05, m);

    for (var i = 0; i < 8; i++) {
        let fi = f32(i);
        let seed = hash31(vec3f(fi * 1.7, fi * 3.1, 9.1));
        let pulse = 0.08 * m * sin(t * 0.43 + fi * 1.31);
        let axis = blob_dir(i);
        let axis_stretch = vec3f(
            axis.x * (1.0 + (aspect - 1.0) * 0.75),
            axis.y,
            axis.z,
        );
        let dir = normalize(axis_stretch);
        let dist = spread * (0.72 + 0.34 * seed) + pulse;
        let center = dir * dist;
        let blob_radius = 0.18 + 0.09 * seed + 0.05 * pulse;
        let blob = length(p - center) - blob_radius;

        d = smooth_min(d, blob, blend_k);

        let strand_radius = strand_base * (0.8 + 0.35 * (1.0 - seed));
        let strand = sd_capsule(
            p,
            vec3f(0.0),
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
    noise_amp: f32,
    noise_freq: f32,
    aspect: f32,
) -> f32 {
    let rp = rotate_y(
        rotate_x(p, t * rot_speed * 0.7),
        t * rot_speed,
    );
    let m = smoothstep(0.0, 1.0, morph);
    let sphere = length(rp) - 0.8;
    let cluster = exploded_cluster_sdf(rp, t, morph, aspect);
    var d = mix(sphere, cluster, m);

    // Noise displacement for surface detail
    let np = rp * noise_freq;
    let n = fbm3(
        np + vec3f(t * 0.1, 0.0, t * 0.07),
    );
    d += (n - 0.5) * noise_amp * mix(0.45, 1.0, m);

    return d;
}

fn ray_march(
    ro: vec3f,
    rd: vec3f,
    t: f32,
    rot_speed: f32,
    morph: f32,
    noise_amp: f32,
    noise_freq: f32,
    aspect: f32,
) -> vec2f {
    var d = 0.0;
    for (var i = 0; i < MAX_STEPS; i++) {
        let p = ro + rd * d;
        let ds = scene_sdf(
            p, t, rot_speed, morph,
            noise_amp, noise_freq, aspect,
        );
        d += ds * 0.8;
        if abs(ds) < SURF_DIST || d > MAX_DIST {
            break;
        }
    }
    return vec2f(d, 0.0);
}

fn calc_normal(
    p: vec3f,
    t: f32,
    rot_speed: f32,
    morph: f32,
    noise_amp: f32,
    noise_freq: f32,
    aspect: f32,
) -> vec3f {
    let e = vec2f(0.001, 0.0);
    let d = scene_sdf(
        p, t, rot_speed, morph,
        noise_amp, noise_freq, aspect,
    );
    let n = vec3f(
        scene_sdf(
            p + e.xyy, t, rot_speed, morph,
            noise_amp, noise_freq, aspect,
        ) - d,
        scene_sdf(
            p + e.yxy, t, rot_speed, morph,
            noise_amp, noise_freq, aspect,
        ) - d,
        scene_sdf(
            p + e.yyx, t, rot_speed, morph,
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
    noise_amp: f32,
    noise_freq: f32,
    aspect: f32,
) -> f32 {
    var occ = 0.0;
    var w = 1.0;
    for (var i = 0; i < 5; i++) {
        let h = 0.01 + 0.12 * f32(i) / 4.0;
        let d = scene_sdf(
            p + n * h, t, rot_speed, morph,
            noise_amp, noise_freq, aspect,
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
