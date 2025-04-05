export const Alert = {
  Midi14Bit: 'Expecting 14bit MIDI on channels 0-31',
  Midi7Bit: 'Expecting standard 7bit MIDI messages for all CCs',
  PerfEnabled: `
    When \`Perf\` is enabled, the sketch window will not be resized or 
    repositioned when switching sketches.
  `,
  Queued: 'Recording queued. Awaiting MIDI start message.',
  TapEnabled: 'Tap `Space` key to set BPM',
  TapDisabled: 'Sketch BPM has been restored',
}

export const Title = {
  Perf: 'Enable/disable Performance Mode',
  Random: 'Randomize all UI controls',
  Tap: 'Enabled/disable tap tempo',
  TransitionTime: 'Snapshot transition time (in beats)',
  Settings: 'Global settings and MIDI mappings',
  Sketch: 'Sketch chooser',
  Save: `
    Save UI control states and MIDI mappings for this sketch as well as all 
    global settings like ports',
  `,
}
