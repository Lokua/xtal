# Code-Defined Control Hubs

## Status

Deferred design note. Do not implement yet.

## Goal

Support using `ControlHub` directly from sketch code without a YAML control
script, while preserving the same runtime behavior expected from YAML-backed
hubs:

- UI integration
- persistence
- snapshots
- animation/modulation behavior
- MIDI/audio/OSC mapping behavior

This is a user-experience design note first, not an implementation plan.

## Problem

Today a sketch effectively assumes YAML as the control source. That works well
for hot reload and declarative control scripts, but it blocks sketches that
want to define controls directly in Rust:

```rust
let hub = ControlHubBuilder::new()
    .timing(Timing::new(ctx.bpm()))
    .hrcc(true)
    .midi_n("a", (0, 0))
    .midi_n("b", (0, 32))
    .midi_n("c", (0, 1))
    .midi_n("d", (0, 127))
    .build();
```

The important design question is not builder internals. It is how this should
work from the sketch author's point of view.

## Proposed UX Contract

A sketch should choose exactly one control source:

1. YAML mode
2. Code mode

Both should be first-class. Neither should feel like a workaround.

### YAML Mode

This is the current model:

```rust
fn control_script(&self) -> Option<PathBuf> {
    Some(...)
}
```

### Code Mode

Add a sketch hook that returns a fully built hub:

```rust
fn build_controls(&self, ctx: &Context) -> Option<ControlHub<Timing>> {
    Some(
        ControlHubBuilder::new()
            .timing(Timing::new(ctx.bpm()))
            .hrcc(true)
            .midi_n("a", (0, 0))
            .midi_n("b", (0, 32))
            .build()
    )
}
```

The runtime should treat the resulting hub the same way it treats a
YAML-constructed hub after creation.

## Required Runtime Semantics

Once a hub exists, the runtime behavior should be the same regardless of where
it came from.

That means code-defined hubs must plug into the same downstream behavior as
YAML-defined hubs:

- populate controls into the UI
- persist sketch state
- participate in snapshots
- participate in randomize and exclusions
- participate in animation/modulation/effects
- participate in mappings
- participate in MIDI/audio/OSC control flows

In other words: different authoring source, same runtime contract.

## Mutual Exclusivity Rule

A sketch must not define both control sources at once.

If a sketch implements both:

- `control_script()`
- `build_controls()`

the runtime should fail sketch load with a clear error message explaining that a
sketch must choose one control source.

This avoids undefined precedence rules and prevents subtle state mismatches.

## Hot Reload Policy

Recommended policy:

- YAML mode keeps YAML hot reload.
- Code mode does not hot reload hub definitions.

For code-defined hubs, recompilation is the reload path. That keeps the model
clear and avoids inventing a fake hot-reload layer for Rust-defined controls.

## Why This UX Is Preferred

- There is a single obvious place in a sketch to define controls.
- The runtime does not need a second execution path or a special bin pattern.
- Sketch authors can choose declarative YAML or programmatic Rust based on the
  sketch's needs.
- A sketch can begin in code mode and later move to YAML mode, or the reverse,
  without changing the rest of the runtime model.

## Non-Goals For The Deferred Work

This note does not decide:

- the exact internal shape of `ControlHubBuilder`
- whether builder methods are implemented in one parity pass or incrementally
- whether code-defined hubs support every YAML feature on day one
- any refactor of current runtime ownership beyond what is required to make the
  sketch author experience coherent

## Recommended Future Implementation Guardrails

When this is implemented later, keep these constraints:

1. Do not add a second runtime mode or a new sketch launch pattern.
2. Do not require sketch authors to care about manifest paths or asset-root
   plumbing.
3. Do not let YAML and code hubs silently coexist in one sketch.
4. Do not special-case downstream runtime behavior based on control source once
   the hub has been constructed.
5. Keep the sketch API obvious enough that the control source can be understood
   by reading the sketch file alone.

## Suggested Next Step When This Is Revisited

Before writing code, confirm the sketch trait/API surface:

- exact name of the code-mode hook
- exact return type
- whether `Context` should be passed in
- whether code-defined hubs need any explicit persistence identifier beyond the
  sketch id

After that, port the old v1 builder machinery only as needed to satisfy this UX
contract.
