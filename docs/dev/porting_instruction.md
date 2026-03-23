# Sketch Porting Instructions (main -> v2)

Use this for porting one legacy sketch at a time from `main` into
`sketches/src/core` on branch `v2`.

## Goal

Port `auto_<name>` (or another legacy sketch) as `<name>` in `core` with:

- modern v2 sketch API
- `x/y/z/w` var naming in YAML
- registry wiring in `core/mod.rs` and `sketches/src/main.rs`

## Steps

1. Locate the legacy triplet on `main`.

```bash
git ls-tree -r --name-only main | rg 'auto_<name>|<name>'
```

Expected source path pattern:

- `sketches/src/sketches/auto/<legacy>.rs`
- `sketches/src/sketches/auto/<legacy>.yaml`
- `sketches/src/sketches/auto/<legacy>.wgsl`

2. Create target files in `core`.

- `sketches/src/core/<name>.rs`
- `sketches/src/core/<name>.yaml`
- `sketches/src/core/<name>.wgsl`

3. Port Rust to v2 API.

Use this shape unless sketch needs custom graph setup:

```rust
use xtal::prelude::*;

use crate::constants::{HD_HEIGHT, HD_WIDTH};

pub static SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "<name>",
    display_name: "<Display Name>",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 124.0,
    w: HD_WIDTH,
    h: HD_HEIGHT,
    banks: <N>,
};

pub fn init() -> FullscreenShaderSketch {
    let assets = SketchAssets::from_file(file!());
    FullscreenShaderSketch::new(assets.wgsl())
        .with_control_script(assets.yaml())
}
```

Notes:

- set `banks` to cover every uniform bank used by shader/YAML (`a..h` => `8`)
- if legacy sketch has feedback/multipass behavior, use explicit graph setup
  instead of `FullscreenShaderSketch`

4. Copy WGSL and YAML from `main`.

```bash
git show main:sketches/src/sketches/auto/<legacy>.wgsl > sketches/src/core/<name>.wgsl
git show main:sketches/src/sketches/auto/<legacy>.yaml > sketches/src/core/<name>.yaml
```

5. Copy controls preset JSON from legacy storage.

```bash
git show main:sketches/storage/Controls/<legacy>_controls.json > sketches/storage/Controls/<name>_controls.json
```

6. Convert YAML vars from numeric suffixes to `x/y/z/w`.

- `1 -> x`
- `2 -> y`
- `3 -> z`
- `4 -> w`

Quick conversion:

```bash
perl -i -pe 's/(var:\s*[a-z])1\b/${1}x/g; s/(var:\s*[a-z])2\b/${1}y/g; s/(var:\s*[a-z])3\b/${1}z/g; s/(var:\s*[a-z])4\b/${1}w/g' sketches/src/core/<name>.yaml
```

Verify no numeric vars remain:

```bash
rg 'var:\s*[a-z][1-4]\b' sketches/src/core/<name>.yaml -n
```

7. Register the sketch.

- add `pub mod <name>;` in `sketches/src/core/mod.rs`
- add `<name>,` in the `Main` registry list in `sketches/src/main.rs`

8. Validate.

```bash
cargo check -p sketches
```

Optional runtime smoke test:

```bash
RUST_LOG=xtal=info,sketches=info cargo run -p sketches --release <name>
```

## Fast Checklist

- [ ] Added `core/<name>.rs/.yaml/.wgsl`
- [ ] Copied `storage/Controls/<name>_controls.json`
- [ ] Renamed sketch config (`name`, `display_name`)
- [ ] Rust uses `SketchAssets::from_file(file!())`
- [ ] `banks` matches highest used bank
- [ ] YAML vars converted to `x/y/z/w`
- [ ] Registered in `core/mod.rs`
- [ ] Added to `Main` in `sketches/src/main.rs`
- [ ] `cargo check -p sketches` passes
