// Minimal voxel sketch with performance-focused shading and terrain variance.

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, beats, bg_gray
    a: vec4f,
    // height_scale, camera_height, orbit_amount, time_scale
    b: vec4f,
    // voxel_size, step_scale, top_shade, side_shade
    c: vec4f,
    // terrain_freq, warp_amount, warp_freq, quantize_levels
    d: vec4f,
    // ao_amount, fog_density, fog_start, detail_fade_density
    e: vec4f,
    // horizon_start, horizon_end, focal_len, orbit_radius
    f: vec4f,
}

struct SceneSample {
    dist: f32,
    cell: vec2f,
    height: f32,
}

@group(0) @binding(0)
var<uniform> params: Params;

const EPSILON: f32 = 0.001;
const MIN_STEP: f32 = 0.01;
const MARCH_STEPS: i32 = 96;
const MAX_DISTANCE: f32 = 48.0;
const LOOK_AT_Y: f32 = 1.2;
const TAU: f32 = 6.28318530718;
const HUGE: f32 = 1e9;
const OCEAN_FREQ_MULT: f32 = 0.33;
const OCEAN_AMOUNT: f32 = 0.32;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4f(vert.position, 0.0, 1.0);
    out.pos = vert.position;
    return out;
}

fn sdf_box(p: vec3f, half_extents: vec3f) -> f32 {
    let q = abs(p) - half_extents;
    let outside = length(max(q, vec3f(0.0)));
    let inside = min(max(q.x, max(q.y, q.z)), 0.0);
    return outside + inside;
}

fn hash21(p: vec2f) -> f32 {
    let h = dot(p, vec2f(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

fn tower_height(
    cell: vec2f,
    time: f32,
    height_scale: f32,
    terrain_freq: f32,
    warp_amount: f32,
    warp_freq: f32,
    quantize_levels: f32,
    detail_mix: f32,
) -> f32 {
    let rand_a = hash21(cell + vec2f(17.0, 3.0));
    let rand_b = hash21(cell + vec2f(5.0, 29.0));
    let phase = dot(cell, vec2f(0.71, -0.47)) * terrain_freq * warp_freq
        + time * (0.08 + 0.22 * rand_b);
    let warp = vec2f(
        sin(phase + rand_a * TAU),
        cos(phase * 1.19 - rand_b * TAU),
    ) * warp_amount;
    let warped_cell = cell + warp;

    let angle = rand_a * TAU;
    let dir = vec2f(cos(angle), sin(angle));
    let ocean_speed = 0.1 + 0.22 * rand_b;
    let ocean_phase = dot(warped_cell, dir) * terrain_freq * OCEAN_FREQ_MULT
        + time * ocean_speed;
    let ocean_wave = sin(ocean_phase + rand_b * TAU);

    let wave_a = sin(
        (warped_cell.x + warped_cell.y * 0.35) * terrain_freq + time * 0.55,
    );
    let wave_b = cos(
        (warped_cell.y - warped_cell.x * 0.21) * terrain_freq * 1.73
            - time * 0.31,
    );
    let ridge = abs(sin(
        (warped_cell.x * 0.47 - warped_cell.y * 0.63) * terrain_freq
            + time * 0.12,
    ));
    let jitter = hash21(cell) - 0.5;

    let base_shape = 0.72 * wave_a + 0.1 * wave_b;
    let detail_shape = 0.2 * wave_b
        + 0.23 * ridge
        + 0.35 * jitter
        + OCEAN_AMOUNT * ocean_wave;
    let shape = base_shape + detail_shape * detail_mix;

    let raw_height = max(0.1, (1.05 + shape) * height_scale);
    let levels = max(1.0, quantize_levels);
    let quantized = floor(raw_height * levels + 0.5) / levels;
    let quantize_mix = select(0.0, 1.0, levels > 1.5);
    return mix(raw_height, quantized, quantize_mix);
}

fn scene_sample(
    p: vec3f,
    time: f32,
    height_scale: f32,
    voxel_size: f32,
    terrain_freq: f32,
    warp_amount: f32,
    warp_freq: f32,
    quantize_levels: f32,
    detail_mix: f32,
) -> SceneSample {
    let cell = floor(p.xz / voxel_size);
    let center_xz = (cell + 0.5) * voxel_size;
    let h = tower_height(
        cell,
        time,
        height_scale,
        terrain_freq,
        warp_amount,
        warp_freq,
        quantize_levels,
        detail_mix,
    );
    let local = p - vec3f(center_xz.x, h * 0.5, center_xz.y);
    let half_extents = vec3f(voxel_size * 0.5, h * 0.5, voxel_size * 0.5);
    let d = sdf_box(local, half_extents);
    return SceneSample(d, cell, h);
}

fn next_cell_boundary_t(p: vec3f, ray_dir: vec3f, voxel_size: f32) -> f32 {
    let cell = floor(p.xz / voxel_size);

    var tx = HUGE;
    if (abs(ray_dir.x) > 1e-5) {
        var next_x = cell.x * voxel_size;
        if (ray_dir.x > 0.0) {
            next_x = (cell.x + 1.0) * voxel_size;
        }
        let t = (next_x - p.x) / ray_dir.x;
        if (t > 0.0) {
            tx = t;
        }
    }

    var tz = HUGE;
    if (abs(ray_dir.z) > 1e-5) {
        var next_z = cell.y * voxel_size;
        if (ray_dir.z > 0.0) {
            next_z = (cell.y + 1.0) * voxel_size;
        }
        let t = (next_z - p.z) / ray_dir.z;
        if (t > 0.0) {
            tz = t;
        }
    }

    return min(tx, tz);
}

fn face_shade(
    hit_pos: vec3f,
    sample: SceneSample,
    voxel_size: f32,
    top_shade: f32,
    side_shade: f32,
) -> f32 {
    let center_xz = (sample.cell + 0.5) * voxel_size;
    let center = vec3f(center_xz.x, sample.height * 0.5, center_xz.y);
    let local = hit_pos - center;
    let half_extents = vec3f(
        voxel_size * 0.5,
        sample.height * 0.5,
        voxel_size * 0.5,
    );

    let nx = abs(local.x) / max(half_extents.x, EPSILON);
    let ny = abs(local.y) / max(half_extents.y, EPSILON);
    let nz = abs(local.z) / max(half_extents.z, EPSILON);

    if (ny >= nx && ny >= nz && local.y > 0.0) {
        return top_shade;
    }
    return side_shade;
}

fn ambient_occlusion(
    hit_pos: vec3f,
    sample: SceneSample,
    time: f32,
    ao_amount: f32,
    height_scale: f32,
    terrain_freq: f32,
    warp_amount: f32,
    warp_freq: f32,
    quantize_levels: f32,
    detail_mix: f32,
    voxel_size: f32,
) -> f32 {
    let center_xz = (sample.cell + 0.5) * voxel_size;
    let center = vec3f(center_xz.x, sample.height * 0.5, center_xz.y);
    let local = hit_pos - center;
    let half_h = max(sample.height * 0.5, EPSILON);
    let y01 = clamp((local.y + half_h) / (2.0 * half_h), 0.0, 1.0);

    let h_xp = tower_height(
        sample.cell + vec2f(1.0, 0.0),
        time,
        height_scale,
        terrain_freq,
        warp_amount,
        warp_freq,
        quantize_levels,
        detail_mix,
    );
    let h_xn = tower_height(
        sample.cell + vec2f(-1.0, 0.0),
        time,
        height_scale,
        terrain_freq,
        warp_amount,
        warp_freq,
        quantize_levels,
        detail_mix,
    );
    let h_zp = tower_height(
        sample.cell + vec2f(0.0, 1.0),
        time,
        height_scale,
        terrain_freq,
        warp_amount,
        warp_freq,
        quantize_levels,
        detail_mix,
    );
    let h_zn = tower_height(
        sample.cell + vec2f(0.0, -1.0),
        time,
        height_scale,
        terrain_freq,
        warp_amount,
        warp_freq,
        quantize_levels,
        detail_mix,
    );

    let neighbor_avg = (h_xp + h_xn + h_zp + h_zn) * 0.25;
    let valley = clamp(
        (neighbor_avg - sample.height) / max(height_scale + 0.1, 0.1),
        0.0,
        1.0,
    );
    let vertical_ao = mix(1.0 - 0.35 * ao_amount, 1.0, y01);
    let valley_ao = 1.0 - 0.6 * ao_amount * valley;
    return clamp(vertical_ao * valley_ao, 0.0, 1.0);
}

@fragment
fn fs_main(@location(0) position: vec2f) -> @location(0) vec4f {
    let res = vec2f(max(params.a.x, 1.0), max(params.a.y, 1.0));
    let aspect = res.x / res.y;
    let beats = params.a.z;

    let bg_gray = clamp(params.a.w, 0.0, 1.0);
    let height_scale = max(params.b.x, 0.05);
    let camera_height = max(params.b.y, 0.5);
    let orbit_amount = params.b.z;
    let time_scale = params.b.w;

    let voxel_size = max(params.c.x, 0.05);
    let step_scale = clamp(params.c.y, 0.1, 1.0);
    let top_shade = clamp(params.c.z, 0.0, 1.25);
    let side_shade = clamp(params.c.w, 0.0, 1.25);

    let terrain_freq = max(params.d.x, 0.0);
    let warp_amount = max(params.d.y, 0.0);
    let warp_freq = max(params.d.z, 0.0);
    let quantize_levels = max(params.d.w, 1.0);

    let ao_amount = clamp(params.e.x, 0.0, 1.0);
    let fog_density = max(params.e.y, 0.0);
    let fog_start = max(params.e.z, 0.0);
    let detail_fade_density = max(params.e.w, 0.0);

    let horizon_start = max(params.f.x, 0.0);
    let horizon_end = max(params.f.y, horizon_start + 0.001);
    let focal_len = max(params.f.z, 0.1);
    let orbit_radius = max(params.f.w, 0.5);

    let time = beats * time_scale;
    let bg = vec3f(bg_gray);

    let orbit = orbit_amount * beats * 0.35;
    let camera_pos = vec3f(
        sin(orbit) * orbit_radius,
        camera_height,
        cos(orbit) * orbit_radius,
    );
    let look_at = vec3f(0.0, LOOK_AT_Y, 0.0);

    let forward = normalize(look_at - camera_pos);
    let right = normalize(cross(vec3f(0.0, 1.0, 0.0), forward));
    let up = cross(forward, right);

    let uv = vec2f(position.x * aspect, position.y);
    let ray_dir = normalize(forward * focal_len + uv.x * right + uv.y * up);

    var t = 0.0;
    var hit = false;

    for (var step_index = 0; step_index < MARCH_STEPS; step_index += 1) {
        if (t > MAX_DISTANCE) {
            break;
        }

        let p = camera_pos + ray_dir * t;
        let detail_mix = exp(-t * detail_fade_density);
        let sample = scene_sample(
            p,
            time,
            height_scale,
            voxel_size,
            terrain_freq,
            warp_amount,
            warp_freq,
            quantize_levels,
            detail_mix,
        );

        if (sample.dist < EPSILON) {
            hit = true;
            break;
        }

        let next_boundary = next_cell_boundary_t(p, ray_dir, voxel_size);
        let sdf_step = max(sample.dist * step_scale, MIN_STEP);
        let safe_boundary_step = max(next_boundary * 0.999, MIN_STEP);
        t += min(sdf_step, safe_boundary_step);
    }

    if (!hit) {
        return vec4f(bg, 1.0);
    }

    let hit_pos = camera_pos + ray_dir * t;
    let detail_mix = exp(-t * detail_fade_density);
    let hit_sample = scene_sample(
        hit_pos,
        time,
        height_scale,
        voxel_size,
        terrain_freq,
        warp_amount,
        warp_freq,
        quantize_levels,
        detail_mix,
    );
    let shade = face_shade(
        hit_pos,
        hit_sample,
        voxel_size,
        top_shade,
        side_shade,
    );
    let ao = ambient_occlusion(
        hit_pos,
        hit_sample,
        time,
        ao_amount,
        height_scale,
        terrain_freq,
        warp_amount,
        warp_freq,
        quantize_levels,
        detail_mix,
        voxel_size,
    );

    let fog_dist = max(t - fog_start, 0.0);
    let fog = exp(-fog_dist * fog_density);
    let box_color = vec3f(shade * ao);
    var color = mix(bg, box_color, fog);

    let horizon = smoothstep(horizon_start, horizon_end, t);
    color = mix(color, bg, horizon);
    return vec4f(color, 1.0);
}
