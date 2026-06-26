// Inspired or pseudo-forked from
// https://shaderpark.com/sculpture/-OorznV98xM6m7cVQOcC
const TAU: f32 = 6.283185307;

struct VertexInput {
    @location(0) position: vec2f,
}

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
}

struct Params {
    // w, h, beats, flow_speed
    a: vec4f,
    // noise_scale, amplitude, rings, color_mix
    b: vec4f,
    // attract, warp, refraction, specular
    c: vec4f,
    // hue, saturation, contrast, zoom
    d: vec4f,
    // flow_direction, auto_hue, light_angle, edge_glow
    e: vec4f,
    // hue_animation, disturbance_amount, disturbance_scale, disturbance_radius
    f: vec4f,
    // disturbance_x, disturbance_y, disturbance_probe, unused
    g: vec4f,
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
    let resolution = vec2f(max(params.a.x, 1.0), max(params.a.y, 1.0));
    let beats = params.a.z;
    let flow_speed = params.a.w;
    let noise_scale = params.b.x;
    let amplitude = params.b.y;
    let rings = params.b.z;
    let color_mix = params.b.w;
    let attract = params.c.x;
    let warp = params.c.y;
    let refraction = params.c.z;
    let specular = params.c.w;
    let auto_hue = bool(params.e.y);
    let hue = select(params.d.x, params.f.x, auto_hue);
    let disturbance_amount = params.f.y;
    let disturbance_scale = params.f.z;
    let disturbance_radius = params.f.w;
    let disturbance_center = vec2f(params.g.x, params.g.y);
    let disturbance_probe = bool(params.g.z);
    let saturation = params.d.y;
    let contrast = params.d.z;
    let zoom = params.d.w;
    let flow_direction = i32(params.e.x);
    let light_angle = params.e.z;
    let edge_glow = params.e.w;

    var uv = position;
    uv.x *= resolution.x / resolution.y;
    uv /= max(zoom, 0.01);

    let time = beats * flow_speed;
    let direction = get_flow_direction(flow_direction);
    let sample_uv = disturbed_sample(
        uv,
        noise_scale,
        direction,
        time,
        disturbance_center,
        disturbance_amount,
        disturbance_scale,
        disturbance_radius,
        warp,
    );
    let height = water(sample_uv, warp, attract) * amplitude;
    let epsilon = 0.012 * noise_scale;
    let uv_epsilon = epsilon / max(noise_scale, 0.001);
    let height_x = water(
        disturbed_sample(
            uv + vec2f(uv_epsilon, 0.0),
            noise_scale,
            direction,
            time,
            disturbance_center,
            disturbance_amount,
            disturbance_scale,
            disturbance_radius,
            warp,
        ),
        warp,
        attract,
    ) * amplitude;
    let height_y = water(
        disturbed_sample(
            uv + vec2f(0.0, uv_epsilon),
            noise_scale,
            direction,
            time,
            disturbance_center,
            disturbance_amount,
            disturbance_scale,
            disturbance_radius,
            warp,
        ),
        warp,
        attract,
    ) * amplitude;
    let normal = normalize(vec3f(
        (height - height_x) / epsilon,
        (height - height_y) / epsilon,
        1.0,
    ));

    let refracted_uv = sample_uv + normal.xy * refraction;
    let n1 = normalized_sine(
        water(refracted_uv, warp, attract) * rings,
    );
    let n2 = normalized_sine(
        water(refracted_uv + vec2f(0.31, 0.17), warp, attract)
            * rings,
    );
    let n3 = normalized_sine(
        water(refracted_uv + vec2f(0.68, 0.37), warp, attract)
            * rings,
    );
    let ring_color = pow(max(vec3f(n1, n2, n3), vec3f(0.0)), vec3f(5.0));

    let light = normalize(vec3f(
        cos(light_angle * TAU),
        sin(light_angle * TAU),
        0.8,
    ));
    let diffuse = 0.28 + 0.72 * max(dot(normal, light), 0.0);
    let half_vector = normalize(light + vec3f(0.0, 0.0, 1.0));
    let highlight = pow(max(dot(normal, half_vector), 0.0), 48.0);
    let fresnel = pow(1.0 - max(normal.z, 0.0), 3.0);

    let base = hsv_to_rgb(vec3f(fract(hue + height * 0.08), saturation, 0.8));
    let spectral = mix(base * ring_color, ring_color, saturation * 0.5);
    var color = mix(base * diffuse, spectral, color_mix);
    color += highlight * specular;
    let edge_color = hsv_to_rgb(vec3f(fract(hue + 0.48), 0.7, 1.0));
    color += fresnel * edge_glow * edge_color;
    color = (color - 0.5) * contrast + 0.5;
    color = 1.0 - exp(-max(color, vec3f(0.0)) * 1.35);
    color = draw_probe(color, uv, disturbance_center, beats, disturbance_probe);
    return vec4f(color, 1.0);
}

fn draw_probe(
    color: vec3f,
    uv: vec2f,
    center: vec2f,
    beats: f32,
    enabled: bool,
) -> vec3f {
    if !enabled {
        return color;
    }

    let d = distance(uv, center);
    let beat_phase = fract(beats);
    let ping_radius = mix(0.025, 0.18, beat_phase);
    let ping_fade = pow(1.0 - beat_phase, 1.6);
    let dot_mask = 1.0 - smoothstep(0.012, 0.02, d);
    let ping_mask = 1.0 - smoothstep(0.0, 0.012, abs(d - ping_radius));
    let probe = max(dot_mask, ping_mask * ping_fade);
    return mix(color, vec3f(1.0, 0.0, 0.0), probe);
}

fn disturbed_sample(
    uv: vec2f,
    noise_scale: f32,
    direction: vec2f,
    time: f32,
    center: vec2f,
    amount: f32,
    scale: f32,
    radius: f32,
    warp: f32,
) -> vec2f {
    let p = uv * noise_scale - direction * time;
    return apply_disturbance(
        p,
        uv,
        center,
        amount,
        scale,
        radius,
        warp,
    );
}

fn apply_disturbance(
    p: vec2f,
    uv: vec2f,
    center: vec2f,
    amount: f32,
    scale: f32,
    radius: f32,
    warp: f32,
) -> vec2f {
    let safe_radius = max(radius, 0.001);
    let local = uv - center;
    let shaped = vec2f(local.x * 1.3, local.y * 0.8);
    let mask = exp(-dot(shaped, shaped) / (safe_radius * safe_radius));
    let warp_offset = vec2f(warp * 0.37, -warp * 0.23);
    let swim_p = shaped * max(scale, 0.001) + warp_offset;
    let noise_vec = vec2f(
        fbm(swim_p + vec2f(2.1, 4.3)),
        fbm(swim_p + vec2f(7.7, 1.9)),
    );
    let swirl = vec2f(-local.y, local.x) / safe_radius;
    let push = swirl * 0.2 + noise_vec * 0.65;
    return p + push * mask * amount;
}

fn water(p: vec2f, warp: f32, attract: f32) -> f32 {
    let q = vec2f(
        fbm(p),
        fbm(p + vec2f(4.7, 1.3)),
    );
    let r = vec2f(
        fbm(p + warp * q + vec2f(1.7, 9.2)),
        fbm(p + warp * q + vec2f(8.3, 2.8)),
    );
    let pull = length(p) * attract;
    return fbm(p + warp * r) + sin(pull) * attract * 0.12;
}

fn get_flow_direction(direction: i32) -> vec2f {
    if direction == 1 {
        return normalize(vec2f(1.0, 1.0));
    }
    if direction == 2 {
        return vec2f(1.0, 0.0);
    }
    if direction == 3 {
        return normalize(vec2f(1.0, -1.0));
    }
    if direction == 4 {
        return vec2f(0.0, -1.0);
    }
    if direction == 5 {
        return normalize(vec2f(-1.0, -1.0));
    }
    if direction == 6 {
        return vec2f(-1.0, 0.0);
    }
    if direction == 7 {
        return normalize(vec2f(-1.0, 1.0));
    }
    return vec2f(0.0, 1.0);
}

fn fbm(p: vec2f) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var point = p;
    for (var i = 0; i < 5; i++) {
        value += amplitude * value_noise(point);
        point = mat2x2f(1.6, 1.2, -1.2, 1.6) * point + vec2f(0.17);
        amplitude *= 0.5;
    }
    return value;
}

fn value_noise(p: vec2f) -> f32 {
    let cell = floor(p);
    let local = fract(p);
    let curve = local * local * (3.0 - 2.0 * local);
    let a = hash21(cell);
    let b = hash21(cell + vec2f(1.0, 0.0));
    let c = hash21(cell + vec2f(0.0, 1.0));
    let d = hash21(cell + vec2f(1.0, 1.0));
    return mix(mix(a, b, curve.x), mix(c, d, curve.x), curve.y) * 2.0 - 1.0;
}

fn hash21(p: vec2f) -> f32 {
    let p3 = fract(vec3f(p.x, p.y, p.x) * 0.1031);
    let mixed = p3 + dot(p3, p3.yzx + vec3f(33.33));
    return fract((mixed.x + mixed.y) * mixed.z);
}

fn normalized_sine(value: f32) -> f32 {
    return sin(value) * 0.5 + 0.5;
}

fn hsv_to_rgb(hsv: vec3f) -> vec3f {
    let p = abs(
        fract(hsv.xxx + vec3f(0.0, 2.0 / 3.0, 1.0 / 3.0)) * 
        6.0 - 3.0
    );
    let rgb = clamp(p - 1.0, vec3f(0.0), vec3f(1.0));
    return hsv.z * mix(vec3f(1.0), rgb, hsv.y);
}
