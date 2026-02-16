# xtal2 Migration: Runtime Parity Backlog

## Goal

Ship `xtal2` as the full runtime replacement for `xtal` with the same
user-facing behavior for:

- sketch switching at runtime
- ControlHub-driven animation/control scripts
- xtal-ui integration and event flow
- recording/performance tooling
- state/mapping persistence

## Hard Constraints

- Keep `nannou_osc`, remove all other Nannou runtime dependencies.
- No CPU drawing support.
- No per-sketch `bin` entrypoint pattern.
- Sketch asset lookup should use `SketchAssets` (no manifest boilerplate).

## Current Snapshot (2026-02-16)

### Foundation that is already in place

- Dynamic runtime registry with categorized sketches.
- Runtime sketch switching (`Box<dyn Sketch>` in the runner).
- `register_sketches!` and module-based sketch startup in `xtal2-sketches`.
- `SketchAssets::from_file(file!())` path flow.
- `FrameClock` and `WaitUntil` scheduling.
- ControlHub + shader/yaml hot reload integration.
- `web_view_process` is wired and launched by `run_registry_with_web_view`.
- `ax/ay/az/aw` var pattern is supported (legacy numeric aliases still parse).
- Runtime source layout cleanup is done (`xtal2/src` root is minimal).

### What is not complete yet

- UI event parity is only partial.
- Performance mode is not stateful in runtime and does not gate window
  resize/position behavior.
- MIDI/audio/map-mode/persistence paths are not ported to xtal2 runtime.
- Recording pipeline is not ported.
- Full behavioral parity tests against legacy runtime are missing.

## Active Phase Backlog

### Phase 4: ControlHub parity hardening

- [ ] Validate xtal2 YAML behavior against representative legacy scripts.
- [ ] Add parity fixtures that compare values over N frames between xtal and
  xtal2 for the same control script.
- [ ] Remove panic paths in hot code paths (for example remaining
  `unimplemented!()` branches) or convert them to explicit errors.
- [ ] Re-check snapshot and transition callback behavior against legacy UI flow.

Exit criteria:

- Same control scripts produce matching values and lifecycle events in xtal2 and
  xtal.

### Phase 5: Animation and timing parity

- [ ] Wire external transport timing sources fully (OSC/MIDI/hybrid runtime
  listeners).
- [ ] Port tap-tempo behavior and BPM update flow to runtime state.
- [ ] Validate timing reset semantics (MIDI start/continue/stop) against legacy
  behavior.

Exit criteria:

- Beat-synced animation and transport-driven timing match legacy behavior on
  reference sketches.

### Phase 6: var migration completion

- [ ] Add explicit deprecation warnings/tooling for numeric aliases (`a1..a4`).
- [ ] Set and document a removal milestone for numeric aliases.
- [ ] Keep docs/examples/templates fully on `ax/ay/az/aw` naming.

Exit criteria:

- `ax/ay/az/aw` is the only documented pattern and migration off numeric aliases
  is enforced by tooling.

### Phase 7: UI bridge parity (critical)

The protocol enum is present, but command handling is incomplete.

#### 7.1 Incoming UI event coverage

Currently mapped to runtime commands:

- `Advance`
- `Paused(bool)`
- `Quit`
- `SwitchSketch(String)`
- `UpdateControlBool`
- `UpdateControlFloat`
- `UpdateControlString`

Still missing command mapping and runtime handling:

- [ ] `PerfMode(bool)`
- [ ] `ToggleFullScreen`
- [ ] `ToggleMainFocus`
- [ ] `Tap`
- [ ] `TapTempoEnabled(bool)`
- [ ] `TransitionTime(f32)`
- [ ] `Randomize(Vec<String>)`
- [ ] `Reset`
- [ ] `Save(Vec<String>)`
- [ ] `SnapshotStore(String)`
- [ ] `SnapshotRecall(String)`
- [ ] `SnapshotDelete(String)`
- [ ] `MappingsEnabled(bool)`
- [ ] `Mappings(...)` receive path
- [ ] `CurrentlyMapping(String)`
- [ ] `CommitMappings`
- [ ] `RemoveMapping(String)`
- [ ] `SendMidi`
- [ ] `Hrcc(bool)`
- [ ] `ChangeAudioDevice(String)`
- [ ] `ChangeMidiClockPort(String)`
- [ ] `ChangeMidiControlInputPort(String)`
- [ ] `ChangeMidiControlOutputPort(String)`
- [ ] `ChangeOscPort(u16)`
- [ ] `ChangeDir(UserDir)` + `ReceiveDir(UserDir, String)`
- [ ] `OpenOsDir(OsDir)`
- [ ] `CaptureFrame`
- [ ] `QueueRecord`
- [ ] `StartRecording`
- [ ] `StopRecording`
- [ ] `ClearBuffer`

#### 7.2 Outgoing runtime -> UI event coverage

Already emitted:

- `Init`
- `LoadSketch`
- `Paused`
- `HubPopulated`
- `UpdatedControls`
- `SnapshotSequenceEnabled`

Still missing (or not yet driven by real runtime state):

- [ ] `AverageFps(f32)`
- [ ] `Bpm(f32)` updates
- [ ] `Mappings(...)`
- [ ] `MappingsEnabled(bool)`
- [ ] `CurrentlyMapping(String)`
- [ ] `Encoding(bool)`
- [ ] `StartRecording` and `StopRecording` status events
- [ ] `PerfMode(bool)` state echo/confirmation
- [ ] Directory update confirmations (`ReceiveDir`) from persisted state path

#### 7.3 Performance mode parity (blocking issue)

- [ ] Add runtime perf-mode state and `RuntimeCommand::SetPerfMode(bool)`.
  File touchpoints:
  `xtal2/src/runtime/events.rs`, `xtal2/src/runtime/web_view.rs`,
  `xtal2/src/runtime/app.rs`.
- [ ] Stop hardcoding `LoadSketch.perf_mode = false`; emit actual runtime state.
  File touchpoint: `xtal2/src/runtime/app.rs`.
- [ ] Gate main window resize/reposition on sketch switch when perf mode is on.
  File touchpoint: `xtal2/src/runtime/app.rs`.
- [ ] Ensure webview control window placement behavior respects perf mode without
  breaking control sizing expectations.
  File touchpoint: `xtal2/src/bin/web_view_process.rs`.

#### 7.4 UI bridge parity tests

- [ ] Expand phase-7 tests beyond `SwitchSketch` to cover full mapping set.
- [ ] Add serialization/deserialization golden tests for all UI payload types.
- [ ] Add end-to-end smoke test for webview process message routing.

Exit criteria:

- xtal-ui actions drive runtime behavior equivalently to legacy xtal and all
  expected status events round-trip.

### Phase 8: MIDI, audio, map mode, and persistence

- [ ] Port map-mode runtime (`currently_mapping`, commit/remove, duplicate checks).
- [ ] Port MIDI control in/out lifecycle and hrcc behavior.
- [ ] Port MIDI clock and OSC port runtime controls.
- [ ] Port audio device switching and runtime restarts.
- [ ] Port global settings serialization/restore.
- [ ] Port sketch state serialization/restore (snapshots, mappings, exclusions).
- [ ] Hook persistence into runtime sketch-switch lifecycle.

Exit criteria:

- Mappings, device settings, snapshots, and sketch/global state persist and
  restore like legacy xtal.

### Phase 9: Recording and performance tooling

- [ ] Port still frame capture flow.
- [ ] Port queued recording, start/stop recording, and encode lifecycle.
- [ ] Port frame recorder integration with fixed frame clock.
- [ ] Restore performance telemetry (`AverageFps`, dropped frame reporting).
- [ ] Verify recording correctness under pause/advance/switch scenarios.

Exit criteria:

- Recording and capture behavior match legacy runtime and remain frame-accurate.

### Phase 10: Cutover and cleanup

- [ ] Remove or archive obsolete legacy runtime code once parity is proven.
- [ ] Decide final crate naming (`xtal2` -> `xtal`) after parity sign-off.
- [ ] Add final migration notes for sketch authors and UI maintainers.

Exit criteria:

- xtal2 is the default and only runtime path for normal usage.

## Immediate Next-Sprint Order

1. Phase 7.1 + 7.3: complete perf-mode and missing UI command routing.
2. Phase 7.2: emit missing runtime status events needed by xtal-ui.
3. Phase 8: finish map-mode/MIDI/audio/persistence paths.
4. Phase 9: recording + telemetry parity.
5. Phase 4/5 parity fixtures and final behavior validation.

## Locked API Decisions

1. Category registration shape is `{ title, enabled, sketches }`.
2. Category lives in registry registration, not `SketchConfig`.
3. Keep both flat `sketchNames` and structured catalog for UI compatibility.
4. `SketchAssets` is the standard sketch asset path mechanism.
5. `ax/ay/az/aw` is the forward var pattern.
