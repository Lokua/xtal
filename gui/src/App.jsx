import React, { useEffect, useState } from 'react'
import { match, post } from './util.mjs'
import Header from './Header.jsx'
import Controls from './Controls.jsx'
import Midi from './Midi.jsx'

export default function App() {
  const [alertText, setAlertText] = useState('')
  const [bpm, setBpm] = useState(134)
  const [controls, setControls] = useState([])
  const [fps, setFps] = useState(60)
  const [hrcc, setHrcc] = useState(false)
  const [isEncoding, setIsEncoding] = useState(false)
  const [isLightTheme, setIsLightTheme] = useState(true)
  const [isQueued, setIsQueued] = useState(false)
  const [isRecording, setIsRecording] = useState(false)
  const [midiInputPort, setMidiInputPort] = useState('')
  const [midiInputPorts, setMidiInputPorts] = useState([])
  const [midiOutputPort, setMidiOutputPort] = useState('')
  const [midiOutputPorts, setMidiOutputPorts] = useState([])
  const [paused, setPaused] = useState(false)
  const [perfMode, setPerfMode] = useState(false)
  const [sketchName, setSketchName] = useState('')
  const [sketchNames, setSketchNames] = useState([])
  const [tapTempoEnabled, setTapTempoEnabled] = useState(false)
  const [view, setView] = useState('controls')

  useEffect(() => {
    const unsubscribe = window.latticeEvents.subscribe((event, data) => {
      if (event !== 'AverageFps') {
        console.debug('[app - sub event]:', event, 'data:', data)
      }

      match(event, {
        Alert() {
          setAlertText(data)
        },
        AverageFps() {
          setFps(data.toFixed(1))
        },
        Bpm() {
          setBpm(data.toFixed(1))
        },
        Init() {
          setIsLightTheme(data.isLightTheme)
          setSketchName(data.sketchName)
          setSketchNames(data.sketchNames)
          setMidiInputPort(data.midiInputPort)
          setMidiOutputPort(data.midiOutputPort)
          const getPort = ([, port]) => port
          setMidiInputPorts(data.midiInputPorts.map(getPort))
          setMidiOutputPorts(data.midiOutputPorts.map(getPort))
        },
        HubPopulated() {
          setControls(data)
        },
        LoadSketch() {
          setSketchName(data.sketchName)
          setControls(data.controls)
          setTapTempoEnabled(data.tapTempoEnabled)
          setBpm(data.bpm)
          setFps(data.fps)
          setPaused(data.paused)
        },
        Record() {
          setIsRecording(true)
          setIsQueued(false)
        },
        SetIsEncoding() {
          setIsEncoding(data)
          if (data) {
            setIsQueued(false)
            setIsRecording(false)
          }
        },
        SnapshotEnded() {
          setControls(data)
        },
      })
    })

    post('Ready')

    return () => {
      unsubscribe()
    }
  }, [])

  useEffect(() => {
    document.addEventListener('keydown', onKeyDown)

    function onKeyDown(e) {
      console.debug('[onKeyDown] e:', e)

      if (e.code.startsWith('Digit')) {
        if (e.metaKey) {
          post('SnapshotRecall', e.key)
        } else if (e.shiftKey) {
          // `key` is no longer "1" or "2" but "!" and "@" at this point
          const actualKey = e.code.slice('Digit'.length)
          post('SnapshotStore', actualKey)
        }
      }

      match(e.code, {
        KeyA() {
          if (paused) {
            post('Advance')
          }
        },
        KeyF() {
          if (e.metaKey) {
            post('ToggleFullScreen')
          }
        },
        KeyG() {
          if (e.metaKey) {
            post('ToggleGuiFocus')
          }
        },
        KeyM() {
          if (e.shiftKey && e.metaKey) {
            setView(view === 'midi' ? 'controls' : 'midi')
            // TODO: if leaving midi, send mappings
          } else if (e.metaKey) {
            post('ToggleMainFocus')
          }
        },
        KeyS() {
          if (e.metaKey || e.shiftKey) {
            post('Save')
          } else {
            post('CaptureFrame')
          }
        },
        Space() {
          if (tapTempoEnabled) {
            post('Tap')
          }
        },
      })
    }

    return () => {
      document.removeEventListener('keydown', onKeyDown)
    }
  }, [paused, tapTempoEnabled, view])

  useEffect(() => {
    document.body.classList.add(isLightTheme ? 'light' : 'dark')
    document.body.classList.remove(isLightTheme ? 'dark' : 'light')
  }, [isLightTheme])

  function onAdvance() {
    post('Advance')
  }

  function onCaptureFrame() {
    post('CaptureFrame')
  }

  function onChangeControl(type, name, value, controls) {
    setControls(controls)

    const event =
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
    post('SetHrcc', value)
    setAlertText(
      value
        ? 'Expecting 14bit MIDI on channels 0-31'
        : 'Expecting standard 7bit MIDI messages for all CCs',
    )
  }

  function onChangeInputPort(port) {
    setMidiInputPort(port)
    setAlertText('Changing ports at runtime is not yet supported')
  }

  function onChangeOutputPort(port) {
    setMidiOutputPort(port)
    setAlertText('Changing ports at runtime is not yet supported')
  }

  function onChangePerfMode() {
    const value = !perfMode
    setPerfMode(value)
    post('SetPerfMode', value)
    setAlertText(
      value
        ? 'When `Perf` is enabled, the sketch window will not be resized \
        when switching sketches.'
        : '',
    )
  }

  function onChangeTapTempoEnabled() {
    const enabled = !tapTempoEnabled
    setTapTempoEnabled(enabled)
    post('SetTapTempoEnabled', enabled)
    setAlertText(
      enabled ? 'Tap `Space` key to set BPM' : 'Sketch BPM has been restored',
    )
  }

  function onChangeTransitionTime() {
    post('SetTransitionTime')
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
      post('Record')
    }
  }

  function onReset() {
    post('Reset')
  }

  function onSave() {
    post('Save')
  }

  function onSwitchSketch(sketchName) {
    post('SwitchSketch', sketchName)
  }

  function onTogglePlay() {
    const value = !paused
    setPaused(value)
    post('SetPaused', value)
  }

  function onViewMidi() {
    setView(view === 'midi' ? 'controls' : 'midi')
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
        onClearBuffer={onClearBuffer}
        onReset={onReset}
        onQueueRecord={onQueueRecord}
        onRecord={onRecord}
        onSave={onSave}
        onSwitchSketch={onSwitchSketch}
        onTogglePlay={onTogglePlay}
        onViewMidi={onViewMidi}
      />
      <main>
        {view === 'midi' ? (
          <Midi
            hrcc={hrcc}
            inputPort={midiInputPort}
            inputPorts={midiInputPorts}
            outputPort={midiOutputPort}
            outputPorts={midiOutputPorts}
            onChangeHrcc={onChangeHrcc}
            onChangeInputPort={onChangeInputPort}
            onChangeOutputPort={onChangeOutputPort}
            onClickSend={onClickSendMidi}
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
