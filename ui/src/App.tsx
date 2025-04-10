import { useEffect, useState } from 'react'
import type {
  Bypassed,
  Control,
  ControlValue,
  Exclusions,
  Mappings,
  RawControl,
} from './types'
import { View } from './types'
import Header from './Header'
import Controls from './Controls'
import Snapshots from './Snapshots'
import Settings from './Settings'
import Console from './Console'
import { Alert } from './Help'

type EventMap = {
  Advance: void
  Alert: string
  AverageFps: number
  Bpm: number
  CaptureFrame: void
  ChangeAudioDevice: string
  ChangeMidiClockPort: string
  ChangeMidiControlInputPort: string
  ChangeMidiControlOutputPort: string
  ChangeOscPort: number
  ClearBuffer: void
  CommitMappings: void
  CurrentlyMapping: string
  Encoding: boolean
  Error: string
  Hrcc: boolean
  HubPopulated: [RawControl[], Bypassed]
  Init: {
    audioDevice: string
    audioDevices: string[]
    hrcc: boolean
    isLightTheme: boolean
    mappingsEnabled: boolean
    midiClockPort: string
    midiInputPort: string
    midiOutputPort: string
    midiInputPorts: [number, string][]
    midiOutputPorts: [number, string][]
    oscPort: number
    sketchNames: string[]
    sketchName: string
    transitionTime: number
  }
  LoadSketch: {
    bpm: number
    bypassed: Bypassed
    controls: RawControl[]
    exclusions: Exclusions
    fps: number
    paused: boolean
    mappings: Mappings
    sketchName: string
    snapshotSlots: string[]
    tapTempoEnabled: boolean
  }
  Mappings: Mappings
  MappingsEnabled: boolean
  Paused: boolean
  PerfMode: boolean
  QueueRecord: void
  Quit: void
  Randomize: Exclusions
  Ready: void
  RemoveMapping: string
  Reset: void
  Save: string[]
  SendMidi: void
  SnapshotEnded: RawControl[]
  SnapshotDelete: string
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
  UpdatedControls: RawControl[]
}

function subscribe<K extends keyof EventMap>(
  callback: (event: K, data: EventMap[K]) => void
) {
  function handler(e: MessageEvent) {
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

function post<K extends keyof EventMap>(
  event: EventMap[K] extends void ? K : never
): void
function post<K extends keyof EventMap>(
  event: EventMap[K] extends void ? never : K,
  data: EventMap[K]
): void
function post<K extends keyof EventMap>(event: K, data?: EventMap[K]): void {
  if (data === undefined) {
    window.ipc.postMessage(JSON.stringify(event))
  } else {
    window.ipc.postMessage(JSON.stringify({ [event]: data }))
  }
}

function stringToControlValue(s: string): ControlValue {
  if (s === 'true') {
    return true
  }

  if (s === 'false') {
    return false
  }

  const n = parseFloat(s)
  if (!isNaN(n) && String(n) === s) {
    return n
  }

  return s
}

function fromRawControls(raw_controls: RawControl[]): Control[] {
  return raw_controls.map((control) => ({
    ...control,
    value: stringToControlValue(control.value),
    isRawControl: false,
  }))
}

export default function App() {
  const [alertText, setAlertText] = useState('')
  const [audioDevices, setAudioDevices] = useState<string[]>([])
  const [audioDevice, setAudioDevice] = useState('')
  const [bpm, setBpm] = useState(134)
  const [bypassed, setBypassed] = useState<Bypassed>({})
  const [childView, setChildView] = useState<View>(View.Default)
  const [controls, setControls] = useState<Control[]>([])
  const [exclusions, setExclusions] = useState<string[]>([])
  const [fps, setFps] = useState(60)
  const [hrcc, setHrcc] = useState(false)
  const [isEncoding, setIsEncoding] = useState(false)
  const [isQueued, setIsQueued] = useState(false)
  const [isRecording, setIsRecording] = useState(false)
  const [mappings, setMappings] = useState<Mappings>([])
  const [mappingsEnabled, setMappingsEnabled] = useState(true)
  const [midiClockPort, setMidiClockPort] = useState('')
  const [midiInputPort, setMidiInputPort] = useState('')
  const [midiInputPorts, setMidiInputPorts] = useState<string[]>([])
  const [midiOutputPort, setMidiOutputPort] = useState('')
  const [midiOutputPorts, setMidiOutputPorts] = useState<string[]>([])
  const [oscPort, setOscPort] = useState(5000)
  const [paused, setPaused] = useState(false)
  const [perfMode, setPerfMode] = useState(false)
  const [sketchName, setSketchName] = useState('')
  const [sketchNames, setSketchNames] = useState<string[]>([])
  const [snapshots, setSnapshots] = useState<string[]>([])
  const [tapTempoEnabled, setTapTempoEnabled] = useState(false)
  const [transitionTime, setTransitionTime] = useState(4)
  const [view, setView] = useState<View>(View.Controls)

  useEffect(() => {
    const unsubscribe = subscribe((event: keyof EventMap, data) => {
      if (event !== 'AverageFps') {
        console.debug('[app]', {
          event,
          data,
        })
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
        case 'Encoding': {
          setIsEncoding(data as EventMap['Encoding'])
          if (data) {
            setIsQueued(false)
            setIsRecording(false)
          }
          break
        }
        case 'HubPopulated': {
          const [controls, bypassed] = data as EventMap['HubPopulated']
          setControls(fromRawControls(controls))
          setBypassed(bypassed)
          break
        }
        case 'Init': {
          const d = data as EventMap['Init']
          setAudioDevice(d.audioDevice)
          setAudioDevices(d.audioDevices)
          setHrcc(d.hrcc)
          setMappingsEnabled(d.mappingsEnabled)
          setMidiClockPort(d.midiClockPort)
          setMidiInputPort(d.midiInputPort)
          setMidiOutputPort(d.midiOutputPort)
          const getPort = ([, port]: [number, string]) => port
          setMidiInputPorts(d.midiInputPorts.map(getPort))
          setMidiOutputPorts(d.midiOutputPorts.map(getPort))
          setOscPort(d.oscPort)
          setSketchName(d.sketchName)
          setSketchNames(d.sketchNames)
          setTransitionTime(d.transitionTime)
          break
        }
        case 'LoadSketch': {
          const d = data as EventMap['LoadSketch']
          setBpm(d.bpm)
          setBypassed(d.bypassed)
          setControls(fromRawControls(d.controls))
          setExclusions(d.exclusions)
          setFps(d.fps)
          setMappings(d.mappings)
          setPaused(d.paused)
          setSketchName(d.sketchName)
          setSnapshots(d.snapshotSlots)
          setTapTempoEnabled(d.tapTempoEnabled)
          break
        }
        case 'Mappings': {
          setMappings(data as EventMap['Mappings'])
          break
        }
        case 'SnapshotEnded': {
          setControls(fromRawControls(data as EventMap['SnapshotEnded']))
          setAlertText('Snapshot ended')
          break
        }
        case 'StartRecording': {
          setIsRecording(true)
          setIsQueued(false)
          break
        }
        case 'UpdatedControls': {
          setControls(fromRawControls(data as EventMap['UpdatedControls']))
          break
        }
        default: {
          break
        }
      }
    })

    post('Ready')

    return () => {
      console.log('[app] Unsubscribing')
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
          setAlertText(`Snapshot ${e.key} saved`)
        } else if (e.shiftKey) {
          const actualKey = e.code.slice('Digit'.length)
          post('SnapshotStore', actualKey)
        }
      }

      switch (e.code) {
        case 'Comma': {
          setView(view === View.Settings ? View.Controls : View.Settings)
          break
        }
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
          if (e.metaKey && !e.shiftKey) {
            post('ToggleMainFocus')
          }
          break
        }
        case 'KeyQ': {
          if (e.metaKey) {
            post('Quit')
          }
          break
        }
        case 'KeyR': {
          if (e.metaKey && e.shiftKey) {
            post('SwitchSketch', sketchName)
          } else if (e.metaKey) {
            post('Randomize', exclusions)
          } else {
            post('Reset')
          }
          break
        }
        case 'KeyS': {
          if (e.metaKey || e.shiftKey) {
            post('Save', exclusions)
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
        default: {
          break
        }
      }
    }

    return () => {
      document.removeEventListener('keydown', onKeyDown)
    }
  }, [paused, tapTempoEnabled, view, controls, exclusions, sketchName])

  function getSliderNames() {
    return controls
      .filter((control) => control.kind === 'Slider')
      .map((control) => control.name)
  }

  function onAdvance() {
    post('Advance')
  }

  function onCaptureFrame() {
    post('CaptureFrame')
  }

  function onChangeAudioDevice(name: string) {
    setAudioDevice(name)
    post('ChangeAudioDevice', name)
  }

  function onChangeControl(control: Control, value: ControlValue) {
    setControls(
      controls.map((c) =>
        c.name === control.name
          ? {
              ...c,
              value,
            }
          : c
      )
    )

    const event: keyof EventMap =
      control.kind === 'Checkbox'
        ? 'UpdateControlBool'
        : control.kind === 'Slider'
        ? 'UpdateControlFloat'
        : 'UpdateControlString'

    post(event, {
      name: control.name,
      value,
    })
  }

  function onChangeHrcc() {
    const value = !hrcc
    setHrcc(value)
    post('Hrcc', value)
    setAlertText(value ? Alert.Midi14Bit : Alert.Midi7Bit)
  }

  function onChangeMidiClockPort(port: string) {
    setMidiClockPort(port)
    post('ChangeMidiClockPort', port)
  }

  function onChangeMidiInputPort(port: string) {
    setMidiInputPort(port)
    post('ChangeMidiControlInputPort', port)
  }

  function onChangeMidiOutputPort(port: string) {
    setMidiOutputPort(port)
    post('ChangeMidiControlOutputPort', port)
  }

  function onChangeMappingsEnabled() {
    const enabled = !mappingsEnabled
    setMappingsEnabled(enabled)
    post('MappingsEnabled', enabled)
  }

  function onChangeOscPort(port: number) {
    setOscPort(port)
    post('ChangeOscPort', port)
  }

  function onChangePerfMode() {
    const value = !perfMode
    setPerfMode(value)
    post('PerfMode', value)
    setAlertText(value ? Alert.PerfEnabled : '')
  }

  function onChangeTapTempoEnabled() {
    const enabled = !tapTempoEnabled
    setTapTempoEnabled(enabled)
    post('TapTempoEnabled', enabled)
    setAlertText(enabled ? Alert.TapEnabled : Alert.TapDisabled)
  }

  function onChangeTransitionTime(time: number) {
    setTransitionTime(time)
    post('TransitionTime', time)
  }

  function onChangeView() {
    const v = view === View.Controls ? View.Settings : View.Controls
    setView(v)
    if (v === View.Controls) {
      post('CommitMappings')
    }
  }

  function onChangeViewMain(initiator: View) {
    if (childView === View.Default || childView !== initiator) {
      setChildView(initiator)
    } else if (childView === initiator) {
      setChildView(View.Default)
    }
  }

  function onClearBuffer() {
    post('ClearBuffer')
  }

  function onClickRandomize() {
    post('Randomize', exclusions)
  }

  function onClickRandomizeSingleControl(name: string) {
    post(
      'Randomize',
      controls.filter((c) => c.name !== name).map((c) => c.name)
    )
  }

  function onClickSendMidi() {
    post('SendMidi')
  }

  function onDeleteMappings() {
    mappings.forEach((mapping) => {
      post('RemoveMapping', mapping[0])
    })
    setMappings([])
  }

  function onQueueRecord() {
    const value = !isQueued
    setIsQueued(value)
    post('QueueRecord')
    setAlertText(value ? Alert.Queued : '')
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

  function onReload() {
    post('SwitchSketch', sketchName)
  }

  function onRemoveMapping(name: string) {
    post('RemoveMapping', name)
  }

  function onReset() {
    post('Reset')
  }

  function onSave() {
    post('Save', exclusions)
  }

  function onSetCurrentlyMapping(name: string) {
    post('CurrentlyMapping', name)
  }

  function onSnapshotDeleteAll() {
    snapshots.forEach((slot) => {
      post('SnapshotDelete', slot)
    })
    setSnapshots([])
  }

  function onSnapshotDelete(slot: string) {
    setSnapshots(snapshots.filter((s) => s !== slot))
    post('SnapshotDelete', slot)
  }

  function onSnapshotLoad(slot: string) {
    post('SnapshotRecall', slot)
  }

  function onSnapshotSave(slot: string) {
    setSnapshots(snapshots.concat(slot).slice().sort())
    post('SnapshotStore', slot)
  }

  function onSwitchSketch(sketchName: string) {
    post('SwitchSketch', sketchName)
  }

  function onTogglePlay() {
    const value = !paused
    setPaused(value)
    post('Paused', value)
  }

  function onToggleExclusion(name: string) {
    setExclusions(
      exclusions.includes(name)
        ? exclusions.filter((n) => n !== name)
        : exclusions.concat(name)
    )
  }

  return (
    <div id="app">
      <Header
        fps={fps}
        bpm={bpm}
        childView={childView}
        isEncoding={isEncoding}
        isQueued={isQueued}
        isRecording={isRecording}
        paused={paused}
        perfMode={perfMode}
        sketchName={sketchName}
        sketchNames={sketchNames}
        tapTempoEnabled={tapTempoEnabled}
        transitionTime={transitionTime}
        view={view}
        onAdvance={onAdvance}
        onCaptureFrame={onCaptureFrame}
        onChangeChildView={onChangeViewMain}
        onChangePerfMode={onChangePerfMode}
        onChangeTapTempoEnabled={onChangeTapTempoEnabled}
        onChangeTransitionTime={onChangeTransitionTime}
        onChangeView={onChangeView}
        onClearBuffer={onClearBuffer}
        onClickRandomize={onClickRandomize}
        onQueueRecord={onQueueRecord}
        onReload={onReload}
        onReset={onReset}
        onRecord={onRecord}
        onSave={onSave}
        onSwitchSketch={onSwitchSketch}
        onTogglePlay={onTogglePlay}
      />
      <main>
        {view === View.Settings ? (
          <Settings
            audioDevice={audioDevice}
            audioDevices={audioDevices}
            hrcc={hrcc}
            mappings={mappings}
            mappingsEnabled={mappingsEnabled}
            midiClockPort={midiClockPort}
            midiInputPort={midiInputPort}
            midiInputPorts={midiInputPorts}
            midiOutputPort={midiOutputPort}
            midiOutputPorts={midiOutputPorts}
            oscPort={oscPort}
            sliderNames={getSliderNames()}
            onChangeAudioDevice={onChangeAudioDevice}
            onChangeHrcc={onChangeHrcc}
            onChangeMappingsEnabled={onChangeMappingsEnabled}
            onChangeMidiClockPort={onChangeMidiClockPort}
            onChangeMidiInputPort={onChangeMidiInputPort}
            onChangeMidiOutputPort={onChangeMidiOutputPort}
            onChangeOscPort={onChangeOscPort}
            onClickSend={onClickSendMidi}
            onDeleteMappings={onDeleteMappings}
            onRemoveMapping={onRemoveMapping}
            onSetCurrentlyMapping={onSetCurrentlyMapping}
          />
        ) : childView === View.Snapshots ? (
          <Snapshots
            snapshots={snapshots}
            onDelete={onSnapshotDelete}
            onDeleteAll={onSnapshotDeleteAll}
            onLoad={onSnapshotLoad}
            onSave={onSnapshotSave}
          />
        ) : (
          <Controls
            bypassed={bypassed}
            controls={controls}
            exclusions={exclusions}
            mappings={mappings}
            mappingsEnabled={mappingsEnabled}
            showExclusions={childView == View.Exclusions}
            onChange={onChangeControl}
            onClickRandomize={onClickRandomizeSingleControl}
            onToggleExclusion={onToggleExclusion}
          />
        )}
      </main>
      <footer>
        <Console alertText={alertText} />
      </footer>
    </div>
  )
}
