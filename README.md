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

- Export images and capture mp4 videos with the press of a button
- Declarative animation interface with times specified in musical beats, e.g.
  `3.25` represents a duration of 3 beats a 16th note.
- Sync animations to MIDI using MIDI Clock and Song Position Pointers (to track
  when a DAW is looping so you can work on or see the animation loop in-time) or
  a hybrid MIDI Time Code / MIDI Clock system that uses the higher resolution
  MIDI clock for sync but relies on MTC to detect when a source has jumped or is
  looping (this is for sources that don't properly send the SPP message). There
  is even included a special L.OscTransport Max4Live devices that will send
  transport over OSC which is much more reliable than MIDI clocking.
- Write animations in code or configure your sketch to use an external TOML file
  so you can work on animations without restarting the rust program. Or...
- Automate parameters with MIDI CC, OSC, CV, or even audio with peak, rms, and
  multiband mechanisms all available through a dead simple API
- Declarative per-sketch UI control definitions with frameword agnostic design
- Automatic store/recall of GUI control/parameters

### TODO

- [ ] Multichannel audio

## Status

This project is under active development.

## Requirements

This project has been developed on MacOS, though I'm sure most of it would work
on other platforms. This project requires or optionally needs:

- Rust
- Git LFS for screenshot storage (perhaps this is optional? I'm not too familiar
  with Git LFS but I'm using it for this so you might want to too)
- (optional) [just][just-link] for running commands
- (optional) ffmpeg available on your path for video exports

## Usage

### Creating a new sketch:

1. Copy the [template sketch][template-link] into a new file in sketches folder.
2. Rename at a minimum the `SKETCH_CONFIG.name` field at the top of the file:
   ```rust
   pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
      name: "template", // <-- RENAME THIS!
   ```
3. Add that filename to the [sketches module][module-link]
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

## Resources

- https://inconvergent.net/generative/
- http://www.complexification.net/
- https://n-e-r-v-o-u-s.com/projects/albums/floraform-system/
- https://www.andylomas.com/cellularFormImages.html
- http://www.complexification.net/gallery/machines/sandstroke/
- https://thebookofshaders.com/
- https://github.com/jasonwebb/2d-space-colonization-experiments
- https://paulbourke.net/geometry/

[nannou-link]: https://github.com/nannou-org/nannou
[p5-link]: https://github.com/Lokua/p5/tree/main
[just-link]: https://github.com/casey/just
[template-link]: src/sketches/template.rs
[midi-sketch-link]: src/sketches/midi_test.rs
[module-link]: src/sketches/mod.res
[main-link]: src/main.rs
[blackhole]: https://existential.audio/blackhole/
