// Terrain Scanner — top-down animated contour map
// FBM heightfield with scrolling, contour lines, and
// periodic waves that distort the terrain.

const PI: f32 = 3.14159265359;
const TAU: f32 = 6.283185307179586;
const CONTOUR_OCTAVES: i32 = 6;

struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) pos: vec2f,
};

struct Params {
    // w, h, beats, scroll_speed
    a: vec4f,
    // terrain_scale, contour_count, line_sharpness,
    // line_thickness
    b: vec4f,
    // warp_amount, warp_speed, ridge_mix, octaves
    c: vec4f,
    // brightness, contrast, wave_amp, wave_freq
    d: vec4f,
    // wave_speed, invert, grain, vignette
    e: vec4f,
    // scroll_angle, color_mode, accent_hue, accent_sat
    f: vec4f,
    // accent_intensity, terrain_speed, fill_mix,
    // band_count
    g: vec4f,
    // wave_angle, _, _, _
    h: vec4f,
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
    let scroll_speed = params.a.w;
    let terrain_scale = max(params.b.x, 0.1);
    let contour_count = max(params.b.y, 1.0);
    let line_sharpness = max(params.b.z, 0.1);
    let line_thickness = clamp(
        params.b.w, 0.001, 0.5,
    );
    let warp_amount = params.c.x;
    let warp_speed = params.c.y;
    let ridge_mix = clamp(params.c.z, 0.0, 1.0);
    let octaves = clamp(i32(params.c.w), 1, 8);
    let brightness = max(params.d.x, 0.0);
    let contrast = max(params.d.y, 0.1);
    let wave_amp = params.d.z;
    let wave_freq = max(params.d.w, 0.0);
    let wave_speed = params.e.x;
    let invert = params.e.y > 0.5;
    let grain_amount = max(params.e.z, 0.0);
    let vignette = max(params.e.w, 0.0);
    let scroll_angle = params.f.x;
    let color_mode = i32(params.f.y);
    let accent_hue = params.f.z;
    let accent_sat = clamp(params.f.w, 0.0, 1.0);
    let accent_intensity = max(params.g.x, 0.0);
    let terrain_speed = params.g.y;
    let fill_mix = clamp(params.g.z, 0.0, 1.0);
    let band_count = max(params.g.w, 2.0);
    let wave_angle = params.h.x;

    let p = correct_aspect(position);

    // Scrolling offset
    let scroll_dir = vec2f(
        cos(scroll_angle),
        sin(scroll_angle),
    );
    let scroll = scroll_dir * beats * scroll_speed;
    let terrain_t = beats * terrain_speed;

    // Terrain sample position
    let tp = p * terrain_scale + scroll;

    // Domain warping
    var warped = tp;
    if warp_amount > 0.001 {
        let wt = terrain_t * warp_speed;
        let wx = fbm(
            tp + vec2f(5.2, 1.3)
                + vec2f(wt * 0.11, -wt * 0.07),
            3,
        );
        let wy = fbm(
            tp + vec2f(1.7, 9.2)
                + vec2f(-wt * 0.09, wt * 0.13),
            3,
        );
        warped = tp + vec2f(wx, wy) * warp_amount;
    }

    // Height value
    let raw_height = fbm(
        warped + vec2f(terrain_t * 0.05),
        octaves,
    );

    // Optional ridge noise blend
    var height = raw_height;
    if ridge_mix > 0.001 {
        let ridge = ridge_fbm(
            warped + vec2f(
                terrain_t * 0.03,
                -terrain_t * 0.02,
            ),
            octaves,
        );
        height = mix(raw_height, ridge, ridge_mix);
    }

    // Periodic waves moving through the terrain.
    // Projects position onto a wave direction axis,
    // producing travelling sine ripples that add to
    // the height field.
    if wave_amp > 0.001 {
        let wave_dir = vec2f(
            cos(wave_angle),
            sin(wave_angle),
        );
        let proj = dot(p, wave_dir);
        let wave_phase = beats * wave_speed;
        let wave = sin(
            proj * wave_freq - wave_phase,
        ) * wave_amp;
        // Second harmonic at offset angle for depth
        let wave_dir2 = vec2f(
            cos(wave_angle + 1.2),
            sin(wave_angle + 1.2),
        );
        let proj2 = dot(p, wave_dir2);
        let wave2 = sin(
            proj2 * wave_freq * 1.618
                - wave_phase * 0.7,
        ) * wave_amp * 0.4;
        height += wave + wave2;
    }

    // Contour lines
    let scaled = height * contour_count;
    let contour_fract = fract(scaled);
    let contour_dist = min(
        contour_fract,
        1.0 - contour_fract,
    );
    let line_val = 1.0 - smoothstep(
        0.0,
        line_thickness,
        contour_dist,
    );
    let sharp_line = pow(line_val, line_sharpness);

    // Banded fill between contour lines
    let band_idx = floor(height * band_count);
    let band_val = band_idx
        / max(band_count - 1.0, 1.0);

    // Combine line and fill
    let topo_val = mix(
        band_val,
        sharp_line,
        1.0 - fill_mix,
    );

    // Compose brightness
    var value = topo_val * brightness;
    value = pow(clamp(value, 0.0, 1.0), contrast);

    // Apply color
    var color = vec3f(0.0);
    if color_mode == 0 {
        // Monochrome
        color = vec3f(value);
    } else if color_mode == 1 {
        // Single accent color on lines
        let base_gray = vec3f(value * 0.6);
        let accent = hsv_to_rgb(
            vec3f(accent_hue, accent_sat, 1.0),
        );
        let line_color = accent * sharp_line
            * accent_intensity;
        color = base_gray + line_color;
    } else {
        // Height-mapped color
        let hue = fract(
            accent_hue + height * 0.15
                + band_val * 0.1,
        );
        let sat = accent_sat
            * (0.3 + 0.7 * sharp_line);
        let val = value;
        color = hsv_to_rgb(vec3f(hue, sat, val));
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

    for (var i = 0; i < CONTOUR_OCTAVES; i++) {
        if i >= octaves {
            break;
        }
        value += amp * noise2(q * freq);
        freq *= 2.0;
        amp *= 0.5;
        // Rotate between octaves to reduce
        // axis-alignment artifacts
        q = vec2f(
            q.x * 0.866 - q.y * 0.5,
            q.x * 0.5 + q.y * 0.866,
        );
    }

    return value;
}

fn ridge_fbm(p: vec2f, octaves: i32) -> f32 {
    var value = 0.0;
    var amp = 0.5;
    var freq = 1.0;
    var q = p;

    for (var i = 0; i < CONTOUR_OCTAVES; i++) {
        if i >= octaves {
            break;
        }
        let n = noise2(q * freq);
        // Ridge: fold the noise around 0.5
        let ridge = 1.0 - abs(n * 2.0 - 1.0);
        value += amp * ridge * ridge;
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
