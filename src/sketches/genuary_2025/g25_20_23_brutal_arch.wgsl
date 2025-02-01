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

    // scale, texture_strength, texture_scale, glitch_time
    b: vec4f,

    // echo_threshold, echo_intensity, grid_contrast, grid_size
    c: vec4f,

    // grid_border_size, ...unused
    d: vec4f,

    // corner_t_1 - corner_t_4
    e: vec4f,
    // corner_t_5 - corner_t_8
    f: vec4f,
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
        let p = correct_aspect(vert.position);
        out.clip_position = vec4f(p.xy, 0.999, 1.0);
        out.pos = vec3f(p.xy, 0.999);

        return out;
    }

    let r_x = params.a.x;
    let r_y = params.a.y;
    let r_z = params.a.z;
    let z_offset = clamp(params.a.w, -10.0, -0.5);
    let scale = params.b.x;
    let corner_t = params.d.y;

    var position = vert.position;
    let corner_sum = 
        abs(vert.center.x) + 
        abs(vert.center.y) + 
        abs(vert.center.z);
    let is_corner = corner_sum > 1.0;

    if is_corner {
        // Corner indices are binary combinations:
        // 000 = 0 = (-x, -y, -z)
        // 001 = 1 = (+x, -y, -z)
        // 010 = 2 = (-x, +y, -z)
        // 011 = 3 = (+x, +y, -z)
        // 100 = 4 = (-x, -y, +z)
        // 101 = 5 = (+x, -y, +z)
        // 110 = 6 = (-x, +y, +z)
        // 111 = 7 = (+x, +y, +z)
        let corner_index = 
            select(0, 1, vert.center.x > 0.0) +
            select(0, 2, vert.center.y > 0.0) +
            select(0, 4, vert.center.z > 0.0);

        let is_outer_vertex = 
            sign(position.x) == sign(vert.center.x) && 
            sign(position.y) == sign(vert.center.y) && 
            sign(position.z) == sign(vert.center.z);
            
        if is_outer_vertex {
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
            
            let factor = 0.5;
            let corner_axis = sign(vert.center);
            position = position + corner_axis * phase * factor;
        }
    }

    // TRS = Translate, Rotate, Scale 
    // (applied in reverse, because...that's what you do?)
    let scaled_position = position * scale;
    let positioned = scaled_position + vert.center;

    let echoed = modular_echo(positioned, vert.center);

    // var rotated = rotate_x(glitched, r_x);
    var rotated = rotate_x(echoed, r_x);
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

    let proj = mat4x4f(
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
        return vec4f(vout.layer, vout.layer, vout.layer, 1.0);
    }

    if vout.layer < FOREGROUND { 
        return vec4f(vec3f(0.13), 1.0);
    } 

    if vout.layer == FOREGROUND && DEBUG_CORNERS {
        let is_corner = 
            abs(vout.center.x) + 
            abs(vout.center.y) + 
            abs(vout.center.z) > 1.0;
        if is_corner {
            let corner_index = 
                select(0, 1, vout.center.x > 0.0) +
                select(0, 2, vout.center.y > 0.0) +
                select(0, 4, vout.center.z > 0.0);
                
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
            
            let color = (phase + 1.0) * 0.5;
            return vec4f(0.0, color, color * 0.75, 1.0);
        }
    }

    let texture_strength = params.b.y;
    let texture_scale = params.b.z;
    let grid_contrast = params.c.z;
    
    let pos = vout.local_pos;
    let light_dir = normalize(vec3f(0.5, 0.7, 0.5));
    
    var normal = vec3f(0.0);
    let eps = 0.0001;
    if abs(abs(pos.x) - 0.5) < eps {
        normal = vec3f(sign(pos.x), 0.0, 0.0);
    } else if abs(abs(pos.y) - 0.5) < eps {
        normal = vec3f(0.0, sign(pos.y), 0.0);
    } else {
        normal = vec3f(0.0, 0.0, sign(pos.z));
    }
    
    let ambient = 0.3;
    let diffuse = max(dot(normal, light_dir), 0.0);
    let light = ambient + diffuse * (1.0 - ambient); 
    
    let face_color = vec3f(
        1.0 - abs(normal.x) * 0.01,
        1.0 - abs(normal.y) * 0.01,
        1.0 - abs(normal.z) * 0.01, 
    );

    let subdivision = subdivide_face(pos, normal);
    let texture = concrete_texture(pos * texture_scale, normal, vout.center);

    let color = face_color * light * 
        (1.0 + texture * texture_strength) * 
        (grid_contrast + subdivision * (1.0 - grid_contrast));

    return vec4f(color, 1.0);
}

fn modular_echo(pos: vec3f, center: vec3f) -> vec3f {
    let scale = params.b.x;
    let time = params.b.w;
    
    let echo_threshold = params.c.x;
    let echo_intensity = params.c.y; 
    
    let cube_id = floor(center / scale * 0.5);
    
    let noise_x = hash(vec2f(cube_id.x + sin(time * 0.5), cube_id.y));
    let noise_y = hash(vec2f(cube_id.y + cos(time * 0.3), cube_id.z));
    let noise_z = hash(vec2f(cube_id.z + sin(time * 0.4), cube_id.x));
    
    var echo_offset = vec3f(0.0);
    
    let quantize = 12.0;

    if noise_x > echo_threshold {
        let step_x = floor(noise_x * quantize) / quantize;
        echo_offset.x = step_x * echo_intensity * scale;
    }
    if noise_y > echo_threshold {
        let step_y = floor(noise_y * quantize) / quantize;
        echo_offset.y = step_y * echo_intensity * scale;
    }
    if noise_z > echo_threshold {
        let step_z = floor(noise_z * quantize) / quantize;
        echo_offset.z = step_z * echo_intensity * scale;
    }
    
    let echo_fade = smoothstep(0.0, scale, length(echo_offset));
    echo_offset *= 1.0 - echo_fade;
    
    return pos + echo_offset;
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