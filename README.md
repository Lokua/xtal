# lattice

A hobbyist project exploring generative art while learning Rust and
[nannou][nannou-link].

Stuff like this:

<img src="https://s3.us-east-1.amazonaws.com/lokua.net.lattice/images/displacement_2-627iz.png" alt="displacement_2-627iz">
<img src="https://s3.us-east-1.amazonaws.com/lokua.net.lattice/images/displacement_2-tm8s9.png" alt="displacement_2-tm8s9">
<img src="https://s3.us-east-1.amazonaws.com/lokua.net.lattice/images/displacement_2-vnh7y.png" alt="displacement_2-vnh7y.png">

You can see more here on github by looking at the auto generated
[markdown index](index.md).

## Overview

This project aims to port my [p5.js project][p5-link] to Rust for improved
performance. It provides a personal framework around nannou that simplifies
creating multiple sketches by handling common concerns like window creation, GUI
controls, and declarative frame-based animation.

## Features

-   Export images and capture mp4 videos with the press of a button
-   Declarative animation interface with times specified in musical beats, e.g.
    `3.25` represents a duration of 3 beats and 1 16th note.
-   Sync animations to BPM and frame count, MIDI clock, MIDI Time Code, or OSC
-   Automate parameters with MIDI CC, OSC, CV, or audio with peak, rms, and
    multiband mechanisms all available through a dead simple API
-   Write animations in code or configure your sketch to use an external yaml
    file that can be hot-reloaded (similar to live coding - see
    [Control Scripting](#control-scripting))
-   Declarative per-sketch UI control definitions with framework agnostic design
-   Automatic store/recall of GUI control/parameters
-   Hot reloadable WGSL shaders.

## Requirements

This project has been developed on MacOS, though I'm sure most of it would work
on other platforms. This project requires or optionally needs:

-   Rust
-   Git LFS for screenshot storage (perhaps this is optional? I'm not too
    familiar with Git LFS but I'm using it for this so you might want to too)
-   (optional) [just][just-link] for running commands
-   (optional) ffmpeg available on your path for video exports

## Usage

### Running a sketch

```sh
cargo run --release -- <sketch>
# or alternatively
just start <sketch>
```

Where `sketch` is a sketch is a file in the src/sketches folder (without the
extension) and registered in [src/sketches/mod.rs][module-link] as well as
[src/main.rs][main-link].

Optionally you can pass a `timing` argument after the required `sketch` argument
to specify what kind of timing system will be used to run animations on sketches
that support it (this is a new feature so not all sketches are using this
run-time `Timing` mechansim yet). Available options include:

#### `frame`

Uses Lattice's internal frame system. This is the default and doesn't require
any external devices to run.

#### `osc`

Requires `assets/L.OscTransport.amxd` to be running in Ableton Live. This
provides the most reliable syncing mechanism as Ableton does not properly send
MIDI SPP messages and doesn't support MTC.

#### `midi`

Uses MIDI clock and MIDI Song Position Pointers (SPP) to stay in sync (e.g. when
a MIDI source loops or you jump to somewhere else in a timeline, you animations
will jump or loop accordingly). Bitwig properly sends SPP; Ableton does not.

#### `hybrid`

Uses a combination of MIDI clock (for precision) and MIDI Time Code (MTC) to
stay in sync. This is useful for DAWs that don't support sending SPP but do
support MTC. Ableton, for example, does not support MTC but you can work around
that with https://support.showsync.com/sync-tools/livemtc/introduction

### Creating a new sketch:

1. Copy the [template sketch][template-link] into a new file in sketches folder.
2. Rename at a minimum the `SKETCH_CONFIG.name` field at the top of the file:
    ```rust
    pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
       name: "template", // <-- RENAME THIS!
    ```
3. Add that filename to the [src/sketches/mod.rs][module-link]
4. Add a match case for the sketch in [src/main.rs][main-link]:
    ```rust
    "my_awesome_sketch" => run_sketch!(my_awesome_sketch),
    ```
5. Run that sketch via command line by `cargo run --release <name>` or
   `just start <name>` where `name` is what you put in your file's
   `SKETCH_CONFIG.name` field.

### Audio

Lattice is hardcoded to read audio from the first input (input 1, or index 0, or
left channel) on a device named "Lattice". I am currently doing this via
Aggregate Device on my Mac using [Blackhole 2ch][blackhole] to capture output
from DAW. Here are some screenshots of the setup:

**Aggregate Device Setup**
![Mac Aggregate Device Setup](assets/aggregate-device-setup.png)

**Routing Audio to Blackhole 2ch `Out(3/4):In(1/2)`**

> Note that Blackhole automatically routes whatever its output channels are to
> its own input, so sending audio out to Blackhole 3/4 will automatically appear
> on inputs 1/2 in this setup; you don't even need to configure the inputs in
> Ableton at all for this to work (just as long as you have the output config
> set to "Lattice" and enable the appropriate ouputs in the output config under
> Live's audio preferences)

![Ableton Live - Blackhole Track Routing](assets/live-blackhole-track-routing.png)

### MIDI

Lattice is hardcoded to accept MIDI on a virtual MIDI device that must be named
`IAC Driver Lattice In`.

### MIDI Loopback

To control synth parameters in Ableton and Lattice parameters simultaneously,
you need to enable MIDI loopback by sending MIDI to `Lattice In` and also route
`Lattice In` back in to Live to control parameters. Here's the routing:

![Live MIDI Preferences](assets/live-midi-prefs.png)

To use Ableton automation lanes to control Lattice params, follow these steps:

1. Create a MIDI track and clip and add CC automation to it.
2. In the tracks **MIDI To** router, select `IAC Driver Lattice In` and `Ch. 1`

Those steps are all you need to send MIDI to Lattice to control parameters. As
for controlling a live parameter with that same CC, follow these steps:

1. Play your clip containing the CC data
2. Stop the transport (this is important!)
3. Enter MIDI Mapping mode.
4. Locate the parameter to you want to map and select it (make sure it's the
   last thing you've clicked)
5. Press the Space bar to start the transport. This should do it!

See the [midi_test.rs sketch][midi-sketch-link] for an example of how to map a
control to something.

> Note: the above instructions are for working without a MIDI controller. When
> working with a MIDI controller you can just map the MIDI control to an Ableton
> device knob that can send CC out to Lattice and also map the controller to an
> Ableton parameter. In this case _you do not_ want Lattice enabled in Ableton's
> MIDI Input ports at all as that just complicates things.

#### Sync Recording

1. In Ableton > Preferences > Record, make sure **Start Transport With Record**
   is set to **Off**
2. Hit **Q Rec** in Lattice.
3. Arm tracks in Ableton, arm the transport (Record button)
4. Now, pressing play in Ableton will also initiate recording in Lattice,
   likewise pressing Stop in Ableton will stop recording in Lattice.

### Control Scripting

Lattice provides various interfaces for controlling parameters including
`Controls` for UI sliders, checkboxes, and selects (dropdowns), `MidiControls`
and `OscControls` for controlling parameters from an external source,
`CvControls` and `Audio` for controlling parameters with audio, and a
comprehensive `Animation` module that can tween or generate random values and
ramp to/from them at musical intervals. While these parameters are simple to
setup, it's a bit of pain to have to restart the rust sketch every time you want
to change an animation or control range. For this reason Lattice provides a
`ControlScript` mechanism that uses yaml for configuration and adds these
controls dynamically and self-updates at runtime when the yaml file is changed.
You still have to take care to setup the routings in your sketch (e.g.
`let radius = model.control_script.get("radius")`), but once these routings are
in place you are free to edit their ranges, values, timing, etc. See [Control
Script Test][control-script-test-link] for a working example. See below for
scripting documentation:

> Note: you cannot use an instance of `Controls` and `ControlScript` in a sketch
> at the same time; you must choose one or the other and it must be attached to
> the sketch's `Model` as `model.controls`.

```yaml
# Any yaml field that doesn't match an object with a known type
# will be ignored. Use them however you want; here I just use it house
# aliases/anchors (variables) and prefix with underscore to make it
# explicit
_vars:
    example_var: &example_var 33.0

# Available in sketch as `m.controls.get("radius")`
radius:
    type: slider
    # Optional, defaults to [0.0, 1.0]
    range: [0.0, 500.0]
    # Optional, defaults to 0.5
    # (here we are referencing the example_var declared in the `info` section)
    default: *example_var
    # Optional, defaults to 0.0001
    step: 1.0

some_boolean:
    type: checkbox
    default: false

select_example:
    type: select
    default: foo
    options:
        - foo
        - bar
        - baz

# Available in the sketch as `m.controls.get("position_x")`. The OSC
# address from your sender must be `/position_x` and is currently hardcoded
# to PORT 2346
position_x:
    # The OSC address with forward slash is automatically
    # derived from the key.
    type: osc
    # Optional, defaults to [0.0, 1.0]
    range: [0.0, 100.0]
    # Optional, defaults to 0.5
    default: 50.0

hue:
    # Interface to the `Animation#lerp` method that differs from the normal code
    # signature in that times are expressed in "<bars>.<beats>.<16ths>" like a typical
    # DAW would use and are absolute with respect to the timeline depending on what
    # `TimingSource` is provided to the `ControlScript` constructor.
    type: lerp_abs
    # Optional, defaults to 0.0
    delay: 0.0
    keyframes:
        # beats, bars, and 16ths are zero indexed!
        #...start at 0 then...
        - ["0.0.0", 0.0]
        # ramp to 1 over the duration from 0 to the 2nd beat, then...
        - ["0.1.0", 1.0]
        # ramp back down to 0.0 from the 2nd to the start of the 3rd beat
        - ["0.2.0", 0.0]
        # ^ the above creates a perfect 2 beat loop and will continue looping.
        # `bypass`, if omitted or is not a number will simply be ignored,
        # however if it is a number will be used instead of the animation.
        # This is great for testing, debugging, of even "live coding" to mute
        # animations. All animation definitions support bypass;
        bypass: _

saturation:
    # Another interface to the same `Animation#lerp` method as above but uses the
    # exact same signature as the code instance for keyframes which is [beats, value].
    # This example and last are 100% equivalent but read quite differently.
    # While the `abs` version can be read as "arrive at this value at this time",
    # the `rel` version should be read as "ramp from this value to the next over this time"
    type: lerp_rel
    # Optional, defaults to 0.0
    delay: 0.0
    bypass: _
    keyframes:
        # Ramp from 0.0 to the next keyframe value (1.0) over 1 beat
        - [1.0, 0.0]
        # etc...
        - [1.0, 1.0]
        # By convention I always set the duration of the last keyframe to 0.0
        # Since it represents the last arrival value and doesn't really have a
        # duration.
        - [0.0, 0.0]

lightness:
    # A 1:1 interface to the `Animation#r_ramp` method.
    type: r_ramp_rel
    # Optional, defaults to "linear". Easing options include:
    # linear, ease_in, ease_out, ease_in_out, cubic_ease_in, cubic_ease_out,
    # cubic_ease_in_out, sine_ease_in, sine_ease_out, sine_ease_in_out, logarithmic
    ramp: linear
    # Optional, defaults to 0.25 (1/16th note)
    ramp_time: 0.5
    bypass: _
    keyframes:
        # Every beat, pick a random value between 0.0 and 1.0, and ramp to it
        # over `ramp_time` beats. So for example let's say the random number generated
        # for the 1st cycle was 0.2 and the 2nd cycle was 0.7: When the animation is
        # started it will stay at 0.2 for the first 1/2 of a beat, then over the next 1/2
        # beat it will ramp to 0.7. If `ramp_time` was 0.25, it would stay at 0.2 for the
        # duration of a dotted eigth note, then ramp to 0.7 over a single 16th.
        # Note that only the first keyframe is held; all subsequent cycles will always be
        # ramped to (ramp happens at the end of a cycle and happens within that cycle's
        # duration; this is why the first cycle starts static until its ramp phase -
        # this is because there was no previous cycle that could ramp to it)
        - [1.0, [0.0, 1.0]]

foo:
    # A "ping" animation that linearly ramps from min to max and back to min
    # as specified in `range` option
    type: triangle
    beats: 2.0
    # Optional, defaults to [0.0, 1.0]
    range: [-1.0, 1.0]
    # Phase offset expressed as percentage (0..1) of the above range.
    # Optional defaults to 0
    phase: 0.25
    bypass: _
```

## Resources

-   https://inconvergent.net/generative/
-   http://www.complexification.net/
-   https://n-e-r-v-o-u-s.com/projects/albums/floraform-system/
-   https://www.andylomas.com/cellularFormImages.html
-   http://www.complexification.net/gallery/machines/sandstroke/
-   https://thebookofshaders.com/
-   https://github.com/jasonwebb/2d-space-colonization-experiments
-   https://paulbourke.net/geometry/

[nannou-link]: https://github.com/nannou-org/nannou
[p5-link]: https://github.com/Lokua/p5/tree/main
[just-link]: https://github.com/casey/just
[blackhole]: https://existential.audio/blackhole/
[template-link]: src/sketches/templates/template.rs
[midi-sketch-link]: src/sketches/midi_test.rs
[module-link]: src/sketches/mod.rs
[main-link]: src/main.rs
[control-script-test-link]: src/sketches/scratch/control_script_test.rs
