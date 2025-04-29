import { useCallback, useEffect, useState } from 'react'

import {
  Bypassed,
  Control,
  ControlKind,
  ControlValue,
  Exclusions,
  Mappings,
  OsDir,
  RawControl,
  UserDir,
  View,
} from './types'

import Header from './Header'
import Controls from './Controls'
import Settings from './Settings'
import Console from './Console'
import useKeyDownOnce from './useKeyDownOnce'
import { isMac, setCssBeat } from './util'

type EventMap = {
  Advance: void
  Alert: string
  AverageFps: number
  Bpm: number
  CaptureFrame: void
  ChangeAudioDevice: string
  ChangeDir: UserDir
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
    imagesDir: string
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
    userDataDir: string
    videosDir: string
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
  OpenOsDir: OsDir
  Paused: boolean
  PerfMode: boolean
  QueueRecord: void
  Quit: void
  Randomize: Exclusions
  Ready: void
  ReceiveDir: [UserDir, string]
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

function toControlValue(kind: ControlKind, s: string): ControlValue {
  if (kind === 'Checkbox') {
    if (s === 'true') {
      return true
    }

    if (s === 'false') {
      return false
    }
  }

  if (kind === 'Slider') {
    return Number(s)
  }

  return s
}

function fromRawControls(raw_controls: RawControl[]): Control[] {
  return raw_controls.map((control) => ({
    ...control,
    value: toControlValue(control.kind, control.value),
    isRawControl: false,
  }))
}

export default function App() {
  const [alertText, setAlertText] = useState('')
  const [audioDevices, setAudioDevices] = useState<string[]>([])
  const [audioDevice, setAudioDevice] = useState('')
  const [bpm, setBpm] = useState(134)
  const [bypassed, setBypassed] = useState<Bypassed>({})
  const [controls, setControls] = useState<Control[]>([])
  const [controlsLastSaved, setControlsLastSaved] = useState<Control[]>([])
  const [exclusions, setExclusions] = useState<string[]>([])
  const [fps, setFps] = useState(60)
  const [hrcc, setHrcc] = useState(false)
  const [imagesDir, setImagesDir] = useState('')
  const [isEncoding, setIsEncoding] = useState(false)
  const [isQueued, setIsQueued] = useState(false)
  const [isRecording, setIsRecording] = useState(false)
  const [mappings, setMappings] = useState<Mappings>({})
  const [mappingsEnabled, setMappingsEnabled] = useState(true)
  const [midiClockPort, setMidiClockPort] = useState('')
  const [midiInputPort, setMidiInputPort] = useState('')
  const [midiInputPorts, setMidiInputPorts] = useState<string[]>([])
  const [midiOutputPort, setMidiOutputPort] = useState('')
  const [midiOutputPorts, setMidiOutputPorts] = useState<string[]>([])
  const [oscPort, setOscPort] = useState(5000)
  const [paused, setPaused] = useState(false)
  const [perfMode, setPerfMode] = useState(false)
  const [showExclusions, setShowExclusions] = useState(false)
  const [showSnapshots, setShowSnapshots] = useState(false)
  const [singleTransitionControlName, setSingleTransitionControlName] =
    useState('')
  const [sketchName, setSketchName] = useState('')
  const [sketchNames, setSketchNames] = useState<string[]>([])
  const [snapshots, setSnapshots] = useState<string[]>([])
  const [tapTempoEnabled, setTapTempoEnabled] = useState(false)
  const [transitionTime, setTransitionTime] = useState(4)
  const [transitionInProgress, setTransitionInProgress] = useState(false)
  const [videosDir, setVideosDir] = useState('')
  const [userDataDir, setUserDataDir] = useState('')
  const [view, setView] = useState<View>(View.Controls)

  useEffect(() => {
    const unsubscribe = subscribe((event: keyof EventMap, data) => {
      if (event !== 'AverageFps') {
        console.debug('[app]', event, data)
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
          const bpm = data as EventMap['Bpm']
          setBpm(bpm)
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
          setImagesDir(d.imagesDir)
          setMappingsEnabled(d.mappingsEnabled)
          setMidiClockPort(d.midiClockPort)
          setMidiInputPort(d.midiInputPort)
          setMidiOutputPort(d.midiOutputPort)
          const getPort = ([, port]: [number, string]) => port
          setMidiInputPorts(d.midiInputPorts.map(getPort))
          setMidiOutputPorts(d.midiOutputPorts.map(getPort))
          setOscPort(d.oscPort)
          setUserDataDir(d.userDataDir)
          setSketchName(d.sketchName)
          setSketchNames(d.sketchNames)
          setTransitionTime(d.transitionTime)
          setVideosDir(d.videosDir)
          break
        }
        case 'LoadSketch': {
          const d = data as EventMap['LoadSketch']
          setBpm(d.bpm)
          setBypassed(d.bypassed)
          const controls = fromRawControls(d.controls)
          setControls(controls)
          setControlsLastSaved(controls)
          setExclusions(d.exclusions)
          setFps(d.fps)
          setMappings(d.mappings)
          setPaused(d.paused)
          setSketchName(d.sketchName)
          setSnapshots(d.snapshotSlots)
          // TODO: why are we sending this with the sketch?
          setTapTempoEnabled(d.tapTempoEnabled)
          break
        }
        case 'Mappings': {
          setMappings(data as EventMap['Mappings'])
          break
        }
        case 'ReceiveDir': {
          const [kind, dir] = data as EventMap['ReceiveDir']
          if (kind === UserDir.Images) {
            setImagesDir(dir)
          } else if (kind === UserDir.UserData) {
            setUserDataDir(dir)
          } else {
            setImagesDir(dir)
          }
          break
        }
        case 'SnapshotEnded': {
          setControls(fromRawControls(data as EventMap['SnapshotEnded']))
          setTransitionInProgress(false)
          setSingleTransitionControlName('')
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

  useKeyDownOnce(
    useCallback(
      (e: KeyboardEvent) => {
        console.debug('[onKeyDown] e:', e)

        const platformModPressed = isMac ? e.metaKey : e.ctrlKey

        if (e.code.startsWith('Digit')) {
          if (platformModPressed) {
            post('SnapshotRecall', e.key)
            if (snapshots.includes(e.key)) {
              setTransitionInProgress(true)
            } else {
              // It's fine, the backend will alert
            }
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
          case 'KeyE': {
            setShowExclusions(!showExclusions)
            break
          }
          case 'KeyF': {
            post('ToggleFullScreen')
            break
          }
          case 'KeyI': {
            post('CaptureFrame')
            break
          }
          case 'KeyM': {
            // Don't interfere with native minimization on macOS
            if (!platformModPressed) {
              post('ToggleMainFocus')
            }
            break
          }
          case 'KeyP': {
            const value = !paused
            setPaused(value)
            post('Paused', value)
            break
          }
          case 'KeyQ': {
            if (platformModPressed) {
              post('Quit')
            }
            break
          }
          case 'KeyR': {
            if (platformModPressed && e.shiftKey) {
              post('SwitchSketch', sketchName)
            } else if (platformModPressed) {
              post('Randomize', exclusions)
            } else {
              post('Reset')
            }
            break
          }
          case 'KeyS': {
            if (platformModPressed || e.shiftKey) {
              post('Save', exclusions)
            } else {
              setShowSnapshots(!showSnapshots)
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
      },
      [
        exclusions,
        paused,
        showExclusions,
        showSnapshots,
        sketchName,
        tapTempoEnabled,
        view,
      ]
    )
  )

  useEffect(() => {
    setCssBeat(bpm)
  }, [bpm])

  function getSliderNames() {
    return controls
      .filter((control) => control.kind === 'Slider')
      .map((control) => control.name)
  }

  function updateEventForControl(control: Control): keyof EventMap {
    return control.kind === 'Checkbox'
      ? 'UpdateControlBool'
      : control.kind === 'Slider'
      ? 'UpdateControlFloat'
      : 'UpdateControlString'
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

    post(updateEventForControl(control), {
      name: control.name,
      value,
    })
  }

  function onChangeFolder(kind: UserDir) {
    post('ChangeDir', kind)
  }

  function onChangeHrcc() {
    const value = !hrcc
    setHrcc(value)
    post('Hrcc', value)
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
  }

  function onChangeTapTempoEnabled() {
    const enabled = !tapTempoEnabled
    setTapTempoEnabled(enabled)
    post('TapTempoEnabled', enabled)
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

  function onClearBuffer() {
    post('ClearBuffer')
  }

  function onClickRandomize() {
    post('Randomize', exclusions)
    setTransitionInProgress(true)
  }

  function onClickRandomizeSingleControl(name: string) {
    post(
      'Randomize',
      controls.filter((c) => c.name !== name).map((c) => c.name)
    )
    setSingleTransitionControlName(name)
  }

  function onClickRevert(control: Control) {
    const originalControl = controlsLastSaved.find(
      (c) => c.name === control.name
    )!
    const updatedControl = {
      ...control,
      value: originalControl.value,
    }

    setControls(
      controls.map((c) => {
        if (c.name === control.name) {
          return updatedControl
        }

        return c
      })
    )

    post(updateEventForControl(control), {
      name: updatedControl.name,
      value: updatedControl.value,
    })
  }

  function onClickSendMidi() {
    post('SendMidi')
  }

  function onDeleteMappings() {
    Object.keys(mappings).forEach((key) => {
      post('RemoveMapping', key)
    })
    setMappings({})
  }

  function onOpenOsDir(osDir: OsDir) {
    post('OpenOsDir', osDir)
  }

  function onQueueRecord() {
    const value = !isQueued
    setIsQueued(value)
    post('QueueRecord')
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
    setControlsLastSaved(controls)
  }

  function onSetCurrentlyMapping(name: string) {
    post('CurrentlyMapping', name)
  }

  function onDeleteSnapshot(slot: string) {
    setSnapshots(snapshots.filter((s) => s !== slot))
    post('SnapshotDelete', slot)
  }

  function onLoadSnapshot(slot: string) {
    post('SnapshotRecall', slot)
    setTransitionInProgress(true)
  }

  function onSaveSnapshot(slot: string) {
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
        isEncoding={isEncoding}
        isQueued={isQueued}
        isRecording={isRecording}
        paused={paused}
        perfMode={perfMode}
        showExclusions={showExclusions}
        showSnapshots={showSnapshots}
        sketchName={sketchName}
        sketchNames={sketchNames}
        tapTempoEnabled={tapTempoEnabled}
        transitionTime={transitionTime}
        view={view}
        onAdvance={onAdvance}
        onCaptureFrame={onCaptureFrame}
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
        onToggleExclusions={() => {
          setShowExclusions(!showExclusions)
        }}
        onTogglePlay={onTogglePlay}
        onToggleSnapshots={() => {
          setShowSnapshots(!showSnapshots)
        }}
      />
      <main>
        {view === View.Settings ? (
          <Settings
            audioDevice={audioDevice}
            audioDevices={audioDevices}
            hrcc={hrcc}
            imagesDir={imagesDir}
            mappings={mappings}
            mappingsEnabled={mappingsEnabled}
            midiClockPort={midiClockPort}
            midiInputPort={midiInputPort}
            midiInputPorts={midiInputPorts}
            midiOutputPort={midiOutputPort}
            midiOutputPorts={midiOutputPorts}
            oscPort={oscPort}
            sliderNames={getSliderNames()}
            userDataDir={userDataDir}
            videosDir={videosDir}
            onChangeAudioDevice={onChangeAudioDevice}
            onChangeFolder={onChangeFolder}
            onChangeHrcc={onChangeHrcc}
            onChangeMappingsEnabled={onChangeMappingsEnabled}
            onChangeMidiClockPort={onChangeMidiClockPort}
            onChangeMidiInputPort={onChangeMidiInputPort}
            onChangeMidiOutputPort={onChangeMidiOutputPort}
            onChangeOscPort={onChangeOscPort}
            onClickSend={onClickSendMidi}
            onDeleteMappings={onDeleteMappings}
            onOpenOsDir={onOpenOsDir}
            onRemoveMapping={onRemoveMapping}
            onSetCurrentlyMapping={onSetCurrentlyMapping}
          />
        ) : (
          <Controls
            bypassed={bypassed}
            controls={controls}
            exclusions={exclusions}
            mappings={mappings}
            mappingsEnabled={mappingsEnabled}
            showExclusions={showExclusions}
            showSnapshots={showSnapshots}
            singleTransitionControlName={singleTransitionControlName}
            transitionInProgress={transitionInProgress}
            onChange={onChangeControl}
            onClickRandomize={onClickRandomizeSingleControl}
            onClickRevert={onClickRevert}
            onToggleExclusion={onToggleExclusion}
            snapshots={snapshots}
            onDeleteSnapshot={onDeleteSnapshot}
            onLoadSnapshot={onLoadSnapshot}
            onSaveSnapshot={onSaveSnapshot}
          />
        )}
      </main>
      <footer>
        <Console alertText={alertText} />
      </footer>
    </div>
  )
}
