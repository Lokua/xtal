# ControlScript YAML Reference

# Table of Contents

- [General](#general)
- [UI](#ui)
  - [slider](#slider)
  - [checkbox](#checkbox)
  - [select](#select)
- [MIDI](#midi)
- [OSC](#osc)
- [Audio](#audio)
- [Animation](#animation)
  - [triangle](#triangle)
  - [automate](#automate)
    - [breakpoints](#breakpoints)
    - [kind](#kind)
      - [ramp](#ramp)
      - [step](#step)
      - [wave](#wave)
      - [random](#random)
      - [random_smooth](#random_smooth)
      - [end](#end)
- [Modulation](#modulation)
  - [mod](#mod)
- [Effects](#effects)
  - [hysteresis](#hysteresis)
  - [quantizer](#quantizer)
  - [ring_modulator](#ring_modulator)
  - [saturator](#saturator)
  - [slew_limiter](#slew_limiter)
  - [wave_folder](#wave_folder)
- [Parameter Modulation](#parameter-modulation)
- [Using `var`](#using-var)

# General

Lattice provides various interfaces for controlling parameters including
`Controls` for UI (sliders, checkboxes, and selects), ~~`MidiControls`~~ (TODO)
and `OscControls` for controlling parameters from an external source,
`AudioControls` for controlling parameters with audio or CV, and a comprehensive
`Animation` module that can tween or generate random values and ramp to/from
them at musical intervals. While these parameters are simple to setup, it's a
bit of pain to have to restart the rust sketch every time you want to change an
animation or control configuration. For this reason Lattice provides a
`ControlScript` mechanism that uses yaml for configuration and adds these
controls dynamically and _self-updates at runtime when the yaml file is
changed_, quite similar to live coding. You still have to take care to setup the
routings in your sketch (e.g. `let radius = model.controls.get("radius")`), but
once these routings are in place you are free to edit their ranges, values,
timing, etc. Here's an example that covers the overall capabilities:

```yaml
radius:
  type: slider
  range: [50, 300]
  default: 100

# ramp up and down over 16 beats = 4 bars
pos_x:
  type: triangle
  beats: 16
  range: [-500, 500]

# linearly from left to right then wiggle back and forth
# from right to left over 2 bars
pos_y:
  type: automate
  breakpoints:
    # linearly ramp to the next position
    - kind: ramp
      position: 0.0
      value: -500
    # linearly ramp to the next position with amplitude modulation applied
    - kind: wave
      # transition from here to next point will take 4 beats = 1 bar
      position: 4
      value: 500
      shape: sine
      # modulate up/down from base ramp over a period of 1 beat
      frequency: 1
      amplitude: 100
      constrain: clamp
    # End right where the loop restarts to ensure smooth transition
    - kind: end
      position: 8
      value: -500

# And now we can do some _crazy_ stuff!
imagination_amount:
  type: slider

imagination:
  type: automate
  breakpoints:
    - kind: ramp
      position: 0
      value: 0
    - kind: ramp
      position: 3
      # control the peak of this animation with a slider while its happening!
      value: $imagination_amount
    - kind: end
      position: 6
      value: 0

imagination_folder:
  type: effect
  kind: wave_folder
  gain: 2
  # Use the same control for two parameters because, why not?
  symmetry: $imagination_amount

# effects like above need to be connected with sources:
imagination_mod:
  type: mod
  source: imagination
  modulators:
    - imagination_folder
```

In your sketch, the above controls can be accessed as follows:

```rust
use crate::framework::prelude::*;

#[derive(SketchComponents)]
pub struct Model {
    controls: ControlScript<Timing>,
}

pub fn init_model(_app: &App, _wr: WindowRect) -> Model {
    let controls = ControlScript::from_path(
        to_absolute_path(file!(), "controls.yaml"),
        Timing::new(SKETCH_CONFIG.bpm),
    );

    Model { controls }
}

pub fn view(app: &App, m: &Model, frame: Frame) {
  let draw = app.draw();

  let radius = m.controls.get("radius");
  let pos_x = m.controls.get("pos_x");

  // do stuff
  // ...
}
```

The above example contains a bunch of YAML objects that we will refer to
henceforth as _mappings_. All mappings in general are 1:1 mappings to their Rust
structs. Some notes about mappings to keep in mind:

- All mappings require a `type` e.g. `slider`, `osc` or `automate`.
- Most, but not all, parameters can be omitted from a mapping except in cases
  where it makes no logical sense to omit them
- All mappings except UI controls support a `bypass` field. When this field is a
  number, that number will be used as a static value. This is useful for pausing
  animations or muting OSC streams. Any other value besides a number can be used
  to bypass the bypass.
- All controls support an optional `var` field. This is very useful for
  pre-loading shader uniforms before you know what the actual role or name of a
  control will be. See the [Using `var` section](#using-var).
- All numbers will be interpreted as `f32` so feel free to use integers where it
  makes sense

# UI

Interface to [Controls][crate::framework::controls]

All UI controls are added to the UI in the order they are declared.

## Slider

**Params**

- `type` - `slider`
- `range` - defaults to `[0.0, 1.0]`
- `default` - defaults to `0.5`
- `step` - defaults to `1.0`

**Example**

```yaml
slider_example:
  type: slider
  range: [0.0, 1.0]
  default: 0.5
  step: 1.0
```

## Checkbox

**Params**

- `type` - `checkbox`
- `default` - defaults to `false`

**Example**

```yaml
checkbox_example:
  type: checkbox
  default: false
```

## Select

**Params**

- `type` - `select`
- `range` - defaults to `[0.0, 1.0]`
- `default` - defaults to `0.5`
- `step` - defaults to `1.0`

**Example**

```yaml
slider_example:
  type: select
  default: foo
  options:
    - foo
    - bar
    - baz
```

# OSC

Listens for incoming floats on the port specified in the
[src/config.rs](src/config.rs)'s `OSC_PORT` constant (`2346`).

**Params**

Note that there is no `address` field; the address is taken from the mapping
name (`osc_example` in the example below - forward slash is handled internally).

- `type` - `osc`
- `range` - defaults to `[0.0, 1.0]`
- `default` - a default to use in the case an OSC message hasn't arrived at
  address since the program start. Defaults to `0.5`

**Example**

```yaml
osc_example:
  type: osc
  range: [0.0, 1.0]
  default: 0.5
```

# MIDI

Listens for incoming control change messages on the port specified in the
[src/config.rs](src/config.rs)'s `MIDI_INPUT_PORT` constant (currently hardcoded
to `IAC Driver Lattice In`). MIDI values are by default scaled to a `[0.0, 1.0]`
range.

**Params**

- `type` - `midi`
- `channel` - zero-indexed; defaults to `0`
- `cc` - zero-indexed; defaults to `0`
- `range` - defaults to `[0.0, 1.0]`
- `default` - a default to use in the case a CC message hasn't arrived since the
  program start. Defaults to `0.0`

**Example**

```yaml
midi_example:
  type: midi
  channel: 0
  cc: 0
  range: [0.0, 1.0]
  default: 0.0
```

# Audio

Listens for audio signals on the device specified in
[src/config.rs](src/config.rs)'s `MULTICHANNEL_AUDIO_DEVICE_NAME` constant and
transforms them into a stream suitable for parameter automation/modulation.

**Params**

- `type` - `audio`
- `channel` - the zero-indexed audio channel
- `slew` - Controls smoothing ([rise, fall]) when signal amplitude increases.
  - `0.0` = instant rise/fall (no smoothing)
  - `1.0` = very slow rise/fall (maximum smoothing)
  - defaults to `[0.0, 0.0]`
- `detect` - Linearly mix between 0=peak detection and 1=RMS peak detection.
  Peak is snappier, RMS is smoother but limits amplitude more. Defaults to
  `0.0`.
- `range` - defaults to `[0.0, 1.0]`

**Example**

```yaml
animation_example:
  type: audio
  channel: 0
  slew: [0.3, 0.9]
  detect: 0.0
  range: [0.0, 100.0]
```

# Animation

## Triangle

A "ping pong" animation that linearly ramps from min to max and back to min as
specified in the `range` param.

**Params**

- `type` - `triangle`
- `beats` - defaults to `1.0`
- `range` - defaults to `[0.0, 1.0]`
- `phase` - Phase offset expressed as percentage (0..1) of the above range. A
  phase offset of `0.5`, for example, will invert the triangle so that it ramps
  from max-min-max instead of min-max-min. Defaults to `0.0`

**Example**

```yaml
triangle_example:
  type: triangle
  # 16 beats = 4 bars
  beats: 16.0
  range: [0.0, 1.0]
  phase: 0.0
```

## Automate

Advanced DAW-style animation. This is the bread-and-butter of Lattice.

**Params**

- `type` - automate
- `mode` - `loop` or `once`. Defaults to `loop`
- `breakpoints` - a list of breakpoint kinds including `step`, `ramp`, `wave`,
  `random`, and `random_smooth`

### Breakpoints

Each breakpoint shares the following _required_ fields:

- `kind` - one of `step`, `ramp`, `wave`, `random`, `random_smooth`, or `end`.
  See the [`kind`](#kind) section below.
- `position` - expressed in beats. The first breakpoint must start at position
  `0.0` or the program will throw
- `value` - the value this breakpoint will (usually) be when the timing is
  exactly at `position`

### Kind

#### `ramp`

Ramps from `value` at `position` to the next point's value with optional easing.

**Additional Params**

- `easing` - a snake cased version of any of the easings defined in
  [src/framework/easings.rs](src/framework/easings.rs). Defaults to `linear`

#### `step`

Holds `value` from this point's `position` until the next point.

#### `wave`

Like `ramp`, but with a secondary amplitude modulation applied on top of it

**Additional Params**

- `frequency` - the rate of amplitude modulation, expressed in beats.
- `amplitude`- how much above and below the base ramp to add/subtract. Defaults
  to `0.25`
- `width` - For square controls duty cycle; for sine and triangle skews the
  wave. With triangle shape, `0.0` produces a downwards saw, `1.0` an upwards
  one, and `0.5` will produce the regular triangle. Sine is similarly
  transformed into a saw-like asymmetric shape.
- `shape` - one of `sine`, `triangle`, or `square`. Defaults to `sine`
- `easing` - a snake cased version of any of the easings defined in
  [src/framework/easings.rs](src/framework/easings.rs). Defaults to `linear`.
  The combination of `wave` and `easing` (especially the crazier ones like
  `ease_in_out_bounce`) and `clamp: fold` can produce some _very_ complex
  waveforms!
- `constrain` - one of `none`, `clamp`, or `fold`. Defaults to `none`.

#### `random`

Generates a random number somewhere above or below the set `value` by
`amplitude` amount.

**Additional Params**

- `amplitude` - how much +- the random number generator will deviate from
  `value` when choosing a number

### `random_smooth`

Like the [`ramp`](#ramp) type only uses perlin noise to deviate from the base
ramp.

**Additional Params**

- `frequency` - the rate of amplitude modulation, expressed in beats.
- `amplitude`- how much above and below the base ramp to add/subtract. Defaults
  to `0.25`

### `end`

A special breakpoint added for semantic clarity. It is identical to the step
kind. It represents what the overall sequence will end on. In the case of
mode=loop, this segment will never be entered; for mode=once the value that will
be held indefinitely.

**Example**

```yaml
automate_example:
  type: automate
  mode: loop
  breakpoints:
    - kind: step
      position: 0.0
      value: 0.0

    - kind: ramp
      position: 1.0
      value: 0.0
      easing: linear

    - kind: wave
      position: 2.0
      value: 1.0
      frequency: 0.25
      amplitude: 0.25
      width: 0.5
      shape: sine
      easing: linear
      constrain: none

    - kind: random
      position: 4.0
      value: 0.5
      amplitude: 0.5

    - kind: random_smooth
      position: 3.0
      value: 0.0
      frequency: 0.25
      amplitude: 0.25

    - kind: end
      position: 5.0
      value: 1.0
```

# Modulation

## Mod

Takes any declared control as a source and modifies its output using one or more
modulators. A modulator can be an [effect](#effects), [animation](#animation),
or the output of a [slider](#slider) (TODO: validate if Osc/Midi/Audio can be
modulators).

**Params**

- `type` - `mod`
- `source` - name of the control to modulate
- `modulators` - list of effect names to apply to the source

**Example**

```yaml
mod_example:
  type: mod
  source: automate_example
  modulators:
    - wave_folder
    # sliders act as multipliers
    - some_slider
```

# Effects

Effects can only be used as modulators within a `mod` configuration and cannot
be used as sources. A single effect can be used more than once, for example you
might have several animations that use stepped randomness and may want to smooth
them all out with a single slew_limiter.

## Hysteresis

Implements a Schmitt trigger with configurable thresholds that outputs:

- `output_high` when input rises above `upper_threshold`
- `output_low` when input falls below `lower_threshold`
- previous output when input is between thresholds
- input value when between thresholds and `pass_through` is true

**Params**

- `type` - `effect`
- `kind` - `hysteresis`
- `lower_threshold` - defaults to `0.3`
- `upper_threshold` - defaults to `0.7`
- `output_low` - The value to output when input is below the lower threshold.
  Defaults to `0.0`
- `output_high` - The value to output when input is above the upper threshold.
  Defaults to `1.0`
- `pass_through` - When true, allows values that are between the upper and lower
  thresholds to pass through. When false, binary hysteresis is applied. defaults
  to `false`

**Example**

```yaml
hysteresis_example:
  type: effect
  kind: hysteresis
  lower_threshold: 0.3
  upper_threshold: 0.7
  output_low: 0.0
  output_high: 1.0
  pass_through: false
```

## Quantizer

Discretizes continuous input values into fixed steps, creating stair-case
transitions.

For example, with a step size of 0.25 in range (0.0, 1.0):

- Input 0.12 -> Output 0.0
- Input 0.26 -> Output 0.25
- Input 0.51 -> Output 0.50

**Params**

- `type` - `effect`
- `kind` - `quantizer`
- `step` - The size of each discrete step. Defaults to `0.25`
- `range` - defaults to `[0.0, 1.0]`

**Example**

```yaml
quantizer_example:
  type: effect
  kind: quantizer
  step: 0.25
  range: [0.0, 1.0]
```

## Ring Modulator

Implements ring modulation by combining a carrier and modulator signal. Note
that there is no actual "carrier" parameter because the modulator signal will be
applied to the `source` field defined in a `mod` config.

**Params**

- `type` - `effect`
- `kind` - `ring_modulator`
- `mix` - Controls the blend between carrier and modulated signal
  - `0.0`: outputs carrier signal
  - `0.5`: outputs true ring modulation (carrier \* modulator)
  - `1.0`: outputs modulator signal
  - (defaults to `0.0`)
- `modulator` - name of the control to use as modulator

**Example**

```yaml
ring_modulator_example:
  type: effect
  kind: ring_modulator
  mix: 0.0
  modulator: some_other_control

rm_mod_routing:
  type: mod
  source: automate_example
  modulators:
    - ring_modulator_example
```

## Saturator

Applies smooth saturation to a signal, creating a soft roll-off as values
approach the range boundaries. Higher drive values create more aggressive
saturation effects.

**Params**

- `type` - `effect`
- `kind` - `saturator`
- `drive` - defaults to `1.0`
- `range` - defaults to `[0.0, 1.0]`

**Example**

```yaml
saturator_example:
  type: effect
  kind: saturator
  drive: 1.0
  range: [0.0, 1.0]
```

## Slew Limiter

Limits the rate of change (slew rate) of a signal

**Params**

- `type` - `effect`
- `kind` - `slew_limiter`
- `rise` - Controls smoothing when signal amplitude increases.
  - `0.0` = instant attack (no smoothing)
  - `1.0` = very slow attack (maximum smoothing)
  - (defaults to `0.0`)
- `fall` - Controls smoothing when signal amplitude decreases.
  - `0.0` = instant decay (no smoothing)
  - `1.0` = very slow decay (maximum smoothing)
  - (defaults to `0.0`)

**Example**

```yaml
slew_limiter_example:
  type: effect
  kind: slew_limiter
  rise: 0.0
  fall: 0.0
```

## Wave Folder

**Params**

- `type` - `effect`
- `kind` - `wave_folder`
- `gain` - Suggested range: 1.0 to 10.0
  - <1.0: Bypassed
  - 1.0: unity gain
  - 2.0-4.0: typical folding range
  - 4.0-10.0: extreme folding
  - (defaults to `1.0`)
- `iterations` - Suggested range: 1 to 8
  - 1-2: subtle harmonics
  - 3-4: moderate complexity
  - 5+: extreme/digital sound
  - (defaults to `1`)
- `symmetry` - changes the relative intensity of folding above vs below the
  center point by scaling the positive and negative portions differently.
  Suggested range: 0.5 to 2.0
  - 1.0: perfectly symmetric
  - <1.0: negative side folds less
  - \>1.0: negative side folds more
  - (defaults to `1.0`)
- `bias` - Shifts the center point of folding, effectively moving the "zero
  crossing" point. Suggested range: -1.0 to 1.0
  - 0.0: no DC offset
  - ±0.1-0.3: subtle asymmetry
  - ±0.5-1.0: extreme asymmetry
  - (defaults to `0.0`)
- `shape` - Suggested range: [`-2.0`, `2.0`] (values below -2.0 are hard capped)
  - 0.0: linear folding
  - < 0.0: softer folding curves
  - -1.0: perfectly sine-shaped folds
  - < -2.0: introduces intermediary folds but slight loss in overall amplitude
  - \> 0.0: sharper folding edges, power function with exponent (1.0 + shape)
  - 1.0: quadratic folding (power of 2.0)
  - 2.0: cubic folding (power of 3.0)
  - (defaults to `1.0`)
- `range` - defaults to `[0.0, 1.0]`

**Example**

```yaml
wave_folder_example:
  type: effect
  kind: wave_folder
  gain: 1.0
  iterations: 1
  symmetry: 1.0
  bias: 0.0
  shape: 1.0
  range: [0.0, 1.0]
```

# Parameter Modulation

In addition to use of `effect` and `mod` types to modulate the output of
controls, Lattice supports _parameter modulation_. It's easiest to explain with
an example:

```yaml
size_amount:
  type: slider

size:
  type: automate
  breakpoints:
    - kind: ramp
      position: 0
      value: 0
    - kind: ramp
      position: 4
      value: $size_amount # <-- HERE
    - kind: end
      position: 8
      value: 0
```

In the above example we use a UI slider to control the maximum amount of a basic
"ramp up and back down" animation. Some rules about parameter modulation:

- Parameter modulations are denoted by the prefix `$` and the name of another
  mapping.
- Any animation or UI control that produces a float can be the source of
  parameter modulation.
- `effect` and `mod` types _cannot_ be the sources of parameter modulations
- Only named fields that are of type `f32` can be modulated. For example any
  `value` or parameter such was the Wave Folder's `symmetry` param, but not any
  mapping's `range` since that's a list.

# Using `var`

```yaml
radius:
  var: a1
  type: slider
```

In your sketch this control will be accessed via `m.controls.get("a1")`. This is
especially useful for sketches that primarily rely on shaders - since like
control scripts - shaders in Lattice support live reloading. Often creative
coding is an experimental process and you may not know what controls you'll need
up front and it's a huge pain to have to restart the Rust program every time you
want to change a variable name or add a new one. To work around this, you can
setup "banks":

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct ShaderParams {
    // Add a bunch of 4 member arrays that will be unpacked in your shader
    a: [f32; 4],
    b: [f32; 4],
    c: [f32; 4],
    d: [f32; 4],
    e: [f32; 4],
    f: [f32; 4],
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let controls = ControlScript::from_path(
        to_absolute_path(file!(), "example.yaml"),
        Timing::new(SKETCH_CONFIG.bpm),
    );

    // No point in initializing this to anything other than zero
    // as they will just get overwritten in the update function
    let params = ShaderParams {
        resolution: [0.0; 4],
        a: [0.0; 4],
        b: [0.0; 4],
        c: [0.0; 4],
        d: [0.0; 4],
        e: [0.0; 4],
        f: [0.0; 4],
    };

    let gpu = gpu::GpuState::new_fullscreen(
        app,
        wr.resolution_u32(),
        to_absolute_path(file!(), "example.wgsl"),
        &params,
        true,
    );

    Model { controls, wr, gpu }
}

pub fn update(app: &App, m: &mut Model, _update: Update) {
    // Update ensures any file changes have propagated
    m.controls.update();

    let params = ShaderParams {
        a: [
            // Allows us to use `var: a1` in our control_script
            m.controls.get("a1"),
            m.controls.get("a2"),
            m.controls.get("a3"),
            m.controls.get("a4"),
        ],
        b: [
            m.controls.get("b1"),
            m.controls.get("b2"),
            m.controls.get("b3"),
            m.controls.get("b4"),
        ],
        c: [
            m.controls.get("c1"),
            m.controls.get("c2"),
            m.controls.get("c3"),
            m.controls.get("c4"),
        ],
        // ...
    };

    m.gpu.update_params(app, m.wr.resolution_u32(), &params);
}
```

Then in your shader:

```wgsl
struct Params {
    a: vec4f,
    b: vec4f,
    // ...

@fragment
fn fs_main(vout: VertexOutput) -> @location(0) vec4f {
    let radius = params.a.x;
    let pos_x = params.a.y;
    // ...
```

The above is admittedly a decent amount of boilerplate, but with this setup you
are now free to live code in your script and shaders for hours uninterrupted
without having to stop, recompile, wait... it's worth it.
