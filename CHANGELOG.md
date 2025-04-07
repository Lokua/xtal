# Changelog

All notable changes to this project will be documented in this file.

The format is loosely based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). The project will
eventually adhere to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
when it reaches v1, but until then consider all changes as possibly breaking.

## [0.3.0] - 2025-04-06

### Added

- Documentation for the `disabled` micro-DSL in
  [Control Script Reference](docs/control_script_reference.md)
- Help text for all UI elements

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
  immediate access to help information that native titles provide.
- Removed the `useIcons` setting and text-only buttons in favor of help-view and
  titles so we don't sacrifice a11y due to icon-only buttons

### Fixed

- `overflow: hidden` to UI body to prevent that silly bounce effect

## [0.2.0] - 2025-04-05

### Added

- New **Randomize** feature via button `[?]` or `Cmd+R` that utilizes a
  temporary snapshot to linearly interpolate from the current state of all
  controls to a randomized state with respect to a controls given min, max, and
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
