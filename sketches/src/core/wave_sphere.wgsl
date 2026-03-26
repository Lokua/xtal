// Forked form https://www.shadertoy.com/view/WX3cR4
const PI: f32 = 3.141592653589793;
const NUM_COLORS: u32 = 14u;
const MAX_STEPS: i32 = 96;
const MIN_DT: f32 = 0.003;
const MAX_CONTRIB: f32 = 24.0;
const CAM_SAFE_CLEARANCE: f32 = 0.6;

const BLUE: vec3f = vec3f(47.0, 75.0, 162.0) / 255.0;
const PINK: vec3f = vec3f(233.0, 71.0, 245.0) / 255.0;
const PURPLE: vec3f = vec3f(128.0, 63.0, 224.0) / 255.0;
const CYAN: vec3f = vec3f(61.0, 199.0, 220.0) / 255.0;
const MAGENTA: vec3f = vec3f(222.0, 51.0, 150.0) / 255.0;
const LIME: vec3f = vec3f(160.0, 220.0, 70.0) / 255.0;
const ORANGE: vec3f = vec3f(245.0, 140.0, 60.0) / 255.0;
const TEAL: vec3f = vec3f(38.0, 178.0, 133.0) / 255.0;
const RED: vec3f = vec3f(220.0, 50.0, 50.0) / 255.0;
const YELLOW: vec3f = vec3f(240.0, 220.0, 80.0) / 255.0;
const VIOLET: vec3f = vec3f(180.0, 90.0, 240.0) / 255.0;
const AQUA: vec3f = vec3f(80.0, 210.0, 255.0) / 255.0;
const FUCHSIA: vec3f = vec3f(245.0, 80.0, 220.0) / 255.0;
const GREEN: vec3f = vec3f(70.0, 200.0, 100.0) / 255.0;

const COLORS: array<vec3f, NUM_COLORS> = array<vec3f, NUM_COLORS>(
    BLUE,
    PINK,
    PURPLE,
    CYAN,
    MAGENTA,
    LIME,
    ORANGE,
    TEAL,
    RED,
    YELLOW,
    VIOLET,
    AQUA,
    FUCHSIA,
    GREEN,
);

struct VertexInput {
    @location(0) position: vec2f,
}

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
}

struct Params {
    // w, h, beats, global_time_scale
    a: vec4f,

    // layer_distance, cell_size, color_cycle_speed, color_phase
    b: vec4f,

    // march_steps, march_advance, density, sphere_scale
    c: vec4f,

    // camera_travel_speed, spin_amount, tone_map, brightness
    d: vec4f,

    // camera_mode, camera_orbit_radius, fog_amount, layer_hue_drift
    e: vec4f,

    // ray_warp_amount, ray_warp_freq, terrain_amp, terrain_freq
    f: vec4f,
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
    let w = max(params.a.x, 1.0);
    let h = max(params.a.y, 1.0);
    let time = params.a.z * params.a.w;

    let layer_distance = max(params.b.x, 0.1);
    let cell_size = max(params.b.y, 0.01);
    let color_cycle_speed = max(params.b.z, 0.0);
    let color_phase = params.b.w;

    let march_steps = clamp(params.c.x, 1.0, f32(MAX_STEPS));
    let march_advance = max(params.c.y, 0.05);
    let density = max(params.c.z, 0.001);
    let sphere_scale = max(params.c.w, 0.001);

    let camera_travel_speed = params.d.x;
    let spin_amount = params.d.y;
    let tone_map = max(params.d.z, 0.0001);
    let brightness = max(params.d.w, 0.0);
    let camera_mode = i32(params.e.x);
    let camera_orbit_radius = max(params.e.y, 0.0);
    let fog_amount = clamp(params.e.z, 0.0, 1.0);
    let layer_hue_drift = max(params.e.w, 0.0);
    let ray_warp_amount = clamp(params.f.x, 0.0, 1.0);
    let ray_warp_freq = max(params.f.y, 0.01);
    let terrain_amp = max(params.f.z, 0.01);
    let terrain_freq = max(params.f.w, 0.01);

    var uv = position;
    uv.x *= w / h;
    uv.y *= -1.0;

    let phase = time * 0.14;
    let y_raw = sin(phase);
    let y = sign_nonzero(y_raw) * sqrt(abs(y_raw));
    let ny = smoothstep(-1.0, 1.0, y);
    let color_t = fract(
        ((time * color_cycle_speed) + color_phase) / f32(NUM_COLORS),
    );
    let travel = time * camera_travel_speed;
    let base_y = y * layer_distance * 0.5;

    var ro = vec3f(0.0, base_y, -travel);
    if (camera_mode == 1) {
        let orbit_a = travel * 0.35;
        ro = vec3f(
            sin(orbit_a) * camera_orbit_radius,
            base_y,
            -travel + cos(orbit_a) * camera_orbit_radius,
        );
    }
    if (camera_mode == 2) {
        let helix_a = travel * 0.6;
        ro = vec3f(
            sin(helix_a) * camera_orbit_radius,
            base_y + cos(helix_a * 0.5) * layer_distance * 0.35,
            -travel,
        );
    }

    var rd = normalize(vec3f(uv, -1.0));
    let rd_xy = rotate2(rd.xy, -ny * PI * spin_amount);
    rd = vec3f(rd_xy.x, rd_xy.y, rd.z);
    let rd_xz = rotate2(vec2f(rd.x, rd.z), sin(time * 0.5) * 0.4 * spin_amount);
    rd = vec3f(rd_xz.x, rd.y, rd_xz.y);
    let warp = ray_warp(
        rd,
        ro,
        time,
        ray_warp_amount,
        ray_warp_freq,
    );
    rd = normalize(rd + warp);

    var d = 0.0;
    let cam_clearance = map_scene(
        ro,
        time,
        layer_distance,
        cell_size,
        sphere_scale,
        terrain_amp,
        terrain_freq,
    );
    if (cam_clearance < CAM_SAFE_CLEARANCE) {
        d = CAM_SAFE_CLEARANCE - cam_clearance;
    }

    var col = vec3f(0.0);
    let steps = i32(march_steps);
    let ny_term = cos(ny * PI * 2.0) * 0.3 + 0.5;

    for (var i = 0; i < MAX_STEPS; i++) {
        if (i >= steps) {
            break;
        }

        let p = ro + rd * d;
        let layer_id = round(p.y / layer_distance);
        let layer_t = fract(color_t + layer_hue_drift * layer_id * 0.07);
        let c = get_color(layer_t);
        var dt = map_scene(
            p,
            time,
            layer_distance,
            cell_size,
            sphere_scale,
            terrain_amp,
            terrain_freq,
        );

        dt = max(dt * ny_term, MIN_DT);
        let energy = min(density / dt, MAX_CONTRIB);
        col += energy * c * brightness;
        d += dt * march_advance;
    }

    col = tanh(col * tone_map);
    let fog_factor = mix(1.0, exp(-d * 0.02), fog_amount);
    let fog_color = get_color(fract(color_t + 0.5)) * 0.08;
    col = mix(fog_color, col, fog_factor);
    return vec4f(col, 1.0);
}

fn get_color(t: f32) -> vec3f {
    let wrapped_t = fract(t);
    let scaled_t = wrapped_t * f32(NUM_COLORS);
    let curr = u32(floor(scaled_t)) % NUM_COLORS;
    let next = (curr + 1u) % NUM_COLORS;
    let local_t = fract(scaled_t);
    return mix(COLORS[curr], COLORS[next], local_t);
}

// https://www.shadertoy.com/view/4djSRW
fn hash41(p: f32) -> vec4f {
    var p4 = fract(vec4f(p) * vec4f(0.1031, 0.1030, 0.0973, 0.1099));
    let d = dot(p4, p4.wzxy + vec4f(33.33));
    p4 += vec4f(d);
    return fract((p4.xxyz + p4.yzzw) * p4.zywx);
}

fn get_height(
    id: vec2f,
    layer: f32,
    time: f32,
    terrain_amp: f32,
    terrain_freq: f32,
) -> f32 {
    let h = hash41(layer) * 1000.0;
    let t = time;
    var o = 0.0;
    o += sin(id.x * 0.2 * terrain_freq + h.x * 0.2 + t) * 0.3;
    o += sin(id.y * 0.2 * terrain_freq + h.y * 0.2 + t) * 0.3;
    o += sin(
        (-id.x + id.y) * 0.3 * terrain_freq + h.z * 0.3 + t
    ) * 0.3;
    o += sin(
        (id.x + id.y) * 0.3 * terrain_freq + h.z * 0.3 + t
    ) * 0.4;
    o += sin(
        (id.x - id.y) * 0.8 * terrain_freq + h.w * 0.8 + t
    ) * 0.1;
    return o * terrain_amp;
}

fn map_scene(
    p: vec3f,
    time: f32,
    layer_distance: f32,
    cell_size: f32,
    sphere_scale: f32,
    terrain_amp: f32,
    terrain_freq: f32,
) -> f32 {
    let spacing = vec3f(cell_size, layer_distance, cell_size);
    let cell = p / spacing;
    let id_xz = round(cell.xz);
    let y0 = floor(cell.y);
    let y1 = y0 + 1.0;
    let d0 = map_layer_cell(
        p,
        id_xz,
        y0,
        time,
        spacing,
        sphere_scale,
        terrain_amp,
        terrain_freq,
    );
    let d1 = map_layer_cell(
        p,
        id_xz,
        y1,
        time,
        spacing,
        sphere_scale,
        terrain_amp,
        terrain_freq,
    );

    let k = spacing.y * 0.12;
    return smin(d0, d1, k);
}

fn rotate2(v: vec2f, angle: f32) -> vec2f {
    let c = cos(angle);
    let s = sin(angle);
    return vec2f(
        c * v.x - s * v.y,
        s * v.x + c * v.y,
    );
}

fn sd_sphere(p: vec3f, r: f32) -> f32 {
    return length(p) - r;
}

fn sign_nonzero(v: f32) -> f32 {
    return select(-1.0, 1.0, v >= 0.0);
}

fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (b - a) / max(k, 1e-4), 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}

fn map_layer_cell(
    p: vec3f,
    id_xz: vec2f,
    layer: f32,
    time: f32,
    spacing: vec3f,
    sphere_scale: f32,
    terrain_amp: f32,
    terrain_freq: f32,
) -> f32 {
    let ho = get_height(id_xz, layer, time, terrain_amp, terrain_freq);

    var q = p;
    q.y += ho;
    q -= vec3f(
        spacing.x * id_xz.x,
        spacing.y * layer,
        spacing.z * id_xz.y,
    );

    let radius = smoothstep(1.3, -1.3, ho) * sphere_scale + 0.0001;
    return sd_sphere(q, radius);
}

fn ray_warp(
    rd: vec3f,
    ro: vec3f,
    time: f32,
    amount: f32,
    warp_freq: f32,
) -> vec3f {
    if (amount < 1e-4) {
        return vec3f(0.0);
    }

    let p = rd * 8.0 + ro * 0.04;
    let t = time;
    let w = vec3f(
        sin(p.y * warp_freq + t * 0.73) - cos(p.z * 1.7 * warp_freq - t * 0.41),
        sin(p.z * warp_freq + t * 0.67) - cos(p.x * 1.9 * warp_freq + t * 0.37),
        sin(p.x * warp_freq + t * 0.59) - cos(p.y * 1.5 * warp_freq - t * 0.53),
    );

    let amt = amount * amount;
    return w * (amt * 0.06);
}
