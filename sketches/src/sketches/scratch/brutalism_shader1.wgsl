const PI: f32 = 3.14159265359;
const TAU: f32 = 6.283185307179586;
const BACKGROUND: f32 = 0.0;
const FOREGROUND: f32 = 1.0;
const DEBUG: bool = false;
const DEBUG_CORNERS: bool = false;

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) center: vec3f,
    @location(2) @interpolate(flat) layer: f32
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) pos: vec3f,
    @location(1) local_pos: vec3f,
    @location(2) @interpolate(flat) layer: f32,
    @location(3) center: vec3f,
};

struct Params {
    // w, h, ..unused
    resolution: vec4f,

    // rot_x, rot_y, rot_z, z_offset
    a: vec4f,

    // scale, texture_strength, texture_scale, echo_time
    b: vec4f,

    // echo_threshold, echo_intensity, grid_contrast, grid_size
    c: vec4f,

    // grid_border_size, corner_offset, middle_translate, middle_size
    d: vec4f,

    // corner_t_1 - corner_t_4
    e: vec4f,
    // corner_t_5 - corner_t_8
    f: vec4f,

    // stag, diag, bulge, offs
    g: vec4f,

    // bg_noise, bg_noise_scale, color_spread, corner_translate
    h: vec4f,

    // twist, explode, wave, phase_twist
    i: vec4f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.layer = vert.layer;
    out.local_pos = vert.position;
    out.center = vert.center;

    if vert.layer < FOREGROUND {
        out.clip_position = vec4f(vert.position.xy * 2.0, 0.999, 1.0);
        out.pos = vec3f(vert.position.xy, 0.999);
        return out;
    }

    let r_x = params.a.x;
    let r_y = params.a.y;
    let r_z = params.a.z;
    let z_offset = clamp(params.a.w, -10.0, -0.5);
    let scale = params.b.x;
    let corner_t = params.d.y;
    let corner_translate = params.h.w;
    let middle_translate = params.d.z;
    let middle_size = params.d.w;
    let echo_intensity = params.c.y;
    let stag = params.g.x;
    let diag = params.g.y;
    let bulge = params.g.z;
    let offs = params.g.w;
    let twist_mix = params.i.x;
    let explode_mix = params.i.y;
    let wave_mix = params.i.z;
    let phase_twist_mix = params.i.w;

    var position = vert.position;

    let is_outer_vertex = 
        sign(position.x) == sign(vert.center.x) && 
        sign(position.y) == sign(vert.center.y) && 
        sign(position.z) == sign(vert.center.z);

    if is_corner(vert.center) {
        let corner_index = get_corner_index(vert.center);

        if is_outer_vertex {
            let phase = get_corner_phase(corner_index, params);
            let factor = 0.333;
            let corner_axis = sign(vert.center);
            position += corner_axis * phase * factor;
        }

        // Move corners out and back
        let dir = normalize(vert.center);
        position += dir * corner_t;
        position += dir * corner_translate;
    } else {
        // Translate out and back
        if is_outer_vertex {
            let axis = sign(vert.center) * vec3f(
                select(0.0, 1.0, abs(vert.center.x) > 0.1),
                select(0.0, 1.0, abs(vert.center.y) > 0.1),
                select(0.0, 1.0, abs(vert.center.z) > 0.1)
            );
            position += axis * middle_translate * 0.333;
        }

        // Scale middles out and back
        let dir = normalize(vert.center);
        position += dir * middle_translate;

        let primary_axis = abs(vert.center);
        let factor = 1.0 + middle_size;
        position *= vec3f(
            select(1.0, factor, primary_axis.x > 0.1),
            select(1.0, factor, primary_axis.y > 0.1),
            select(1.0, factor, primary_axis.z > 0.1)
        ); 
    }

    // TRS = Translate, Rotate, Scale 
    // (applied in reverse, because...that's what you do?)
    let scaled_position = position * scale;
    let positioned = scaled_position + vert.center;

    var p = modular_echo(positioned, vert.center, echo_intensity);
    p = staggered_offset(p, vert.center, stag);
    p = diagonal_shear(p, vert.center, diag);
    p = radial_bulge(p, vert.center, bulge);
    p = layered_offset(p, vert.center, offs);
    p = twist(p, vert.center, twist_mix);
    p = explode(p, vert.center, explode_mix);
    p = wave(p, vert.center, wave_mix);
    p = phase_twist(p, vert.center, phase_twist_mix);

    var rotated = rotate_x(p, r_x);
    rotated = rotate_y(rotated, r_y);
    rotated = rotate_z(rotated, r_z);
    let translated = vec3f(rotated.x, rotated.y, rotated.z + z_offset);

    // Perspective projection matrix
    // Field of view
    let fov = radians(45.0);
    let aspect = params.resolution.x / params.resolution.y; 
    let near = 0.1;
    let far = 100.0;

    let f = 1.0 / tan(fov / 2.0);
    let range_inv = 1.0 / (near - far);

    let proj = mat4x4(
        vec4f(f / aspect, 0.0, 0.0, 0.0),
        vec4f(0.0, f, 0.0, 0.0),
        vec4f(0.0, 0.0, far * range_inv, -1.0),
        vec4f(0.0, 0.0, near * far * range_inv, 0.0)
    );

    out.clip_position = proj * vec4f(translated, 1.0);
    out.pos = translated;

    return out;
}

@fragment
fn fs_main(vout: VertexOutput) -> @location(0) vec4f {
    if DEBUG {
        return vec4f(
            abs(vout.center.x),
            abs(vout.center.y),
            abs(vout.center.z),
            1.0
        );
    }

    if vout.layer == FOREGROUND && DEBUG_CORNERS {
        if is_corner(vout.center) {
            let corner_index = get_corner_index(vout.center);
            let phase = get_corner_phase(corner_index, params);
            let color = (phase + 1.0) * 0.5;
            return vec4f(0.0, color, color * 0.75, 1.0);
        }
    }
    
    let pos = vout.local_pos;
    var normal = vec3f(0.0);
    let eps = 0.0001;
    let world_dir = normalize(vout.pos - vout.center);
    
    if abs(abs(pos.x) - 0.5) < eps {
        normal = vec3f(sign(pos.x), 0.0, 0.0);
    } else if abs(abs(pos.y) - 0.5) < eps {
        normal = vec3f(0.0, sign(pos.y), 0.0);
    } else {
        normal = vec3f(0.0, 0.0, sign(pos.z));
    }

    let texture_strength = params.b.y;
    let texture_scale = params.b.z;
    let grid_contrast = params.c.z;

    let face_tint = 0.01;
    let face_color = vec3f(
        1.0 - abs(normal.x) * face_tint,
        1.0 - abs(normal.y) * face_tint,
        1.0 - abs(normal.z) * face_tint
    );

    let subdivision = subdivide_face(pos, normal);
    let texture = concrete_texture(pos * texture_scale, normal, vout.center);
    let light = get_light(normal);

    var foreground_color = vec3f(
        face_color * 
            light * 
            (1.0 + texture * texture_strength) * 
            (grid_contrast + subdivision * (1.0 - grid_contrast))
    );

    let bg_noise = params.h.x;
    let bg_noise_scale = params.h.y;

    var background_color = mix(
        get_bg_noise(
            vout.pos.xy, 
            foreground_color, 
            bg_noise, 
            bg_noise_scale
        ),
        get_bg_noise(
            vout.pos.xy, 
            foreground_color, 
            bg_noise, 
            100.0 - bg_noise_scale
        ),
        0.75
    );
    background_color *= vec3f(0.99, 0.98, 0.97);
    
    let border_size = 0.4;
    let is_border = 
        abs(vout.pos.x) > border_size || 
        abs(vout.pos.y) > border_size;

    let bg_nudge = 0.2;
    let final_color = select(
        select(
            background_color + bg_nudge,
            background_color + bg_nudge * 2.0,
            is_border
        ),
        foreground_color,
        vout.layer >= FOREGROUND
    );

    let processed_color = post(final_color, vout.pos.xyz);

    return vec4f(processed_color, 1.0);
}

fn post(color: vec3f, pos: vec3f) -> vec3f {
    return mix(
        color, 
        vec3f(
            color.x,
            color.y * 0.8,
            color.z * 0.49, 
        ),
        0.15
    );
}

fn wavefold(x: f32, limit: f32) -> f32 {
    let normalized = x / limit;
    let folded = abs(fract(normalized * 0.5) * 2.0 - 1.0);
    return folded * limit;
}

fn is_corner(center: vec3f) -> bool {
    let x_abs = abs(center.x);
    let y_abs = abs(center.y);
    let z_abs = abs(center.z);
    
    let epsilon = 0.0001;
    return abs(x_abs - y_abs) < epsilon && 
        abs(y_abs - z_abs) < epsilon && 
        abs(x_abs - z_abs) < epsilon;
}

fn get_light(normal: vec3f) -> f32 {
    let spread = params.h.z * 0.25;

    // light 
    if normal.x > 0.5 || normal.y > 0.5 {
        return 0.5 - spread;
    }

    // medium
    if normal.x < -0.5 || normal.z < -0.5 {
        return 0.25;
    }

    // dark
    return 0.0 + spread;
}

fn get_bg_noise(
    p: vec2f, 
    foreground_color: vec3f, 
    amount: f32, 
    scale: f32
) -> vec3f {
    let noise_value = fbm(p * scale);
    let background_color = vec3f(0.99);

    return mix(
        background_color,
        mix(background_color, foreground_color, noise_value),
        amount
    );
}

fn modular_echo(pos: vec3f, center: vec3f, mix: f32) -> vec3f {
    let scale = params.b.x;
    let time = params.b.w;
    
    let echo_threshold = params.c.x;
    
    let cube_id = floor(center / scale * 0.5);
    
    let noise_x = hash(vec2f(cube_id.x + sin(time * 0.5), cube_id.y));
    let noise_y = hash(vec2f(cube_id.y + cos(time * 0.3), cube_id.z));
    let noise_z = hash(vec2f(cube_id.z + sin(time * 0.4), cube_id.x));
    
    var echo_offset = vec3f(0.0);
    
    let quantize = 12.0;

    if noise_x > echo_threshold {
        let step_x = floor(noise_x * quantize) / quantize;
        echo_offset.x = step_x * mix * scale;
    }
    if noise_y > echo_threshold {
        let step_y = floor(noise_y * quantize) / quantize;
        echo_offset.y = step_y * mix * scale;
    }
    if noise_z > echo_threshold {
        let step_z = floor(noise_z * quantize) / quantize;
        echo_offset.z = step_z * mix * scale;
    }
    
    let echo_fade = smoothstep(0.0, scale, length(echo_offset));
    echo_offset *= 1.0 - echo_fade;
    
    return pos + echo_offset;
}

fn staggered_offset(pos: vec3f, center: vec3f, intensity: f32) -> vec3f {
    let offset = sign(center) * vec3f(
        select(0.0, 1.0, abs(center.x) > 0.25),
        select(0.0, 1.0, abs(center.y) > 0.25),
        select(0.0, 1.0, abs(center.z) > 0.25)
    );
    return pos + offset * intensity;
}

fn diagonal_shear(pos: vec3f, center: vec3f, intensity: f32) -> vec3f {
    let shear = vec3f(
        center.y * center.z,
        center.x * center.z,
        center.x * center.y
    );
    return pos + shear * intensity;
}

fn radial_bulge(pos: vec3f, center: vec3f, intensity: f32) -> vec3f {
    let dist = length(center);
    let bulge = normalize(pos) * powf(dist, 2.0);
    return pos + bulge * intensity;
}

fn layered_offset(pos: vec3f, center: vec3f, intensity: f32) -> vec3f {
    let layer = floor(abs(pos) * 4.0) * 0.5 * sign(center);
    return pos + layer * intensity;
}

fn twist(pos: vec3f, center: vec3f, mix: f32) -> vec3f {
    let is_middle = 
        abs(sign(center).x + 
        sign(center).y + 
        sign(center).z) < 2.0;
        
    if !is_middle {
        return pos;
    }

    let axis = normalize(vec3f(
        select(0.0, 1.0, abs(center.x) > 0.1),
        select(0.0, 1.0, abs(center.y) > 0.1),
        select(0.0, 1.0, abs(center.z) > 0.1)
    ));

    let angle = length(pos - center) * mix;
    let rot_pos = rotate_around_axis(pos - center, axis, angle);
    return rot_pos + center;
}

fn explode(pos: vec3f, center: vec3f, mix: f32) -> vec3f {
    let vertical_scale = abs(pos.y - center.y) * 2.0;
    let horizontal_dist = length(pos.xz - center.xz);
    let scale_factor = 1.0 + (horizontal_dist + vertical_scale) * mix;
    let centered_pos = pos - center;
    let scaled_pos = centered_pos * scale_factor;
    return scaled_pos + center;
}

fn wave(pos: vec3f, center: vec3f, mix: f32) -> vec3f {
    if !is_corner(center) {
        return pos;
    }

    let corner_index = get_corner_index(center);
    var phase = 0.0;
    
    switch corner_index {
        case 0: { phase = params.e.x; }
        case 1: { phase = params.e.y; }
        case 2: { phase = params.e.z; }
        case 3: { phase = params.e.w; }
        case 4: { phase = params.f.x; }
        case 5: { phase = params.f.y; }
        case 6: { phase = params.f.z; }
        case 7: { phase = params.f.w; }
        default: { phase = 0.0; }
    }

    let dist = length(pos - center);
    let wave = sin(dist + phase) * mix;
    let dir = normalize(pos - center);
    let wave_offset = dir * wave;
    let scale_factor = 1.0 + wave * 0.125;
    
    return center + (pos - center) * scale_factor + wave_offset;
}

fn phase_twist(pos: vec3f, center: vec3f, mix: f32) -> vec3f {
    let vertical_align = length(pos.xz - center.xz);
    let falloff = smoothstep(0.5, 0.0, vertical_align);
    
    let height_factor = pos.y - center.y;
    let twist_angle = height_factor * mix * 4.0;
    
    let local_pos = pos - center;
    let twisted_pos = vec3f(
        local_pos.x * cos(twist_angle) - local_pos.z * sin(twist_angle),
        local_pos.y,
        local_pos.x * sin(twist_angle) + local_pos.z * cos(twist_angle)
    );
    
    return center + mix(local_pos, twisted_pos, falloff);
}

fn subdivide_face(pos: vec3f, normal: vec3f) -> f32 {
    let grid_size = params.c.w;
    let grid_border_size = params.d.x;

    let proj_pos = select(
        select(
            pos.xy,
            pos.xz,
            abs(normal.y) > abs(normal.x)
        ),
        pos.yz,
        abs(normal.x) > max(abs(normal.y), abs(normal.z))
    );
    
    let cell_pos = fract(proj_pos * grid_size);
    
    let horizontal = sharp_transition(cell_pos.x, grid_border_size);
    let vertical = sharp_transition(cell_pos.y, grid_border_size);
    
    return min(horizontal, vertical);
}

fn get_corner_index(center: vec3f) -> i32 {
    return select(0, 1, center.x > 0.0) +
        select(0, 2, center.y > 0.0) +
        select(0, 4, center.z > 0.0);
}

fn get_corner_phase(corner_index: i32, params: Params) -> f32 {
    return params.e.x;
    // var phase = 0.0;
    // switch corner_index {
    //     case 0: { phase = params.e.x; }
    //     case 1: { phase = params.e.y; }
    //     case 2: { phase = params.e.z; }
    //     case 3: { phase = params.e.w; }
    //     case 4: { phase = params.f.x; }
    //     case 5: { phase = params.f.y; }
    //     case 6: { phase = params.f.z; }
    //     case 7: { phase = params.f.w; }
    //     default: { phase = 0.0; }
    // }
    // return phase;
}

fn concrete_texture(pos: vec3f, normal: vec3f, center: vec3f) -> f32 {
    let proj_pos = select(
        select(
            pos.xy + center.xy,
            pos.xz + center.xz,
            abs(normal.y) > abs(normal.x)
        ),
        pos.yz + center.yz,
        abs(normal.x) > max(abs(normal.y), abs(normal.z))
    );
    
    let large_scale = fbm(proj_pos) * 0.8;
    let medium_scale = fbm(proj_pos * 3.333) * 0.3;
    let small_scale = fbm(proj_pos * 14.746) * 0.2;
    
    return large_scale + medium_scale + small_scale;
}

fn sharp_transition(t: f32, edge_width: f32) -> f32 {
    return smoothstep(0.0, edge_width, t) * 
        smoothstep(1.0, 1.0 - edge_width, t);
}

fn correct_aspect(position: vec3f) -> vec3f {
    let w = params.resolution.x;
    let h = params.resolution.y;
    let aspect = w / h;
    return vec3f(position.x * aspect, position.y, position.z);
}

fn rotate_x(p: vec3f, radians: f32) -> vec3f {
    let c = cos(radians);
    let s = sin(radians);
    
    return vec3f(
        p.x,
        p.y * c - p.z * s,
        p.y * s + p.z * c
    );
}

fn rotate_y(p: vec3f, radians: f32) -> vec3f {
    let c = cos(radians);
    let s = sin(radians);
    
    return vec3f(
        p.x * c - p.z * s,
        p.y,
        p.x * s + p.z * c
    );
}

fn rotate_z(p: vec3f, radians: f32) -> vec3f {
    let c = cos(radians);
    let s = sin(radians);
    
    return vec3f(
        p.x * c - p.y * s,
        p.x * s + p.y * c,
        p.z
    );
}

fn rotate_around_axis(p: vec3f, axis: vec3f, angle: f32) -> vec3f {
    let c = cos(angle);
    let s = sin(angle);
    let t = 1.0 - c;
    
    let x = axis.x;
    let y = axis.y;
    let z = axis.z;
    
    let rot = mat3x3(
        vec3f(t * x * x + c, t * x * y - s * z, t * x * z + s * y),
        vec3f(t * x * y + s * z, t * y * y + c, t * y * z - s * x),
        vec3f(t * x * z - s * y, t * y * z + s * x, t * z * z + c)
    );
    
    return rot * p;
}

fn noise(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    
    // Cubic Hermine Curve for smoother interpolation
    let u = f * f * (3.0 - 2.0 * f);
    
    // Four corners
    let a = hash(i + vec2f(0.0, 0.0));
    let b = hash(i + vec2f(1.0, 0.0));
    let c = hash(i + vec2f(0.0, 1.0));
    let d = hash(i + vec2f(1.0, 1.0));
    
    // Bilinear interpolation
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Fractional Brownian Motion for layered noise
fn fbm(p: vec2f) -> f32 {
    let n_octaves = 5;
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 3.0;
    
    for(var i = 0; i < n_octaves; i++) {
        value += amplitude * noise(p * frequency);
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    
    return value;
}

fn hash(p: vec2f) -> f32 {
    let p3 = fract(vec3f(p.xyx) * 0.13);
    let p4 = p3 + vec3f(7.0, 157.0, 113.0);
    return fract(dot(p4, vec3f(268.5453123, 143.2354234, 424.2424234)));
}

fn powf(x: f32, y: f32) -> f32 {
    return sign(x) * exp(log(abs(x)) * y);
}

fn random_normal(seed: u32, std_dev: f32) -> f32 {
    let u1 = rand_pcg(seed);
    let u2 = rand_pcg(seed + 1u);
    let mag = sqrt(-2.0 * log(u1));
    let z0 = mag * cos(2.0 * PI * u2);
    return std_dev * z0;
}

fn rand_pcg(seed: u32) -> f32 {
    var state = seed * 747796405u + 2891336453u;
    var word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    var result = (word >> 22u) ^ word;
    return f32(result) / 4294967295.0;
}