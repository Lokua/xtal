// Forked form https://www.shadertoy.com/view/WX3cR4
const PI: f32 = 3.141592653589793;
const NUM_COLORS: u32 = 14u;
const MAX_STEPS: i32 = 96;

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

    var uv = position;
    uv.x *= w / h;
    uv.y *= -1.0;

    let phase = time * 0.2;
    let y = sin(phase);
    let ny = smoothstep(-1.0, 1.0, y);
    let color_t = fract(
        ((time * color_cycle_speed) + color_phase) / f32(NUM_COLORS),
    );
    let c = get_color(color_t);

    let ro = vec3f(
        0.0,
        y * layer_distance * 0.5,
        -time * camera_travel_speed,
    );
    var rd = normalize(vec3f(uv, -1.0));
    let rd_xy = rotate2(rd.xy, -ny * PI * spin_amount);
    rd = vec3f(rd_xy.x, rd_xy.y, rd.z);
    let rd_xz = rotate2(vec2f(rd.x, rd.z), sin(time * 0.5) * 0.4 * spin_amount);
    rd = vec3f(rd_xz.x, rd.y, rd_xz.y);

    var d = 0.0;
    var col = vec3f(0.0);
    let steps = i32(march_steps);
    let ny_term = cos(ny * PI * 2.0) * 0.3 + 0.5;

    for (var i = 0; i < MAX_STEPS; i++) {
        if (i >= steps) {
            break;
        }

        let p = ro + rd * d;
        var dt = map_scene(
            p,
            time,
            layer_distance,
            cell_size,
            sphere_scale,
        );

        dt = max(dt * ny_term, 1e-3);
        col += (density / dt) * c * brightness;
        d += dt * march_advance;
    }

    col = tanh(col * tone_map);
    return vec4f(col, 1.0);
}

fn get_color(t: f32) -> vec3f {
    let scaled_t = clamp(t, 0.0, 1.0) * f32(NUM_COLORS - 1u);
    let curr = u32(floor(scaled_t));
    let next = min(curr + 1u, NUM_COLORS - 1u);
    let local_t = scaled_t - f32(curr);
    return mix(COLORS[curr], COLORS[next], local_t);
}

// https://www.shadertoy.com/view/4djSRW
fn hash41(p: f32) -> vec4f {
    var p4 = fract(vec4f(p) * vec4f(0.1031, 0.1030, 0.0973, 0.1099));
    let d = dot(p4, p4.wzxy + vec4f(33.33));
    p4 += vec4f(d);
    return fract((p4.xxyz + p4.yzzw) * p4.zywx);
}

fn get_height(id: vec2f, layer: f32, time: f32) -> f32 {
    let h = hash41(layer) * 1000.0;
    var o = 0.0;
    o += sin((id.x + h.x) * 0.2 + time) * 0.3;
    o += sin((id.y + h.y) * 0.2 + time) * 0.3;
    o += sin((-id.x + id.y + h.z) * 0.3 + time) * 0.3;
    o += sin((id.x + id.y + h.z) * 0.3 + time) * 0.4;
    o += sin((id.x - id.y + h.w) * 0.8 + time) * 0.1;
    return o;
}

fn map_scene(
    p: vec3f,
    time: f32,
    layer_distance: f32,
    cell_size: f32,
    sphere_scale: f32,
) -> f32 {
    let spacing = vec3f(cell_size, layer_distance, cell_size);
    let id = round(p / spacing);
    let ho = get_height(id.xz, id.y, time);

    var q = p;
    q.y += ho;
    q -= spacing * id;

    let radius = smoothstep(1.3, -1.3, ho) * sphere_scale + 0.0001;
    return sd_sphere(q, radius);
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
