# Xtal Project Context

## Project Overview

Xtal is a Rust creative coding framework built on [Nannou](https://nannou.cc/)
for creating generative art and audio-visual compositions. It emphasizes live
performance capabilities with beat-synchronized animation, MIDI/OSC integration,
and hot-reloadable controls.

## Key Architecture Components

### 1. ControlHub and Control Scripting

The `ControlHub` is the central system for managing all sketch parameters.
Instead of hardcoding values or recompiling for every parameter change, Xtal
uses YAML-based control scripts that hot-reload at runtime.

**Core Pattern:**

```rust
// In sketch init
let hub = ControlHub::from_path(
    to_absolute_path(file!(), "sketch_name.yaml"),
    Timing::new(ctx.bpm()),
);

// In update method
let param_value = self.hub.get("param_name");
```

**Control scripts live alongside sketches** (same directory as the .rs file) and
define:

- UI controls (sliders, checkboxes, selects)
- Animations (ramp, triangle, random, automate)
- MIDI/OSC/Audio mappings
- Effects chains (math, map, wave_folder, etc.)
- Parameter modulation (using `$reference` syntax)

**Important:** See `docs/control_script_reference.md` for complete control
scripting documentation.

### 2. Animation System

The animation module (`xtal/src/framework/motion/`) provides beat-synchronized
animation primitives:

- `ramp(beats)` - sawtooth wave over N beats
- `tri(beats)` - triangle wave over N beats
- `random(beats, range)` - stepped random values
- `random_slewed(beats, range, slew, phase)` - smoothed random
- `automate(breakpoints, mode)` - powerful keyframe system

**Critical behavior for `automate`:**

- Breakpoints define transitions between values
- `kind: step` holds the value until the next breakpoint
- `kind: ramp` interpolates FROM current value TO next breakpoint's value over
  the time span
- `kind: end` marks the loop point
- The `value` field at a `ramp` breakpoint is the TARGET value that will be
  reached by the time the animation reaches the NEXT breakpoint's position

See the animation module source code for full details.

### 3. Shader Integration

Most sketches use WGSL shaders via the `gpu::GpuState` wrapper. Parameters are
passed as uniform buffers.

**Two approaches:**

1. **Using the `#[uniforms]` macro** (simpler, recommended):

```rust
#[uniforms(banks = 4)]
struct ShaderParams {}

// Auto-populates from ControlHub vars a1-d4
let mut params = ShaderParams::from((&wr, &self.hub));
params.set("a3", custom_value);
```

2. **Manual struct with Pod/Zeroable** (full control):

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct ShaderParams {
    resolution: [f32; 4],
    colors: [f32; 4],
    // ... more fields
}
```

Shaders are hot-reloaded when modified. See `genuary_2025/` and `genuary_2026/`
sketches for examples.

### 4. Sketch Structure

All sketches follow this pattern:

```rust
use nannou::prelude::*;
use xtal::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "sketch_name",
    display_name: "Display Name",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 120.0,
    w: 700,
    h: 700,
};

#[derive(SketchComponents)]
pub struct MySketch {
    hub: ControlHub<Timing>,
    // other components...
}

pub fn init(app: &App, ctx: &Context) -> MySketch {
    // Initialize ControlHub, GPU state, etc.
}

impl Sketch for MySketch {
    fn update(&mut self, app: &App, update: Update, ctx: &Context) {
        // Update logic, read controls, calculate params
    }

    fn view(&self, app: &App, frame: Frame, ctx: &Context) {
        // Render
    }
}
```

## Code Style Guidelines

1. **Indentation**: Use consistent block-level indentation only. DO NOT align
   parameters, struct fields, or array elements to match previous lines. Each
   nested level gets one indent, period.
   ```rust
   // GOOD - consistent block indentation
   let params = ShaderParams {
       resolution: [wr.w(), wr.h(), 0.0, 0.0],
       colors: [1.0, 0.0, 0.0, 1.0],
       values: [x, y, z, w],
   };

   // BAD - aligning to match previous line lengths
   let params = ShaderParams {
       resolution: [wr.w(), wr.h(), 0.0, 0.0],
       colors:     [1.0,    0.0,    0.0,  1.0],  // Don't do this!
       values:     [x,      y,      z,    w],    // Don't do this!
   };
   ```

2. **Line length**: Keep lines under 80 characters where practical
3. **Imports**: Group by std/external/internal, separated by blank lines
4. **Naming**:
   - snake_case for functions, variables, modules
   - PascalCase for types, traits, enums
   - SCREAMING_SNAKE_CASE for constants
5. **Formatting**: Use `rustfmt` defaults (already configured)
6. **Comments**: Use `//` for line comments, `///` for doc comments
7. **Control variables**: Prefer using `var: a1`, `var: b2` etc. in YAML for
   shader-bound parameters (matches the uniform bank convention)

## Example Sketches

**Best references for learning patterns:**

- `sketches/src/sketches/genuary_2025/g25_14_black_and_white.rs` - Complex
  automate breakpoints, manual shader params
- `sketches/src/sketches/genuary_2025/g25_20_23_brutal_arch.rs` - Extensive
  control scripting with hold-ramp patterns
- `sketches/src/sketches/genuary_2026/g26_12_boxes.rs` - Simple shader-based
  sketch with uniforms macro

Control script examples in the same directories show various animation patterns.

## Important Constraints

### Parameter Modulation

- Only `f32` scalar fields can be modulated with `$reference` syntax
- List parameters like `range: [f32; 2]` CANNOT be modulated
- `effect` and `mod` type mappings CANNOT be parameter modulation sources
- Only animations and UI controls can be modulation sources

See `docs/control_script_reference.md` line 996 for details.

## Key Directories

- `xtal/src/framework/` - Core framework code (motion, control, gpu)
- `sketches/src/sketches/` - Example sketches
- `sketches/src/sketches/genuary_2025/` - Comprehensive sketch examples
- `sketches/src/sketches/genuary_2026/` - Latest sketch examples
- `docs/` - Framework documentation
- `xtal-ui/` - Web-based control UI (TypeScript/React)

## Development Tips

1. Use `cargo run --release` for performance (debug builds are slow)
2. Edit YAML control scripts while running - they hot-reload
3. Edit WGSL shaders while running - they hot-reload
4. Check `docs/control_script_reference.md` when working with control scripts
5. Run `cargo doc --package xtal --open` for full API documentation

## Git Workflow

**IMPORTANT:** Work directly on the user's current branch. DO NOT create your
own branches.

- Make commits when requested by the user
- DO NOT push unless explicitly requested
- Use whatever branch the user currently has checked out
