export const Alert = {
  Midi14Bit: 'Expecting 14bit MIDI on channels 0-31',
  Midi7Bit: 'Expecting standard 7bit MIDI messages for all CCs',
  PerfEnabled: format(`
    When \`Perf\` is enabled, the sketch window will not be resized or 
    repositioned when switching sketches.
  `),
  Queued: 'Recording queued. Awaiting MIDI start message.',
  TapEnabled: 'Tap `Space` key to set BPM',
  TapDisabled: 'Sketch BPM has been restored',
}

export const Help = {
  Advance: format(
    `When the [Play/Pause] toggle is set to [Pause], allows manually advancing 
    frames (Shortcut: [Meta+A])`
  ),
  Audio: 'The Audio input device used for audio controls',
  Clear: format(
    `Clear any alpha blending or "fade trails" from frame persistence. Requires 
    your sketch is using the clear_color attribute via sketch_components macro`
  ),
  Exclusions: format(
    `Toggle Exclusions Mode where you can select controls to exclude from 
    Randomization`
  ),
  Fps: 'The effective framerate over a 1 second running average',
  Hrcc: format(`
    Enable high resolution (14bit) MIDI for CCs 0-31 (requires support 
    from your MIDI device)
  `),
  Image: 'Capture image to disk (Shortcut: [Meta+S])',
  IncludeCheckboxes: format(`
      When enabled, the [Randomization] feature will also randomize checkbox 
      controls
  `),
  IncludeSelects: format(`
      When enabled, the [Randomization] feature will also randomize select 
      values
  `),
  Mappings: format(`
      Mappings: allows run-time mapping of external MIDI CCs to UI sliders, aka
      "MIDI Learn". Mappings are saved with the sketch when you click [Save]. 
  `),
  MidiClockPort:
    "The MIDI port used to sync all Lattice's frame counter and animations",
  MidiInputPort:
    'The MIDI port Lattice will listen to for incoming MIDI CC messages',
  MidiOutputPort: format(`
    The MIDI port Lattice will send internally stored MIDI values 
    to (use for resyncing controllers after changing sketches)
  `),
  OscPort: 'The OSC port Lattice will use for OSC controls',
  Play: format(`
    Play/Pause Toggle. When Pause is engaged, use the [Advance] button 
    or [Meta+A] to manually advance frames.
  `),
  Perf: format(
    `Enable/disable Performance Mode. When enabled, prevents Lattice from 
    applying a sketch's default width and height and also disables automatic 
    window repositioning. This is necessary in live performance contexts where 
    you likely will fullsize the screen and want to keep it that way when 
    switching sketches`
  ),
  Queue: 'Queue recording to start upon receiving a MIDI Start message',
  Random: 'Randomize all UI controls',
  Record: 'Start/Stop recording',
  Reset: 'Reset the frame counter and all animations to their starting points',
  Tap: `
    Enabled/disable tap tempo. When enabled, use the [Space] key to tap. 
    Note that keeping enabled will preserve the currently tapped in tempo when 
    switching sketches; disabling will always revert to a sketch's configured BPM.
  `,
  TransitionTime: 'Snapshot and Randomization transition time (in beats)',
  Send: 'Sends the state of all CCs to the MIDI output port',
  Settings: 'Global settings and MIDI mappings',
  Sketch: 'Sketch chooser',
  Save: format(`
    Save UI control states and MIDI mappings for this sketch as well as all 
    global settings like ports (global and sketch settings are separate)
  `),
}

function format(s: string): string {
  return s
    .split('\n')
    .filter((s) => s)
    .map((line) => line.trim())
    .join(' ')
}
