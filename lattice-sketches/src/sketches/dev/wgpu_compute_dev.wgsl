struct ComputeParams {
    n_segments: u32,
    points_per_segment: u32,
    noise_scale: f32,
    angle_variation: f32,
}

struct Point {
    pos: vec2<f32>,
    _padding: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> params: ComputeParams;

@group(0) @binding(1)
var<storage, read> input_points: array<Point>;

@group(0) @binding(2)
var<storage, read_write> output_points: array<Point>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let total_points = params.n_segments * params.points_per_segment;
    
    if (idx >= total_points) {
        return;
    }
    
    // Determine which segment this point belongs to
    let segment_idx = idx / params.points_per_segment;
    let point_in_segment = idx % params.points_per_segment;
    let t = f32(point_in_segment) / f32(params.points_per_segment);
    
    // Get the segment's start and end points
    let start = input_points[segment_idx].pos;
    let end = input_points[segment_idx + 1u].pos;
    
    // Interpolate along the line segment
    let base_point = lerp(start, end, t);
    
    // Calculate perpendicular direction
    let segment_dir = normalize(end - start);
    let perp_dir = vec2<f32>(-segment_dir.y, segment_dir.x);
    
    // Generate noise using seed based on position
    let noise = random_normal(
        idx, 
        0.0,
        params.noise_scale
    );
    
    // Add random angle variation
    let angle_offset = random_normal(
        idx + total_points,  // Different seed than noise
        0.0,
        params.angle_variation
    );
    
    // Rotate perpendicular direction by random angle
    let cos_theta = cos(angle_offset);
    let sin_theta = sin(angle_offset);
    let rotated_dir = vec2<f32>(
        perp_dir.x * cos_theta - perp_dir.y * sin_theta,
        perp_dir.x * sin_theta + perp_dir.y * cos_theta
    );
    
    // Calculate final position with noise applied in the rotated direction
    let final_pos = base_point + rotated_dir * noise;
    
    // Store the result
    output_points[idx].pos = final_pos;
    output_points[idx]._padding = vec2<f32>(0.0, 0.0);
}

// Basic random number generation (PCG)
fn rand_pcg(seed: u32) -> f32 {
    var state = seed * 747796405u + 2891336453u;
    var word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    var result = (word >> 22u) ^ word;
    return f32(result) / 4294967295.0;
}

// Box-Muller transform for normal distribution
fn random_normal(seed: u32, mean: f32, stddev: f32) -> f32 {
    let u1 = rand_pcg(seed);
    let u2 = rand_pcg(seed + 1u);
    
    let mag = sqrt(-2.0 * log(u1));
    let z0 = mag * cos(6.28318530718 * u2);
    
    return mean + stddev * z0;
}

// Interpolate between two points
fn lerp(a: vec2<f32>, b: vec2<f32>, t: f32) -> vec2<f32> {
    return a + (b - a) * t;
}