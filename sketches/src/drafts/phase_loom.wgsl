const TAU: f32 = 6.283185307179586;

struct Params {
    a: vec4f,
    b: vec4f,
    c: vec4f,
    d: vec4f,
    e: vec4f,
    f: vec4f,
    g: vec4f,
    h: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

struct VsOut {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

struct VertexInput {
    @location(0) position: vec2f,
}

@vertex
fn vs_main(vert: VertexInput) -> VsOut {
    let p = vert.position;
    var out: VsOut;
    out.position = vec4f(p, 0.0, 1.0);
    out.uv = p * 0.5 + vec2f(0.5, 0.5);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4f {
    let resolution = vec2f(max(params.a.x, 1.0), max(params.a.y, 1.0));
    let seed = params.a.w;
    let density = params.b.x;
    let variation = params.b.y;
    let aperture = params.b.z;
    let scale = params.b.w;
    let phase = params.c.x;
    let drift = params.c.y;
    let line_weight = params.c.z;
    let pulse_amount = params.c.w;
    let ink = params.d.x;
    let bg_hue = params.d.y;
    let stroke_hue = params.d.z;
    let accent_hue = params.d.w;
    let pulse = params.e.x;
    let pulse_shift = params.e.y;
    let pulse_count = params.e.z;
    let saturation = params.f.x;
    let brightness = params.f.y;
    let grain_amount = params.f.w;

    let screen_p = (in.uv * resolution - 0.5 * resolution) / resolution.y;
    let p = screen_p * scale;

    let loom = phase_loom(p, seed, density, variation, phase, drift);
    let pulse_mask = pulse_regions(
        loom.x,
        loom.y,
        seed,
        pulse_shift,
        pulse_count,
    );
    let pulsed_field = loom.x + pulse * pulse_amount * pulse_mask * 0.16;
    let line = contour_lines(
        pulsed_field,
        aperture,
        density,
        ink,
        line_weight,
    );
    let accent = accent_mask(pulsed_field, loom.y, line);
    let dry = dry_mask(p, loom.y, grain_amount, ink);
    let paper = paper_grain(in.uv * resolution, grain_amount);

    var mark = clamp(line * (0.94 + accent * 0.18), 0.0, 1.0);
    mark = clamp(mark * dry, 0.0, 1.0);
    mark = pow(mark, mix(1.15, 0.72, ink));

    var color = palette(
        mark,
        accent,
        bg_hue,
        stroke_hue,
        accent_hue,
        saturation,
        brightness,
        ink,
        paper,
    );
    color = apply_vignette(color, in.uv);

    return vec4f(clamp(color, vec3f(0.0), vec3f(1.0)), 1.0);
}

fn phase_loom(
    p: vec2f,
    seed: f32,
    density: f32,
    variation: f32,
    phase: f32,
    drift: f32,
) -> vec3f {
    let phase_a = sin(phase);
    let phase_b = cos(phase);
    let phase_c = sin(phase * 2.0);
    var q = p * 1.08;
    q += vec2f(phase_a, phase_b) * 0.045;

    var carrier = 0.0;
    var lattice = 0.0;
    var pins = 0.0;

    for (var i = 0; i < 6; i = i + 1) {
        let fi = f32(i) + 1.0;
        let spin = seed * TAU + fi * 2.399963 + drift * 0.28;
        let wobble = phase_a * sin(fi * 1.7) * 0.18 +
            phase_b * cos(fi * 0.9) * 0.11;
        let dir = unit(spin + wobble);
        let local = rotate2(q, spin * 0.37 + wobble);
        let freq = density * (0.54 + fi * 0.075);

        let plane = sin(dot(q, dir) * freq + phase_a * fi);
        let braid = sin((local.x + sin(local.y * 1.6) * 0.42) * freq);
        let shear = sin((local.x - local.y) * freq * 0.72 + phase_c);
        let a = smoothstep(0.0, 0.62, variation);
        let b = smoothstep(0.42, 1.0, variation);
        let signal = mix(mix(plane, braid, a), shear, b);

        carrier += signal / fi;
        lattice += sin(signal * 2.4 + local.x * 0.35) * (0.55 / fi);
        pins += abs(plane * braid) * (0.26 / fi);

        q += dir.yx * vec2f(-1.0, 1.0) * signal * 0.012;
    }

    return vec3f(carrier + lattice, lattice, pins);
}

fn contour_lines(
    field: f32,
    aperture: f32,
    density: f32,
    ink: f32,
    line_weight: f32,
) -> f32 {
    let density_norm = smoothstep(2.0, 8.0, density);
    let scaled = field * (0.86 + density * 0.072);
    let contour = abs(fract(scaled) - 0.5);
    let grad = length(vec2f(dpdx(scaled), dpdy(scaled)));
    let aa = max(grad * 0.45, 0.0006);
    let width = mix(0.035, 0.008, aperture) *
        mix(1.0, 1.18, ink) *
        line_weight;
    let wire = 1.0 - smoothstep(width, width + aa, contour);
    let gain = mix(1.0, 1.28, density_norm);
    return clamp(wire * gain, 0.0, 1.0);
}

fn pulse_regions(
    field: f32,
    lattice: f32,
    seed: f32,
    pulse_shift: f32,
    pulse_count: f32,
) -> f32 {
    let count = max(1.0, floor(pulse_count + 0.5));
    let shifted = field * 0.23 + lattice * 0.11 + pulse_shift + seed * 0.17;
    let slot = cos(fract(shifted) * TAU * count);
    let selected = smoothstep(0.74, 0.98, slot);
    let contour_group = smoothstep(0.08, 0.7, abs(sin(field * 2.0)));
    return selected * contour_group;
}

fn accent_mask(field: f32, lattice: f32, line: f32) -> f32 {
    let woven = sin(field * 1.7 + lattice * 3.1);
    let fine = sin(field * 5.0 - lattice * 2.3);
    let mask = smoothstep(0.24, 0.96, abs(woven + fine * 0.35));
    return mask * smoothstep(0.08, 0.82, line);
}

fn palette(
    mark: f32,
    accent_mix: f32,
    bg_hue: f32,
    stroke_hue: f32,
    accent_hue: f32,
    saturation: f32,
    brightness: f32,
    ink: f32,
    paper: f32,
) -> vec3f {
    let bg_value = brightness * (0.045 + paper * 0.035);
    let bg = hsv_to_rgb(vec3f(bg_hue, saturation * 0.5, bg_value));
    let stroke = hsv_to_rgb(vec3f(stroke_hue, saturation, brightness * 0.82));
    let accent = hsv_to_rgb(vec3f(accent_hue, saturation, brightness));
    let pigment = mix(stroke, accent, accent_mix * (1.0 - ink * 0.35));
    let core = smoothstep(0.58, 1.0, mark) * ink;
    var color = mix(bg, pigment, mark);
    color = mix(color, vec3f(0.006, 0.005, 0.004), core * 0.74);
    return color + vec3f(paper * 0.025);
}

fn dry_mask(p: vec2f, lattice: f32, grain_amount: f32, ink: f32) -> f32 {
    let grid = floor(p * vec2f(420.0, 260.0) + vec2f(lattice * 13.0));
    let fiber = hash21(grid);
    let scratch = sin((p.x - p.y) * 720.0 + lattice * 11.0) * 0.5 + 0.5;
    let dry = smoothstep(0.12, 0.92, fiber * 0.72 + scratch * 0.28);
    return mix(1.0, dry, grain_amount * mix(0.22, 0.82, ink));
}

fn paper_grain(px: vec2f, grain_amount: f32) -> f32 {
    let fine = hash21(floor(px));
    let coarse = hash21(floor(px * 0.23));
    let grain = fine * 0.72 + coarse * 0.28;
    return (grain - 0.5) * grain_amount;
}

fn apply_vignette(color: vec3f, uv: vec2f) -> vec3f {
    let p = uv * 2.0 - vec2f(1.0);
    let d = dot(p, p);
    let amount = smoothstep(0.34, 1.18, d);
    return mix(color, color * 0.32, amount);
}

fn unit(a: f32) -> vec2f {
    return vec2f(cos(a), sin(a));
}

fn rotate2(p: vec2f, a: f32) -> vec2f {
    let c = cos(a);
    let s = sin(a);
    return vec2f(p.x * c - p.y * s, p.x * s + p.y * c);
}

fn hash21(p: vec2f) -> f32 {
    let q = fract(vec3f(p.xyx) * 0.1031);
    let r = q + dot(q, q.yzx + 33.33);
    return fract((r.x + r.y) * r.z);
}

fn hsv_to_rgb(hsv: vec3f) -> vec3f {
    let k = vec4f(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(vec3f(hsv.x) + k.xyz) * 6.0 - vec3f(k.w));
    let rgb = clamp(p - vec3f(k.x), vec3f(0.0), vec3f(1.0));
    return hsv.z * mix(vec3f(k.x), rgb, hsv.y);
}
