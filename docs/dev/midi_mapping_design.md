# MIDI Mapping Design: Current State and Alternatives

## What the proxy pattern does today

The goal is simple: let a UI slider be overridden by a live MIDI CC at
runtime without requiring a YAML change or sketch restart.

The current implementation achieves this through a proxy indirection:

1. **Learn phase** (`MapMode`): a temporary MIDI listener waits for the
   user to wiggle a CC. When received it stores `slider_name -> (ch, cc)`
   in `MapMode.state.mappings`.

2. **Commit phase** (`CommitMappings`): for every `(name, ch, cc)` mapping,
   a new `MidiControlConfig` is inserted into `hub.midi_controls` under the
   synthetic key `"<name>__slider_proxy"`. The range is taken from the
   matching UI slider config. The permanent MIDI listener (`MidiControls`)
   picks up the CC and writes the scaled value into `state` under the proxy
   key.

3. **Read phase** (`hub.get(name)`): before looking up `name`, `get` checks
   whether `"<name>__slider_proxy"` exists in `midi_controls` and, if
   `midi_proxies_enabled` is true, substitutes the proxy key for the
   original name. The MIDI value is returned instead of the UI slider value.

4. **Persistence**: on `Save`, `hub.midi_controls` is serialized including
   all proxy entries. The mapping table (`name -> ch/cc`) is also saved.
   On restore, `setup_midi_mappings` reconstructs the proxy entries from
   the saved mapping table.

5. **Snapshot**: `create_snapshot` includes proxy values under their
   `__slider_proxy` keys. `recall_snapshot` handles them in the
   `midi_controls.has(name)` branch.

---

## Where the complexity comes from

The proxy pattern works but generates complexity at every layer it touches:

### In `ControlHub.get` and `run_dependencies`
The `get` and `run_dependencies` methods both contain the same proxy
substitution logic — check for `__slider_proxy` in `midi_controls`, swap the
key. They have to stay in sync. The dep graph walk also has to perform this
swap for modulation chains that reference a mapped slider.

### In `get_raw`
`get_raw` has to handle the case where the incoming name *is already* a proxy
name (i.e. it was passed from `get`). It must derive `unproxied_name` to
correctly look up the dep graph and decide where to cache the result.

### In `CommitMappings` (app.rs)
The commit handler is the most complex part of the whole system. It:
- Removes orphaned proxies (proxies whose UI slider no longer exists)
- Checks for and skips unchanged mappings
- Looks up slider ranges from `ui_controls` to build configs
- Inserts proxy configs into `midi_controls`
- Restarts the MIDI listener to pick up the new config set
- Reports missing slider ranges as errors

This is a significant amount of multi-step mutation spread across `app.rs`
touching both `map_mode` and `hub.midi_controls` at the same time.

### In serialization
`TransitorySketchState.setup_midi_mappings` reconstructs proxies from the
saved mapping table on disk. It mirrors the `CommitMappings` logic but lives
in a different place. If one changes, the other might not.

### In snapshot logic
Snapshots are keyed by the proxy name (`ax__slider_proxy`), not the UI name
(`ax`). This means snapshot values are silently split: some keys are plain UI
names, some are proxy names. `recall_snapshot` dispatches on
`midi_controls.has(name)` vs `ui_controls.has(name)` to route them. This is
invisible to users (snapshots should be an abstraction over "current values")
but is a hidden variant in the data model.

### In the UI event flow
`Mappings` (the `name -> ch/cc` table) is maintained in three places
simultaneously: `MapMode.state`, `SketchUiState.mappings`, and
`hub.midi_controls` (implicitly, as configs). Keeping them in sync requires
careful ordering across `ReceiveMappings`, `CommitMappings`, `RemoveMapping`,
`SendMappings`, and `emit_web_view_load_sketch`.

### The `__slider_proxy` string suffix
The naming convention is load-bearing. It flows through every layer:
`CommitMappings`, `get`, `run_dependencies`, `get_raw`, `create_snapshot`,
`recall_snapshot`, `setup_midi_mappings`. Any code that iterates control
names must filter or translate proxy names explicitly.

---

## Alternative designs

### Option A: Override table in `ControlHub` (recommended)

Instead of injecting phantom entries into `MidiControls`, maintain a
first-class `midi_overrides: HashMap<String, f32>` directly on the hub,
keyed by the plain UI slider name. `MidiControls` receives CC messages and
writes the scaled value into this table (just as it does into its own
`State` today, but using the slider name rather than a proxy name).

```
hub.midi_overrides: HashMap<String, f32>
  "ax" -> 0.73   // set by MIDI listener; uses slider name directly
```

`hub.get("ax")` checks `midi_overrides.get("ax")` first, before consulting
`ui_controls`. No name mangling, no proxy existence check, no key swap.

**Changes required:**

- `MidiControls` needs to know the UI name to write to the override table.
  This can be done by storing a `ui_name -> MidiControlConfig` lookup (same
  as today, just keyed differently) and by writing to a shared
  `Arc<Mutex<HashMap<String, f32>>>` that the hub also holds.
- `get` becomes: `midi_overrides.get(name).copied().unwrap_or_else(|| ...
  normal lookup ...)`
- `run_dependencies` loses the proxy substitution block entirely.
- `get_raw` loses the proxy-name detection block.
- `CommitMappings` inserts into `hub.midi_overrides_config` (ch/cc/range)
  and restarts the listener; no `__slider_proxy` keys created anywhere.
- Snapshots capture `ui_controls` values and `midi_overrides` values both
  under the plain name. The split-namespace problem goes away.
- Serialization saves the override table as plain name/value pairs; no
  proxy-name translation needed.
- The `__slider_proxy` constants, `proxy_name`, `unproxied_name`, and
  `is_proxy_name` helpers are deleted.

**Trade-off:** The MIDI listener callback needs a reference to the override
table. Today it writes to `MidiControls.State` which is already
`Arc<Mutex<_>>`, so the pattern is the same — just rename and retarget the
write destination.

---

### Option B: Shadow value on `UiControls`

Instead of a separate structure, `UiControls` gains an optional
`midi_shadow: HashMap<String, f32>`. When a mapping is active, incoming CC
values are written into `midi_shadow`. `UiControls.get(name)` returns the
shadow value if present, otherwise the stored value. `hub.get` stays
unaware of MIDI entirely.

**Upside:** The hub's `get` method is completely clean; mappings are an
internal concern of `UiControls`.

**Downside:** `UiControls` now has a MIDI dependency, which is architecturally
backwards (controls should not know about transport). It also makes it harder
to disable all MIDI overrides atomically (`midi_proxies_enabled` today is a
single flag on the hub; here it would need to propagate into `UiControls`).

---

### Option C: Replace `MidiControls` membership with a mapping registry

Keep `MidiControls` for controls that are YAML-declared (e.g. a CC that
maps directly to a parameter with no corresponding UI slider). Add a
separate `MidiMappingRegistry` that is only for the learned runtime
overrides. The registry stores `(name, ch, cc, min, max)` and drives a
single shared CC listener. On each CC message it writes into a
`HashMap<String, f32>` by plain name. The hub checks this table at the top
of `get`.

This is essentially Option A with a cleaner structural boundary: YAML-declared
`MidiControls` and runtime-learned mappings are no longer in the same
collection. Orphan cleanup, conflict detection, and serialization are all
scoped to the registry rather than scattered across the hub and app.rs.

---

## Recommendation

**Option A** is the most direct improvement with the smallest surface area
change. It removes the proxy naming convention entirely and collapses the
multi-layer coordination into a single `midi_overrides` lookup at the top
of `hub.get`. The commit/remove/restore lifecycle becomes simpler because
there is no longer a need to match phantom entries in `MidiControls` against
real UI controls.

**Option C** is worth pursuing if you want a cleaner architectural split
between YAML-declared MIDI and runtime-learned MIDI. It is a larger
refactor but would make both systems more understandable in isolation.

**Option B** is not recommended because it inverts the dependency direction.

---

## Migration path for Option A

1. Add `midi_overrides: HashMap<String, f32>` and
   `midi_override_configs: HashMap<String, (u8, u8, f32, f32)>` to
   `ControlHub`.
2. Update `MidiControls` listener to also write to an
   `Arc<Mutex<HashMap<String, f32>>>` passed in at `start` time (the
   override table).
3. Update `hub.get` to check `midi_overrides` first.
4. Remove the proxy substitution from `hub.get`, `run_dependencies`,
   and `get_raw`.
5. Update `CommitMappings` in `app.rs` to populate
   `hub.midi_override_configs` and pass the override table ref to the
   listener instead of inserting into `hub.midi_controls`.
6. Update `create_snapshot` and `recall_snapshot` to use the override table
   under plain names.
7. Update serialization to save `midi_overrides` as plain name/value pairs
   (same `BasicNameValueConfig` shape, different semantic label).
8. Delete `MapMode::proxy_name`, `unproxied_name`, `is_proxy_name`,
   and `PROXY_NAME_SUFFIX`.
9. Audit anywhere `hub.midi_controls.configs()` is iterated and filters
   for `is_proxy_name` — those sites either disappear or become queries
   against `midi_override_configs`.
