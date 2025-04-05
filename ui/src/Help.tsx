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

export const Title = {
  CaptureImage: 'Capture image',
  Perf: 'Enable/disable Performance Mode',
  Queue: 'Queue recording to start upon receiving a MIDI Start message',
  Random: 'Randomize all UI controls',
  Record: 'Start/Stop recording',
  Tap: 'Enabled/disable tap tempo',
  TransitionTime: 'Snapshot and Randomization transition time (in beats)',
  Settings: 'Global settings and MIDI mappings',
  Sketch: 'Sketch chooser',
  Save: format(`
    Save UI control states and MIDI mappings for this sketch as well as all 
    global settings like ports
  `),
}

function format(s: string): string {
  return s
    .split('\n')
    .filter((s) => s)
    .map((line) => line.trim())
    .join(' ')
}
