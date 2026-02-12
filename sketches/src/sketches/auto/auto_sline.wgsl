const TAU: f32 = 6.283185307179586;
// Must match what's on Rust side exactly
const DENSITY: f32 = 1.0;
// Must be <= what's declared on the Rust side
const SAMPLES_PER_LINE: f32 = 2100.0;
const DOMAIN_SCALE: f32 = 1.08;
const TILT: f32 = -4.0;
const FLOW_FIELD_ENABLED: f32 = 1.0;
const FLOW_FIELD_RATE: f32 = 0.1;
const RATE_GRADIENT: f32 = 1.0;
const RATE_SPREAD: f32 = 1.0;
const GLOW: f32 = 2.0;
const BG_LIFT: f32 = 0.0;

struct VertexOutput {
    @builtin(position) pos: vec4f,
    @location(0) point_color: vec4f,
    @location(1) uv: vec2f,
}

struct Params {
    // a1, a2 are width/height from dynamic uniforms, a4 is beats.
    a: vec4f,

    // b1 line_count, b3 point_size
    b: vec4f,

    // c2 wave_amp, c3 wave_freq, c4 phase_rate
    c: vec4f,

    // d2 focus_x, d3 focus_y, d4 focus_pull
    d: vec4f,

    // e1 stripe_amount, e2 stripe_freq, e3 stripe_rate
    e: vec4f,

    // f1 jitter, f2 hue, f3 brightness
    f: vec4f,

    // g3 focus_color_impact, g4 harmonic_amp
    g: vec4f,

    // reserved
    h: vec4f,

    // i2 flow_field_amount, i3 flow_field_scale
    i: vec4f,

    // ...
    j: vec4f,

    // k1 line_hue_span, k2 harmonic_freq, k3 harmonic_rate
    k: vec4f,

    // ...
    l: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@vertex
fn vs_main(@builtin(vertex_index) vidx: u32) -> VertexOutput {
    if (vidx < 3u) {
        var out: VertexOutput;
        let bg_pos = fullscreen_triangle_pos(vidx);
        out.pos = vec4f(bg_pos, 0.0, 1.0);
        out.point_color = vec4f(0.0);
        out.uv = (bg_pos + 1.0) * 0.5;
        return out;
    }

    let vert_index = vidx - 3u;

    let line_count = max(1.0, params.b.x * DENSITY);
    let samples_per_line = max(2.0, SAMPLES_PER_LINE);
    let point_size = max(0.0002, params.b.z);

    let domain_scale = max(0.01, DOMAIN_SCALE);
    let wave_amp = params.c.y;
    let wave_freq = params.c.z;
    let phase_rate = params.c.w;
    let beats = params.a.w;

    let tilt = TILT;
    let focus = vec2f(params.d.y, params.d.z);
    let focus_pull = params.d.w;

    let stripe_amount = params.e.x;
    let stripe_freq = params.e.y;
    let stripe_rate = params.e.z;
    let glow = GLOW;

    let jitter = params.f.x;
    let hue = params.f.y;
    let brightness = params.f.z;
    let rate_gradient = RATE_GRADIENT;
    let rate_spread = RATE_SPREAD;
    let focus_color_impact = params.g.z;
    let harmonic_amp = params.g.w;

    let flow_field_enabled = FLOW_FIELD_ENABLED;
    let flow_field_amount = params.i.y;
    let flow_field_scale = params.i.z;
    let flow_field_rate = FLOW_FIELD_RATE;

    let line_hue_span = params.k.x;
    let harmonic_freq = max(0.0001, params.k.y);
    let harmonic_rate = params.k.z;

    let total_points = u32(line_count * samples_per_line);
    let point_index = (vert_index / 6u) % total_points;
    let corner_index = vert_index % 6u;

    let line_idx = floor(f32(point_index) / samples_per_line);
    let point_in_line = f32(point_index) % samples_per_line;

    let t = point_in_line / (samples_per_line - 1.0);
    let line_norm = select(0.0, line_idx / (line_count - 1.0), line_count > 1.0);
    let aspect = params.a.x / max(1.0, params.a.y);

    var pos = vec2f(
        mix(-aspect, aspect, t),
        line_norm * 2.0 - 1.0
    ) * domain_scale;

    let line_bias = line_norm * 2.0 - 1.0;
    let line_rate_scale = max(0.0, 1.0 - line_bias * rate_gradient * rate_spread);
    let line_phase = beats * phase_rate * line_rate_scale + line_idx * tilt;
    let wave_amp_mod = max(0.0, wave_amp);
    let stripe_phase = beats * stripe_rate;

    // Add a second harmonic with an irrational ratio for richer motion.
    let base_phase = (t * TAU * wave_freq) + line_phase;
    let harmonic_time_phase = beats * TAU * harmonic_rate;
    let harmonic_phase = base_phase * harmonic_freq + line_phase * 0.17 + harmonic_time_phase;
    let wave_shape = sin(base_phase) + sin(harmonic_phase) * harmonic_amp;
    let wave = wave_shape * wave_amp_mod;
    pos.y += wave;

    // Optional flow field advection for moving topo-map style distortion.
    let flow_uv = vec2f(pos.x / max(0.0001, aspect), pos.y) * flow_field_scale;
    let flow_t = beats * flow_field_rate;
    let flow_angle =
        sin(flow_uv.x * 1.7 + flow_t) +
        cos(flow_uv.y * 1.3 - flow_t * 1.11) +
        sin((flow_uv.x + flow_uv.y) * 0.9 + flow_t * 0.63);
    let flow_dir = vec2f(cos(flow_angle * TAU), sin(flow_angle * TAU));
    let flow_advect = flow_dir * flow_field_amount * flow_field_enabled * 0.16;
    pos += vec2f(flow_advect.x * aspect, flow_advect.y);

    let to_focus = focus - pos;
    let dist = max(length(to_focus), 0.0001);
    pos += (to_focus / dist) * (focus_pull * 0.05) / (1.0 + dist * 6.0);

    let grain = random_normal(point_index + 37u, 1.0) * jitter;
    let jitter_angle = 0.0;
    let grain_dir = vec2f(cos(jitter_angle), sin(jitter_angle));
    pos += grain_dir * grain;

    let stripe = sin((pos.y * stripe_freq) + stripe_phase);
    pos.x += stripe * stripe_amount * 0.25;

    pos.x /= aspect;

    let shade = mix(0.7, 1.0, line_norm) * brightness;
    let stripe_energy = 0.5 + 0.5 * abs(stripe);
    let glow_gain = 1.0 + glow * (0.5 + stripe_energy * 1.6);
    let focus_strength = clamp(abs(focus_pull) / 12.0, 0.0, 1.0);
    let depth_falloff = 0.38 + focus_strength * 0.95;
    let depth_mask = exp(-dist * depth_falloff);
    let depth_amount = depth_mask * focus_strength * max(0.0, focus_color_impact);
    let depth_gain = 1.0 + depth_amount * 1.8;
    let focus_for_color = vec2f(focus.x / aspect, focus.y);
    let focus_dir = pos - focus_for_color;
    let focus_dist = max(length(focus_dir), 0.0001);
    let pulse = 0.5 + 0.5 * sin(beats * 0.35);
    let blast_radius = mix(0.22, 1.05, pulse);
    let blast_mask = exp(-pow(focus_dist / max(0.0001, blast_radius), 2.0));
    let line_hue_offset = (line_norm - 0.5) * line_hue_span;
    let blast_hue_offset = blast_mask * focus_strength * focus_color_impact * 0.18;
    let final_hue = fract(hue + line_hue_offset + blast_hue_offset);
    let final_sat = 0.82 + 0.12 * blast_mask * focus_strength;
    let neon = hsv2rgb(vec3f(final_hue, final_sat, 1.0));
    let color = neon * shade * glow_gain * depth_gain;

    let quad_offset = quad_corner(corner_index, point_size);

    var out: VertexOutput;
    out.pos = vec4f(pos + quad_offset, 0.0, 1.0);
    out.point_color = vec4f(color, 0.08 + glow * 0.08);
    out.uv = (out.pos.xy + 1.0) * 0.5;
    return out;
}

@fragment
fn fs_main(
    @location(0) point_color: vec4f,
    @location(1) _uv: vec2f,
) -> @location(0) vec4f {
    let bg = hsv2rgb(vec3f(fract(params.f.y + 0.03), 0.45, BG_LIFT));
    if (point_color.a <= 0.0) {
        return vec4f(bg, 1.0);
    }
    return point_color;
}

fn fullscreen_triangle_pos(index: u32) -> vec2f {
    switch (index) {
        case 0u: { return vec2f(-1.0, -3.0); }
        case 1u: { return vec2f(3.0, 1.0); }
        case 2u: { return vec2f(-1.0, 1.0); }
        default: { return vec2f(-1.0, -3.0); }
    }
}

fn quad_corner(index: u32, size: f32) -> vec2f {
    switch (index) {
        case 0u: { return vec2f(-size, -size); }
        case 1u: { return vec2f(-size, size); }
        case 2u: { return vec2f(size, size); }
        case 3u: { return vec2f(-size, -size); }
        case 4u: { return vec2f(size, size); }
        case 5u: { return vec2f(size, -size); }
        default: { return vec2f(0.0, 0.0); }
    }
}

fn hsv2rgb(c: vec3f) -> vec3f {
    let rgb = clamp(abs(fract(c.x + vec3f(0.0, 2.0 / 3.0, 1.0 / 3.0)) * 6.0 - 3.0) - 1.0, vec3f(0.0), vec3f(1.0));
    let shaped = rgb * rgb * (3.0 - 2.0 * rgb);
    return c.z * mix(vec3f(1.0), shaped, c.y);
}

fn rand_pcg(seed: u32) -> f32 {
    var state = seed * 747796405u + 2891336453u;
    var word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    let result = (word >> 22u) ^ word;
    return f32(result) / 4294967295.0;
}

fn random_normal(seed: u32, std_dev: f32) -> f32 {
    let u1 = max(0.00001, rand_pcg(seed));
    let u2 = rand_pcg(seed + 1u);
    let mag = sqrt(-2.0 * log(u1));
    let z0 = mag * cos(TAU * u2);
    return std_dev * z0;
}
