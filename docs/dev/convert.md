# Sketch Conversion Playbook

This document captures the repeatable steps used to port a sketch from another
repo into this one.

## Goal

Port a sketch into `sketches` with:

- New sketch name
- Current module layout (`sketches/src/core` or `sketches/src/templates`)
- Current runtime API (`SketchConfig` + `init()` returning a sketch type)
- Optional var naming conversion (for example `a1..a4` -> `ax..aw`)

## 1. Locate source files in the old repo

The old repo and sketches are located at
`/Users/lokua/code/xtal-project/sketches`

Find the sketch triplet:

- `<name>.rs`
- `<name>.yaml`
- `<name>.wgsl`

Typical source location in old repo:

- `sketches/src/sketches/auto/`

## 2. Copy assets into target module

Copy YAML + WGSL into target location:

- `sketches/src/core/<new_name>.yaml`
- `sketches/src/core/<new_name>.wgsl`

Create/port Rust file:

- `sketches/src/core/<new_name>.rs`

## 3. Adapt to this repo's sketch API

If old sketch used direct GPU state (`GpuState`, old trait shape), convert to
current API.

Preferred default for fullscreen shader sketches:

```rust
use xtal::prelude::*;

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "<new_name>",
    display_name: "<Display Name>",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: HD_WIDTH as u32,
    h: HD_HEIGHT as u32,
    banks: <N>,
};

pub fn init() -> FullscreenShaderSketch {
    let assets = SketchAssets::from_file(file!());

    FullscreenShaderSketch::new(assets.wgsl())
        .with_control_script(assets.yaml())
}
```

## 4. Register module + sketch

1. Add module export in:

- `sketches/src/core/mod.rs`

2. Add sketch to registry list in:

- `sketches/src/main.rs`

Current pattern in `main.rs`:

- `mod core;`
- `mod templates;`
- `use core::*;`
- `use templates::*;`
- Add new sketch ident in the `sketches: [...]` section.

## 5. Convert YAML var naming (if needed)

For `a1..a4` style to `ax..aw`:

- `1 -> x`
- `2 -> y`
- `3 -> z`
- `4 -> w`

Example:

- `var: a1` -> `var: ax`
- `var: c4` -> `var: cw`

Then verify no numeric suffix vars remain:

- Search regex: `var:\s*[a-z][1-4]\b`

## 6. Shared dimension constants

If sketch should use shared HD defaults, use:

- `sketches/src/constants.rs`

```rust
#[allow(dead_code)]
pub const HD_WIDTH: u32 = 1920 / 2;
#[allow(dead_code)]
pub const HD_HEIGHT: u32 = 1080 / 2;
```

And in sketch config:

- `w: HD_WIDTH`
- `h: HD_HEIGHT`

## 7. Path note

Use:

```rust
SketchAssets::from_file(file!())
```

Reason: `from_file` resolves relative paths from the process current working
directory. Run sketches from the intended workspace root so `file!()`-relative
assets resolve correctly.

## 8. Validate

Run:

```bash
cargo check -p sketches
```

If runtime validation is needed:

```bash
RUST_LOG=xtal=info,sketches=info cargo run --release <new_name>
```

## 9. Optional cleanup pass

After migration, do targeted naming cleanup if needed (for example `xtal2` ->
`xtal`) in source code only first, then docs/lockfiles separately.

## Fast checklist

- [ ] Copied `.rs/.yaml/.wgsl`
- [ ] Renamed sketch (`name`, `display_name`, file names)
- [ ] Converted to current sketch API
- [ ] Registered in `core/mod.rs`
- [ ] Added to registry in `main.rs`
- [ ] Applied var naming conversion (`1..4` -> `x..w`) if requested
- [ ] Uses `SketchAssets::from_file(file!())`
- [ ] Uses shared constants for dimensions if desired
- [ ] `cargo check -p sketches` passes
