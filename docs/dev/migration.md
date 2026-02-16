# xtal2 Migration: POC to Full Runtime

## Goal

Move `xtal2` from a shader POC into the full Xtal runtime replacement.

Required parity target:

- ControlHub features and control script behavior
- animation and timing systems
- UI integration (`xtal-ui` event flow)
- runtime sketch switching
- recording/runtime state features currently in `xtal`

Hard constraints:

- no Nannou dependency except `nannou_osc`
- no CPU drawing support in the new runtime
- no per-sketch `bin` entrypoints or `CARGO_MANIFEST_DIR` boilerplate

## Current State (POC)

What exists today in `xtal2`:

- winit + wgpu surface setup
- explicit graph (`render`, `compute`, `present`)
- shader and YAML hot reload
- runtime uniform banks
- demo sketches in `xtal2-sketches/src/bin`

What blocks parity:

- generic `Runner<S>` prevents runtime sketch switching
- sketch discovery is compile-target (`cargo run --bin ...`) based
- run loop uses `ControlFlow::Poll` with no deterministic FPS pacing
- only control defaults are applied (no full `ControlHub` runtime)
- no UI bridge, map mode, recording, or state persistence
- `var` parsing uses numeric components (`a1`..`a4`)

## Target Architecture

### Workspace direction

Target structure after cutover:

```text
xtal-project-2/
  xtal2/          # new runtime + framework (event loop, graph, control, timing)
  sketches/       # sketch modules registered at compile time
  xtal-ui/        # frontend, ideally unchanged initially
```

Notes:

- `xtal` and `xtal-macros` can coexist during migration.
- Remove old crates only after parity is verified.

### Sketch authoring model

Replace `xtal2-sketches/src/bin/*.rs` with module sketches, same style as legacy
`sketches`, but registered with categories.

Each sketch module exports:

- `SKETCH_CONFIG`
- `init(ctx: &RuntimeContext) -> SketchType`

### Registration model (categorized)

Introduce a categorized macro for runtime registry setup.

```rust
register_sketches! {
    { title: "Main", enabled: true, sketches: [blob, cloud_tunnel, marcher] },
    { title: "Auto", enabled: true, sketches: [auto_un, auto_wave_fract] },
    { title: "Dev", enabled: false, sketches: [control_script_dev, wgpu_compute_dev] },
    { title: "Genuary 2026", enabled: true, sketches: [g26_13_portrait, g26_14_dreams] },
}
```

Registry should keep both:

- `Vec<String>` flat names (for current `xtal-ui` compatibility)
- structured categories for grouped selectors:
  `{ title, enabled, sketches }`

### Asset path model (no MANIFEST boilerplate)

Add helper API that resolves assets relative to the sketch source file.

```rust
let assets = SketchAssets::from_file(file!());
let wgsl = assets.wgsl();
let yaml = assets.yaml();
```

For multipass sketches:

```rust
let assets = SketchAssets::from_file(file!());
let pass_a = assets.path("pass_a.wgsl");
let pass_b = assets.path("pass_b.wgsl");
```

No sketch should need `env!("CARGO_MANIFEST_DIR")`.

## Runtime API Direction

### SketchConfig (target)

`SketchConfig` should return to full runtime semantics:

- `name`, `display_name`
- `play_mode`
- `fps`
- `bpm`
- `w`, `h`
- `banks`
- optional `category` only if we want category at config level

Recommendation:

- Keep category in registration, not config.
- Keep config focused on per-sketch runtime behavior.

### Dynamic sketch runtime

Replace compile-time generic runner with trait-object runtime, so sketches can
switch without restarting the process.

Core requirement:

- the runtime owns `Box<dyn SketchAll>` and can rebuild graph/resources on
  switch

### UI payload compatibility

Keep current `xtal-ui` contract first:

- continue sending `Init.sketchNames: string[]`
- continue sending `SwitchSketch` and `LoadSketch`

Add optional payload field for categories (non-breaking):

- `Init.sketchCatalog` (or similar)

This allows unmodified UI to keep working while enabling grouped selectors.

## Frame Loop and Frame Controller

### Problem

`ControlFlow::Poll + request_redraw` is effectively uncapped and does not give
stable frame pacing or deterministic frame counts.

### Target behavior

Implement a dedicated `FrameClock` for `winit` runtime:

- fixed-step frame cadence (`fps` from current sketch)
- `paused`, `force_render`, `advance_single_frame` behavior
- monotonic `frame_count`
- rolling average FPS metrics
- deterministic beat timing for `FrameTiming`

### Event loop integration

Use `ControlFlow::WaitUntil(next_frame_deadline)`.

Render only when:

- frame clock says a frame is due
- forced render is requested
- single-frame advance is requested

On sketch switch:

- set new FPS immediately
- reset or remap frame counter based on switch policy
- rebuild timing providers cleanly

### Required tests

Port/adapt frame pacing tests from legacy `frame_controller`:

- exact frame interval increments
- lag catch-up behavior
- pause + single-frame advance
- FPS change during runtime

## ControlHub and Animation Port Strategy

Port these modules from `xtal/src/framework` first, then replace Nannou
references:

- `control/*`
- `motion/*`
- `frame_controller` logic (as new `frame_clock`)
- `midi`, `audio`, `osc_receiver` (keep `nannou_osc` only)

Key runtime integrations:

- call `hub.update()` each frame
- bind uniform banks from hub values
- keep snapshots, transitions, bypass, and map mode behavior

## `var` Pattern Migration (`a1` -> `ax`)

### New standard

Use bank + component-letter naming:

- `ax`, `ay`, `az`, `aw`
- `bx`, `by`, `bz`, `bw`
- ...

Mapping:

- `x -> 0`
- `y -> 1`
- `z -> 2`
- `w -> 3`

### Reserved runtime defaults

Bank `a` defaults:

- `ax`: resolution width
- `ay`: resolution height
- `az`: beats
- `aw`: reserved/free default slot (0.0 unless sketch sets it)

### Compatibility recommendation

During migration, accept both forms:

- new: `ax`
- legacy: `a1`

Log warnings for numeric style to help phase-out.

## Nannou Replacement Map

Replace Nannou references with explicit dependencies:

- `nannou::rand` -> `rand`
- `nannou::math::map_range/clamp` -> local math utils
- `nannou::winit` types -> direct `winit`
- `nannou_egui::egui::ahash::HashSet` -> `ahash` or std collections
- `nannou::App`/`Frame` runtime flow -> `xtal2` runtime context/frame
- keep `nannou_osc`

## Piece-Meal Implementation TODOs

### Phase 0: Runtime scaffold and safety rails

- [ ] Create `xtal2::runtime` modules for app, registry, frame clock, events.
- [ ] Add integration test harness that can launch headless/skipped GPU tests.
- [ ] Add feature flags to allow staged cutover (`legacy_runtime`, `xtal2`).

Exit criteria:

- Runtime compiles with both legacy and xtal2 paths enabled.

### Phase 1: Dynamic registry and switching

- [ ] Port registry concept from `xtal::runtime::registry` into `xtal2`.
- [ ] Introduce categorized registry data structure.
- [ ] Refactor runner to hold `Box<dyn SketchAll>`.
- [ ] Implement `switch_sketch()` with graph/resource rebuild.

Exit criteria:

- One process can switch between at least two sketch modules at runtime.

### Phase 2: Replace bin-based sketch entrypoints

- [ ] Add `register_sketches!` categorized macro.
- [ ] Add `SketchAssets::from_file(file!())` helper API.
- [ ] Move `xtal2-sketches/src/bin/*` demos into module-style sketches.
- [ ] Remove `env!("CARGO_MANIFEST_DIR")` from sketch authoring path.

Exit criteria:

- Sketch startup and switching works with only module registration.

### Phase 3: Deterministic frame clock

- [ ] Implement fixed-step `FrameClock` with pause/advance modes.
- [ ] Use `WaitUntil` scheduling in winit loop.
- [ ] Wire FPS changes on sketch switch.
- [ ] Add parity tests for pacing and lag behavior.

Exit criteria:

- Frame pacing tracks requested FPS and survives runtime switching.

### Phase 4: ControlHub core parity

- [ ] Port `control_hub`, config parse, dep graph, eval cache, effects.
- [ ] Hook hot-reload watchers into new runtime update path.
- [ ] Integrate snapshot/transition callbacks and bypass handling.
- [ ] Validate YAML behaviors against representative legacy scripts.

Exit criteria:

- Same control script produces matching values on legacy and xtal2 runtime.

### Phase 5: Animation and timing parity

- [ ] Port `motion::animation`, `effects`, `timing`.
- [ ] Rewire frame-based timing to new `FrameClock`.
- [ ] Port OSC/MIDI timing modes (`frame`, `osc`, `midi`, `hybrid`).

Exit criteria:

- Beat-synced animation behavior matches legacy for reference sketches.

### Phase 6: `var` pattern transition

- [ ] Update uniform parser and control var parser for `ax/ay/az/aw`.
- [ ] Keep temporary legacy parser support for `a1..a4`.
- [ ] Update templates/docs/examples to letter-components only.
- [ ] Add migration note tooling (warn or lint pass).

Exit criteria:

- New sketches use only `ax` style; old scripts still run with warnings.

### Phase 7: UI bridge parity

- [ ] Port `runtime/web_view*` IPC bridge.
- [ ] Preserve current event schema expected by `xtal-ui`.
- [ ] Add optional category payload to `Init` event.
- [ ] Verify sketch switching from UI selector.

Exit criteria:

- `xtal-ui` runs against xtal2 backend with no required frontend edits.

### Phase 8: MIDI, audio, map mode, and persistence

- [ ] Port MIDI control in/out and map mode plumbing.
- [ ] Port audio controls path and buffer sizing to new frame clock.
- [ ] Port global/sketch state serialization and restore behavior.

Exit criteria:

- Saved controls, mappings, and runtime I/O controls behave as before.

### Phase 9: Recording and performance tooling

- [ ] Port frame recorder and encode pipeline.
- [ ] Ensure sync with fixed frame clock.
- [ ] Restore average FPS and dropped-frame reporting.

Exit criteria:

- Recordings complete with expected FPS and stable sync.

### Phase 10: Cutover and cleanup

- [ ] Migrate sketch modules category by category.
- [ ] Remove or archive obsolete `xtal` runtime code.
- [ ] Optionally rename `xtal2` back to `xtal` once parity is proven.

Exit criteria:

- xtal2 runtime is default; legacy runtime is removed or frozen.

## Suggested Build Order for Fastest Value

1. Phase 1 (dynamic switching)
2. Phase 2 (registration + asset API)
3. Phase 3 (frame clock)
4. Phase 7 (UI bridge)
5. Phase 4 and 5 (ControlHub + animation)
6. Remaining parity phases

This gives a usable runtime shell early, before deep control-system porting.

## Open API Decisions (Locked)

1. Category entry shape: `{ title, enabled, sketches }`.

2. Category storage location: keep categories in registry registration, not in
   `SketchConfig`.

3. `var` compatibility window: support both `a1` and `ax` for one migration
   cycle, then drop numeric aliases.

4. UI grouping path: emit both flat `sketchNames` and structured catalog until
   UI grouping lands.

5. Crate naming: keep `xtal2` during migration; rename only after parity and
   cleanup.

## Suggestions to Improve Long-Term Quality

- Add a tiny parity test suite that runs the same control script through legacy
  and xtal2 logic and compares key values over N frames.
- Keep runtime event payloads backward compatible and additive only.
- Keep all Nannou replacements in thin adapter modules to reduce diff noise
  during porting.
- Make `SketchAssets` the only blessed asset-path mechanism.
- Treat frame clock behavior as a tested contract, not runtime glue.
