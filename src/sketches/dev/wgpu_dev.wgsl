// Defines what data each vertex receives
// `@location(0)` matches the vertex buffer we set up in Rust
// (our `VERTICES` array)
struct VertexInput {
    @location(0) position: vec2<f32>,
};

// Defines what the vertex shader sends to the fragment shader 
// @builtin(position) is special - it's the final screen position
// @location(0) is our custom UV coords used for coloring
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Matches our `ShaderParams` struct in Rust.
// Must match the struct's memory layout exactly
struct Params {
    time: f32,
    mix: f32,
    grid_mult: f32,
}

// Makes our parameters available to the shader 
// `group(0)` matches the bind group index in `set_bind_group(0, ...)`
// `binding(0)` matches the binding number in the layout 
// `uniform` means this data is read-only and shared across all shader invocations
@group(0) @binding(0)
var<uniform> params: Params;

// Marks this as the vertex shader entry point 
@vertex
// Takes our 2D vertex position and converts it to a 4D homogeneous coordinate 
// (needed for graphics pipeline)
// Converts position from -1..1 range to 0..1 range for UV coordinates
// - U and V are simply names for the two-dimensional coordinates used in texture mapping.
// - They are analogous to X and Y in a 2D Cartesian plane but are used in 
//   the context of textures, as X, Y, Z are reserved for 3D space.
fn vs_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // The 0.0 is for Z depth, 1.0 is for W (perspective division) 
    out.position = vec4<f32>(vert.position, 0.0, 1.0);
    
    // Convert from NDC to "UV Coordinates" (the standard for texture mapping).
    // This:
    //      (-1,1)      (1,1)
    //         ┌─────────┐
    //         │         │
    //         │         │
    //         │         │
    //         └─────────┘
    //      (-1,-1)     (1,-1)
    //
    // Becomes:
    //      (0,1)       (1,1)
    //         ┌─────────┐
    //         │         │
    //         │         │
    //         │         │
    //         └─────────┘
    //      (0,0)       (1,0)
    out.uv = vert.position * 0.5 + 0.5;
    return out;
}

// Marks this as the fragment shader entry point
@fragment
// Receives the UV coordinates we calculated in vertex shader
// Returns a color (RGBA) for this pixel
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let circle = circle_pattern(uv);
    let grid = grid_pattern(uv);
    let final_color = mix(circle, grid, params.mix);
    return vec4<f32>(final_color, 1.0);
}

// Simple circle that radiates from center
fn circle_pattern(uv: vec2<f32>) -> vec3<f32> {
    // `vecNf is alias for vecN<f32>
    let center = vec2f(0.5, 0.5);

    // Distance from center (0 at center, ~0.7 at corners)
    let dist = length(uv - center); 

    let color_value = params.time;
    return vec3<f32>(dist, color_value, color_value);
}

// Simple grid
fn grid_pattern(uv: vec2<f32>) -> vec3<f32> {
    let grid_size = params.grid_mult;

    // Multiply UV to get more grid cells
    let pos = uv * grid_size;        

    return vec3<f32>(

        // Creates vertical stripes
        fract(pos.x),               

        // Creates horizontal stripes
        fract(pos.y),                
        
        // Blue channel always 0
        1.0                        
    );
}