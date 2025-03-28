import { useEffect, useState } from 'react'
import type { Control, Mappings, Slider } from './types.ts'
import { View } from './types.ts'
import Header from './Header.tsx'
import Controls from './Controls.tsx'
import Midi from './Midi.tsx'

type EventMap = {
  Advance: void
  Alert: string
  AverageFps: number
  Bpm: number
  CaptureFrame: void
  ClearBuffer: void
  CommitMappings: void
  CurrentlyMapping: string
  Encoding: boolean
  Error: string
  Hrcc: boolean
  HubPopulated: Control[]
  Init: {
    isLightTheme: boolean
    midiInputPort: string
    midiOutputPort: string
    midiInputPorts: [number, string][]
    midiOutputPorts: [number, string][]
    sketchNames: string[]
    sketchName: string
  }
  LoadSketch: {
    bpm: number
    controls: Control[]
    displayName: string
    fps: number
    paused: boolean
    mappings: Mappings
    sketchName: string
    tapTempoEnabled: boolean
  }
  Mappings: Mappings
  Paused: boolean
  PerfMode: boolean
  QueueRecord: void
  Ready: void
  RemoveMapping: string
  Reset: void
  Save: void
  SendMidi: void
  SnapshotEnded: Control[]
  SnapshotRecall: string
  SnapshotStore: string
  StartRecording: void
  StopRecording: void
  SwitchSketch: string
  Tap: void
  TapTempoEnabled: boolean
  ToggleFullScreen: void
  ToggleGuiFocus: void
  ToggleMainFocus: void
  TransitionTime: number
  UpdateControlBool: {
    name: string
    value: boolean
  }
  UpdateControlFloat: {
    name: string
    value: number
  }
  UpdateControlString: {
    name: string
    value: string
  }
}

function subscribe<K extends keyof EventMap>(
  callback: (event: K, data: EventMap[K]) => void
) {
  const handler = (e: MessageEvent) => {
    if (!e.data) return

    if (typeof e.data === 'string') {
      const event = e.data as K
      callback(event, undefined as unknown as EventMap[K])
    } else if (typeof e.data === 'object') {
      const eventName = Object.keys(e.data)[0] as K
      const eventData = e.data[eventName] as EventMap[K]
      callback(eventName, eventData)
    }
  }

  window.addEventListener('message', handler)

  return () => {
    window.removeEventListener('message', handler)
  }
}

function post(
  event: keyof EventMap,
  data?: boolean | number | string | object
) {
  if (data === undefined) {
    window.ipc.postMessage(JSON.stringify(event))
  } else {
    window.ipc.postMessage(
      JSON.stringify({
        [event]: data,
      })
    )
  }
}

export default function App() {
  const [alertText, setAlertText] = useState('')
  const [bpm, setBpm] = useState(134)
  const [controls, setControls] = useState<Control[]>([])
  const [fps, setFps] = useState(60)
  const [hrcc, setHrcc] = useState(false)
  const [isEncoding, setIsEncoding] = useState(false)
  const [isLightTheme, setIsLightTheme] = useState(true)
  const [isQueued, setIsQueued] = useState(false)
  const [isRecording, setIsRecording] = useState(false)
  const [mappings, setMappings] = useState<Mappings>([])
  const [mappingsEnabled, setMappingsEnabled] = useState(false)
  const [midiInputPort, setMidiInputPort] = useState('')
  const [midiInputPorts, setMidiInputPorts] = useState<string[]>([])
  const [midiOutputPort, setMidiOutputPort] = useState('')
  const [midiOutputPorts, setMidiOutputPorts] = useState<string[]>([])
  const [paused, setPaused] = useState(false)
  const [perfMode, setPerfMode] = useState(false)
  const [sketchName, setSketchName] = useState('')
  const [sketchNames, setSketchNames] = useState<string[]>([])
  const [tapTempoEnabled, setTapTempoEnabled] = useState(false)
  const [view, setView] = useState<View>(View.Controls)

  useEffect(() => {
    const unsubscribe = subscribe((event: keyof EventMap, data) => {
      if (event !== 'AverageFps') {
        console.debug('[app - sub event]:', event, 'data:', data)
      }

      switch (event) {
        case 'Alert': {
          setAlertText(data as EventMap['Alert'])
          break
        }
        case 'AverageFps': {
          setFps(data as EventMap['AverageFps'])
          break
        }
        case 'Bpm': {
          setBpm(data as EventMap['Bpm'])
          break
        }
        case 'Init': {
          const d = data as EventMap['Init']
          setIsLightTheme(d.isLightTheme)
          setMidiInputPort(d.midiInputPort)
          setMidiOutputPort(d.midiOutputPort)
          const getPort = ([, port]: [number, string]) => port
          setMidiInputPorts(d.midiInputPorts.map(getPort))
          setMidiOutputPorts(d.midiOutputPorts.map(getPort))
          setSketchName(d.sketchName)
          setSketchNames(d.sketchNames)
          break
        }
        case 'HubPopulated': {
          setControls(data as EventMap['HubPopulated'])
          break
        }
        case 'LoadSketch': {
          const d = data as EventMap['LoadSketch']
          setBpm(d.bpm)
          setControls(d.controls)
          setFps(d.fps)
          setMappings(d.mappings)
          setPaused(d.paused)
          setSketchName(d.sketchName)
          setTapTempoEnabled(d.tapTempoEnabled)
          break
        }
        case 'Mappings': {
          setMappings(data as EventMap['Mappings'])
          break
        }
        case 'StartRecording': {
          setIsRecording(true)
          setIsQueued(false)
          break
        }
        case 'Encoding': {
          setIsEncoding(data as EventMap['Encoding'])
          if (data) {
            setIsQueued(false)
            setIsRecording(false)
          }
          break
        }
        case 'SnapshotEnded': {
          setControls(data as EventMap['SnapshotEnded'])
          break
        }
      }
    })

    post('Ready')

    return () => {
      unsubscribe()
    }
  }, [])

  useEffect(() => {
    document.addEventListener('keydown', onKeyDown)

    function onKeyDown(e: KeyboardEvent) {
      console.debug('[onKeyDown] e:', e)

      if (e.code.startsWith('Digit')) {
        if (e.metaKey) {
          post('SnapshotRecall', e.key)
        } else if (e.shiftKey) {
          const actualKey = e.code.slice('Digit'.length)
          post('SnapshotStore', actualKey)
        }
      }

      switch (e.code) {
        case 'KeyA': {
          if (paused) {
            post('Advance')
          }
          break
        }
        case 'KeyF': {
          if (e.metaKey) {
            post('ToggleFullScreen')
          }
          break
        }
        case 'KeyG': {
          if (e.metaKey) {
            post('ToggleGuiFocus')
          }
          break
        }
        case 'KeyM': {
          if (e.shiftKey && e.metaKey) {
            const newView = view === View.Controls ? View.Midi : View.Controls
            setView(newView)
          } else if (e.metaKey) {
            post('ToggleMainFocus')
          }
          break
        }
        case 'KeyS': {
          if (e.metaKey || e.shiftKey) {
            post('Save')
          } else {
            post('CaptureFrame')
          }
          break
        }
        case 'Space': {
          if (tapTempoEnabled) {
            post('Tap')
          }
          break
        }
      }
    }

    return () => {
      document.removeEventListener('keydown', onKeyDown)
    }
  }, [paused, tapTempoEnabled, view])

  useEffect(() => {
    document.body.classList.add(isLightTheme ? 'light' : 'dark')
    document.body.classList.remove(isLightTheme ? 'dark' : 'light')
  }, [isLightTheme])

  function getSliderNames() {
    return controls.reduce<string[]>((names, control) => {
      const type = Object.keys(control)[0]
      if (type === 'slider') {
        names.push((control as Slider).slider.name)
      }
      return names
    }, [])
  }

  function onAdvance() {
    post('Advance')
  }

  function onCaptureFrame() {
    post('CaptureFrame')
  }

  function onChangeControl(
    type: string,
    name: string,
    value: boolean | string | number,
    controls: Control[]
  ) {
    setControls(controls)

    const event: keyof EventMap =
      type === 'checkbox'
        ? 'UpdateControlBool'
        : type === 'slider'
        ? 'UpdateControlFloat'
        : 'UpdateControlString'

    post(event, {
      name,
      value,
    })
  }

  function onChangeHrcc() {
    const value = !hrcc
    setHrcc(value)
    post('Hrcc', value)
    setAlertText(
      value
        ? 'Expecting 14bit MIDI on channels 0-31'
        : 'Expecting standard 7bit MIDI messages for all CCs'
    )
  }

  function onChangeInputPort(port: string) {
    setMidiInputPort(port)
    setAlertText('Changing ports at runtime is not yet supported')
  }

  function onChangeOutputPort(port: string) {
    setMidiOutputPort(port)
    setAlertText('Changing ports at runtime is not yet supported')
  }

  function onChangeMappingsEnabled() {
    setMappingsEnabled(!mappingsEnabled)
  }

  function onChangePerfMode() {
    const value = !perfMode
    setPerfMode(value)
    post('PerfMode', value)
    setAlertText(
      value
        ? 'When `Perf` is enabled, the sketch window will not be resized \
        when switching sketches.'
        : ''
    )
  }

  function onChangeTapTempoEnabled() {
    const enabled = !tapTempoEnabled
    setTapTempoEnabled(enabled)
    post('TapTempoEnabled', enabled)
    setAlertText(
      enabled ? 'Tap `Space` key to set BPM' : 'Sketch BPM has been restored'
    )
  }

  function onChangeTransitionTime(time: number) {
    post('TransitionTime', time)
  }

  function onChangeView() {
    const v = view === View.Controls ? View.Midi : View.Controls
    setView(v)
    if (v === View.Controls) {
      post('CommitMappings')
    }
  }

  function onClearBuffer() {
    post('ClearBuffer')
  }

  function onClickSendMidi() {
    post('SendMidi')
  }

  function onQueueRecord() {
    const value = !isQueued
    setIsQueued(value)
    post('QueueRecord')
    setAlertText(value ? 'Recording queued. Awaiting MIDI start message.' : '')
  }

  function onRecord() {
    if (isRecording) {
      setIsRecording(false)
      post('StopRecording')
    } else {
      setIsRecording(true)
      post('StartRecording')
    }
  }

  function onRemoveMapping(name: string) {
    post('RemoveMapping', name)
  }

  function onReset() {
    post('Reset')
  }

  function onSave() {
    post('Save')
  }

  function onSetCurrentlyMapping(name: string) {
    post('CurrentlyMapping', name)
  }

  function onSwitchSketch(sketchName: string) {
    post('SwitchSketch', sketchName)
  }

  function onTogglePlay() {
    const value = !paused
    setPaused(value)
    post('Paused', value)
  }

  return (
    <div id="app">
      <Header
        fps={fps}
        bpm={bpm}
        isEncoding={isEncoding}
        isQueued={isQueued}
        isRecording={isRecording}
        paused={paused}
        perfMode={perfMode}
        sketchName={sketchName}
        sketchNames={sketchNames}
        tapTempoEnabled={tapTempoEnabled}
        view={view}
        onAdvance={onAdvance}
        onCaptureFrame={onCaptureFrame}
        onChangePerfMode={onChangePerfMode}
        onChangeTapTempoEnabled={onChangeTapTempoEnabled}
        onChangeTransitionTime={onChangeTransitionTime}
        onChangeView={onChangeView}
        onClearBuffer={onClearBuffer}
        onReset={onReset}
        onQueueRecord={onQueueRecord}
        onRecord={onRecord}
        onSave={onSave}
        onSwitchSketch={onSwitchSketch}
        onTogglePlay={onTogglePlay}
      />
      <main>
        {view === View.Midi ? (
          <Midi
            hrcc={hrcc}
            inputPort={midiInputPort}
            inputPorts={midiInputPorts}
            outputPort={midiOutputPort}
            outputPorts={midiOutputPorts}
            mappingsEnabled={mappingsEnabled}
            mappings={mappings}
            sliderNames={getSliderNames()}
            onChangeHrcc={onChangeHrcc}
            onChangeInputPort={onChangeInputPort}
            onChangeOutputPort={onChangeOutputPort}
            onChangeMappingsEnabled={onChangeMappingsEnabled}
            onClickSend={onClickSendMidi}
            onRemoveMapping={onRemoveMapping}
            onSetCurrentlyMapping={onSetCurrentlyMapping}
          />
        ) : (
          <Controls controls={controls} onChange={onChangeControl} />
        )}
      </main>
      <footer>
        <div className="console">{alertText}</div>
      </footer>
    </div>
  )
}
