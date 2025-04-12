# Changelog

All notable changes to this project will be documented in this file.

The format is loosely based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). The project will
eventually adhere to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
when it reaches v1, but until then consider all changes as possibly breaking.

## [0.8.0] - 2025-04-11

### Added

- `ControlHub::select` which makes choosing between a manual slider or animation
  for a single destination slightly less noisy

### Changed

- **Snapshots** UI now appears as a header on top of the controls area instead
  of covering it so we can see the final "snap" of values after the transition.
  I'm still not 100% happy with the design but it's good enough for now and a
  definite improvement over the previous iteration.
- Keyboard shortcuts: **Capture Image** is now `I` instead of `S`; `S` now
  toggles the **Snapshots** panel.
- Removed frontend alerts. From now on the concept of "alert" pertains only to
  messages from the backend.
- Prevent allowing a user map the same MIDI channel/cc pair to multiple
  destinations which wasn't and likely never will be supported since you can
  just a single slider for as many things as you want in code

### Fixed

- MIDI mapping a slider that only functioned as a modulation source did not
  result in the destination parameter being modulated because the proxy was not
  known as a dependency. See
  [Issue #22](https://github.com/Lokua/lattice/issues/22)
- Renaming a control in a Hub would cause orphaned proxies. See
  [Issue #23](https://github.com/Lokua/lattice/issues/23)

## [0.7.0] - 2025-04-10

### Added

- Zoom! Re-added the **Settings > Appearance** section and a new **Font Size**
  option that will scale most UI elements from default to large and largest for
  a11y's sake (unfortunately webviews or at least Apple's does not support +/-
  zoom like most browsers do)
- In addition to already being able to click a control's label to randomize it,
  you can now `<PlatformModifier> + Click` to revert it back to its last saved
  value.
- Exclusions column can now be toggled via `E` key

### Changed

- Converted all sizing units in CSS to rem/em to support the new "zoom" feature
- NumberBox in Controls now properly blurs when pressing Enter and supports
  `<PlatformModifier> + A` to select all text
- Keyboard shortcuts are not platform aware, using `meta` aka `logo` on Mac and
  `ctrl` on Linux/Windows (untested)
- Singe Parameter Randomization is no longer connected to the sketch-level
  Exclusions - in other words you can still click to randomize a single
  parameter even if it's in the exclusions list.
- Remove dark-light library in favor of Tao's automatic theme awareness which is
  pretty damn awesome

### Fixed

- Previously added a **Disable Mappings** feature but forgot to add the
  corresponding logic in Controls.tsx to actually enable slider control when
  needed - works as expected now
- Mo' Proxies Mo' Problems. Similar to the issue in [0.4.0](#040---2025-04-07),
  we were prematurely unwrapping the result of a `UiControls::slider_range` call
  for a slider that didn't exist (again, for a leftover MIDI proxy as the result
  of renaming/debugging). TODO: cleanup data before loading/saving to disk to
  avoid this entirely
- Converting backend string value representations to numbers on the frontend now
  uses `Number` and an `isFinite` check to work around limitations in
  `parseFloat` which cannot parse numbers that have higher precision than f32
  can hold, which resulted in some sliders having an `undefined` value.

## [0.6.0] - 2025-04-08

### Added

- UI for Snapshots
- **Disable Mappings** and **Delete Mappings** buttons to the MIDI Mapping
  panel. Disabled state is saved along with Global Settings.

### Changed

- Removed `docs/parameter_handling.md` due to being outdated and not very
  helpful

### Fixed

- Crash when switching to a sketch with a WGPU Depth Texture and window size
  that differed from the previous sketch. We were correctly updating the winit
  window in the app and texture size in `GpuState`, but Nannou's frame seems to
  be a single frame behind window updates (presumably because it doesn't get
  updated until the next update cycle as opposed to "this" update cycle) which
  led to the depth texture size mismatch with the Nannou frame. When this is the
  case we log a warning and exit the `GpuState::render` early for that frame.

## [0.5.0] - 2025-04-07

### Added

- Reload button next to Sketch selector that reloads the current sketch
  instantly

## [0.4.0] - 2025-04-07

### Added

- Ability to exclude any specific UI control from Randomization + persistence of
  exclusions along with a sketch's program state
- Tiny icons next to a control's label to indicate if it is excluded or MIDI
  mapped
- Strike-through on labels that are bypassed in a Control Script
- Ability to randomize a _single_ control by clicking its label. Hovering over a
  randomizable control (those that aren't bypassed or disabled) will show a
  highlighted background and prepend `<-?` characters to hopefully make it
  obvious what this does?

### Changed

- Removed the **Include Checkboxes** and **Include Selects** options from
  settings since the new Exclusions feature offers more flexibility
- Completely overhauled the `web_view::Control` structure to avoid the
  awkwardness of untagged enums due to
  [bincode](https://github.com/bincode-org/bincode). The frontend control code
  is much, much cleaner now.

### Fixed

- Bug where loading a sketch that has saved program state with MIDI proxies and
  no corresponding Slider range would cause a panic. This could only happen when
  a user saved state, then changed the slider name in their source code, then
  tried to reload the program. `UiControls::slider_range` now returns an option
  instead of panicking and `SaveableProgramState::setup_midi_mappings` will now
  bypass those invalid mappings and log an error to the console with remediation
  steps.

## [0.3.0] - 2025-04-06

### Added

- Documentation for the `disabled` micro-DSL in
  [Control Script Reference](docs/control_script_reference.md)
- Help text for all UI elements
- [Fira Code](https://github.com/tonsky/FiraCode) as our font

### Changed

- `ControlHub` now ensures that bypassed values are returned before transition
  values. This is especially crucial for sketches where a slider when at its max
  value might slaughter performance and you'd like to avoid hitting that max via
  the Randomization feature; now you can easily bypass controls to exclude them
  from Randomization. The side effect here is that `bypassed` will now override
  any snapshot values...need to think through this, but it's the lesser problem
  and perhaps not a problem at all.
- Custom `Checkbox.tsx` implementation using checked icon instead of font
  character to avoid off-by-1 pixel aliasing or whatever it's called
- Split console into Alert and Help views. When hovering over elements, the Help
  view will display help information for that element. This provides more
  immediate access to help information than native titles provide.
- Removed the `useIcons` setting and text-only buttons in favor of help-view and
  titles so we don't sacrifice a11y due to icon-only buttons

### Fixed

- `overflow: hidden` to UI body to prevent that silly bounce effect

## [0.2.0] - 2025-04-05

### Added

- New **Randomize** feature via button `[?]` or `Cmd+R` that utilizes a
  temporary snapshot to linearly interpolate from the current state of all
  controls to a randomized state with respect to a control's given min, max, and
  step.
- Settings options for Randomization to in/exclude checkboxes and selects
- New **Appearance** section in the settings tab with a **Use icons** toggle to
  switch between icon buttons and text buttons. Will likely do away with this
  options once we settle on which is a better user experience.

### Changed

- The order of operations in `ControlHub::get` has changed to support the
  Randomize feature; previously we 1. looked up alias then 2. checked
  transition; now we 1. look up alias then 2. look up MIDI proxy name and _then_
  check the transition. This may have unintended side effects and I do not
  remember why we didn't support proxies in transitions to begin with.
- Send MIDI out for all CCs when a snapshot, random or otherwise, ends
- Send MIDI out for all CCs when a sketch is switched - no more having to
  manually click `Send`
- Refactored `param_mod` effect trait implementations to use a declarative macro
  to reduce a significant amount of boilerplate

### Fixes

- `transition_time` is now properly retained when switching sketches

## [0.1.0] - 2025-04-05

Just now adding a changelog. For changes between November of 2024 and now you
must consult the commit history, but overall at this point we have:

- Built an entire framework around Nannou
- Moved from EGUI for UI to using React/Typescript in a WebView rendered with
  Tao and Wry which has provided amazing UI flexibility at the cost of added
  complexity
- Per sketch MIDI, OSC, UI, and Audio Controls via the ControlHub struct with
  disk persistence
- A live "scripting" mechanism for all controls and animations with
  hot-reloading via ControlHub featuring parameter modulation (use sliders or
  animations to control the parameters of other animations and effects - crazy)
- A comprehensive musical-timing based animation suite featuring DAW-style
  automation breakpoints with complex curves like ramps, random, random smooth,
  and various synth-style waveforms that can amplitude modulated
- Shader support with various templates to get up and running quickly
- A somewhat advanced yet undocumented `uniforms` proc-macro to ease the
  boilerplate of creating wgsl vec4 uniform "banks" and having to manually
  initialize them and pass them to the shader
- Ability to MIDI map controllers to sliders in the UI aka "MIDI learn"
- Capture up to 10 snapshots of all parameters on a per-sketch basis; snapshots
  are linearly interpolated from/to at an adjustable beat-based duration
- Global persistence of MIDI, OSC, and audio ports selectable in the UI's
  Settings View.
- Ability to randomize all sketch parameters via Cmd+R (POC - need to integrate
  this with the snapshot system)

## [0.0.0] - 2024-11-30

Project began as a basic Nannou app.
https://github.com/Lokua/lattice/tree/bb18c8214a7f693c10f84f52562eb7e55c0f6a50
