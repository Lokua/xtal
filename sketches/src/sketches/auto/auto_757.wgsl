// Membrane Resonance — vibrating elastic surface viewed
// from above. Standing wave interference produces
// Chladni-like nodal patterns over a dark field.

const PI: f32 = 3.14159265359;
const TAU: f32 = 6.283185307179586;
const MAX_MODES: i32 = 8;

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, beats, drive
    a: vec4f,
    // mode_count, mode_spread, decay, tension
    b: vec4f,
    // warp_amount, warp_freq, brightness, contrast
    c: vec4f,
    // line_sharpness, node_glow, invert, grain
    d: vec4f,
    // vignette, color_mode, accent_hue, accent_sat
    e: vec4f,
    // phase_speed, asymmetry, ring_count, ring_mix
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
fn fs_main(
    @location(0) position: vec2f,
) -> @location(0) vec4f {
    let beats = params.a.z;
    let drive = max(params.a.w, 0.01);
    let mode_count = clamp(
        i32(params.b.x), 1, MAX_MODES
    );
    let mode_spread = max(params.b.y, 0.1);
    let decay = clamp(params.b.z, 0.01, 1.0);
    let tension = max(params.b.w, 0.1);
    let warp_amount = params.c.x;
    let warp_freq = max(params.c.y, 0.1);
    let brightness = max(params.c.z, 0.0);
    let contrast = max(params.c.w, 0.1);
    let line_sharpness = max(params.d.x, 0.1);
    let node_glow = max(params.d.y, 0.0);
    let invert = params.d.z > 0.5;
    let grain_amount = max(params.d.w, 0.0);
    let vignette = max(params.e.x, 0.0);
    let color_mode = i32(params.e.y);
    let accent_hue = params.e.z;
    let accent_sat = clamp(params.e.w, 0.0, 1.0);
    let phase_speed = params.f.x;
    let asymmetry = clamp(params.f.y, 0.0, 1.0);
    let ring_count = max(params.f.z, 0.0);
    let ring_mix = clamp(params.f.w, 0.0, 1.0);

    let p = correct_aspect(position);

    // Domain warping via low-freq fbm
    var wp = p;
    if warp_amount > 0.001 {
        let wt = beats * 0.07;
        let wx = fbm(
            p * warp_freq
                + vec2f(3.7, 1.2)
                + vec2f(wt, -wt * 0.6),
            3,
        );
        let wy = fbm(
            p * warp_freq
                + vec2f(8.1, 4.3)
                + vec2f(-wt * 0.8, wt * 0.5),
            3,
        );
        wp = p + vec2f(wx - 0.5, wy - 0.5)
            * warp_amount;
    }

    // Accumulate standing wave modes.
    // Each mode is a 2D sinusoidal eigenmode of a
    // membrane, beating at its own frequency derived
    // from tension and mode indices.
    var displacement = 0.0;
    let phase_t = beats * phase_speed;

    for (var i = 0; i < MAX_MODES; i++) {
        if i >= mode_count {
            break;
        }
        let fi = f32(i);
        let seed = hash21(vec2f(fi * 17.3, fi * 7.1));

        // Mode indices (m, n) for rectangular membrane
        let m = floor(fi * 0.5) + 1.0;
        let n = fract(fi * 0.5) * 2.0 + 1.0;

        // Asymmetry shifts mode spacing
        let mx = m + asymmetry * seed * 2.0;
        let ny = n + asymmetry * (1.0 - seed) * 2.0;

        // Eigenfrequency ~ sqrt(m^2 + n^2) * tension
        let freq = sqrt(mx * mx + ny * ny)
            * tension * mode_spread;

        // Spatial pattern
        let spatial = sin(mx * PI * wp.x * 0.5)
            * sin(ny * PI * wp.y * 0.5);

        // Temporal oscillation with drive
        let amp = exp(-fi * decay * 0.5);
        let phase = phase_t * freq * drive
            + seed * TAU;
        let temporal = sin(phase);

        displacement += spatial * temporal * amp;
    }

    // Normalise to roughly [-1, 1]
    displacement = displacement
        / max(f32(mode_count) * 0.4, 1.0);

    // Nodal lines: regions near zero displacement
    let abs_disp = abs(displacement);
    let nodal = 1.0 - smoothstep(
        0.0,
        0.15 / line_sharpness,
        abs_disp,
    );

    // Radial rings (concentric interference)
    var rings = 0.0;
    if ring_count > 0.0 && ring_mix > 0.001 {
        let r = length(wp);
        let ring_phase = r * ring_count
            - beats * phase_speed * 0.5;
        let ring_val = sin(ring_phase * PI);
        rings = (1.0 - smoothstep(
            0.0,
            0.2 / line_sharpness,
            abs(ring_val) * 0.5,
        )) * ring_mix;
    }

    // Combine nodal pattern and rings
    var value = max(nodal, rings);

    // Add glow around nodes proportional to
    // displacement energy
    let energy = abs_disp * abs_disp;
    value += energy * node_glow;

    // Brightness and contrast
    value = value * brightness;
    value = pow(clamp(value, 0.0, 1.0), contrast);

    // Color
    var color = vec3f(0.0);
    if color_mode == 0 {
        // Monochrome
        color = vec3f(value);
    } else if color_mode == 1 {
        // Accent on nodal lines
        let accent = hsv_to_rgb(
            vec3f(accent_hue, accent_sat, 1.0),
        );
        let gray = vec3f(value * 0.5);
        color = gray + accent * nodal * value;
    } else {
        // Displacement-mapped hue
        let hue = fract(
            accent_hue
                + displacement * 0.15
                + energy * 0.3,
        );
        let sat = accent_sat
            * (0.3 + 0.7 * nodal);
        color = hsv_to_rgb(vec3f(hue, sat, value));
    }

    if invert {
        color = 1.0 - color;
    }

    // Vignette
    let radial = length(position);
    let vig = exp(-radial * radial * vignette);
    color *= vig;

    // Film grain
    if grain_amount > 0.0001 {
        let n = hash21(
            position * 418.3
                + vec2f(beats * 0.13),
        ) - 0.5;
        color += n * grain_amount;
    }

    color = clamp(color, vec3f(0.0), vec3f(1.0));
    return vec4f(color, 1.0);
}

// ----------------------------------------------------------------
//  Noise
// ----------------------------------------------------------------

fn fbm(p: vec2f, octaves: i32) -> f32 {
    var value = 0.0;
    var amp = 0.5;
    var freq = 1.0;
    var q = p;

    for (var i = 0; i < 6; i++) {
        if i >= octaves {
            break;
        }
        value += amp * noise2(q * freq);
        freq *= 2.0;
        amp *= 0.5;
        q = vec2f(
            q.x * 0.866 - q.y * 0.5,
            q.x * 0.5 + q.y * 0.866,
        );
    }

    return value;
}

fn noise2(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let a = hash21(i + vec2f(0.0, 0.0));
    let b = hash21(i + vec2f(1.0, 0.0));
    let c = hash21(i + vec2f(0.0, 1.0));
    let d = hash21(i + vec2f(1.0, 1.0));

    return mix(a, b, u.x)
        + (c - a) * u.y * (1.0 - u.x)
        + (d - b) * u.x * u.y;
}

// ----------------------------------------------------------------
//  Helpers
// ----------------------------------------------------------------

fn hash21(p: vec2f) -> f32 {
    let h = dot(p, vec2f(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

fn correct_aspect(position: vec2f) -> vec2f {
    let w = params.a.x;
    let h = params.a.y;
    let aspect = w / h;
    var p = position;
    p.x *= aspect;
    return p;
}

fn hsv_to_rgb(hsv: vec3f) -> vec3f {
    let h = hsv.x;
    let s = hsv.y;
    let v = hsv.z;

    if s == 0.0 {
        return vec3f(v, v, v);
    }

    let i = floor(h * 6.0);
    let f = h * 6.0 - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);

    var r = 0.0;
    var g = 0.0;
    var b = 0.0;

    let sector = i % 6.0;
    if sector < 1.0 {
        r = v; g = t; b = p;
    } else if sector < 2.0 {
        r = q; g = v; b = p;
    } else if sector < 3.0 {
        r = p; g = v; b = t;
    } else if sector < 4.0 {
        r = p; g = q; b = v;
    } else if sector < 5.0 {
        r = t; g = p; b = v;
    } else {
        r = v; g = p; b = q;
    }

    return vec3f(r, g, b);
}
