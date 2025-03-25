import React, { useEffect, useState } from 'react'
import { post } from './util.mjs'
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
      console.log('[app] sub event:', event, 'data:', data)
      switch (event) {
        case 'Init': {
          setIsLightTheme(data.isLightTheme)
          setSketchName(data.sketchName)
          setSketchNames(data.sketchNames)
          setMidiInputPort(data.midiInputPort)
          setMidiOutputPort(data.midiOutputPort)
          const getPort = ([, port]) => port
          setMidiInputPorts(data.midiInputPorts.map(getPort))
          setMidiOutputPorts(data.midiOutputPorts.map(getPort))
          break
        }
        case 'Alert': {
          setAlertText(data)
          break
        }
        case 'LoadSketch': {
          setSketchName(data.sketchName)
          setControls(data.controls)
          setTapTempoEnabled(data.tapTempoEnabled)
          setBpm(data.bpm)
          setFps(data.fps)
          setPaused(data.paused)
          break
        }
        case 'Record': {
          setIsRecording(true)
          setIsQueued(false)
          break
        }
        case 'SetIsEncoding': {
          setIsEncoding(data)
          if (data) {
            setIsQueued(false)
            setIsRecording(false)
          }
          break
        }
        default: {
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
    document.body.classList.add(isLightTheme ? 'light' : 'dark')
    document.body.classList.remove(isLightTheme ? 'dark' : 'light')
  }, [isLightTheme])

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
        onAdvance={() => {
          post('Advance')
        }}
        onCaptureFrame={() => {
          post('CaptureFrame')
        }}
        onChangePerfMode={() => {
          const value = !perfMode
          setPerfMode(value)
          post('SetPerfMode', value)
          setAlertText(
            value
              ? 'When `Perf` is enabled, the sketch window will not be resized \
            when switching sketches.'
              : '',
          )
        }}
        onChangeTapTempoEnabled={() => {
          const value = !tapTempoEnabled
          setTapTempoEnabled(value)
          post('SetTapTempoEnabled', value)
          setAlertText(
            value
              ? 'Tap `Space` key to set BPM'
              : 'Sketch BPM has been restored',
          )
        }}
        onChangeTransitionTime={() => {
          post('SetTransitionTime')
        }}
        onClearBuffer={() => {
          post('ClearBuffer')
        }}
        onReset={() => {
          post('Reset')
        }}
        onQueueRecord={() => {
          const value = !isQueued
          setIsQueued(value)
          post('QueueRecord')
          setAlertText(
            value ? 'Recording queued. Awaiting MIDI start message.' : '',
          )
        }}
        onRecord={() => {
          if (isRecording) {
            setIsRecording(false)
            post('StopRecording')
          } else {
            setIsRecording(true)
            post('Record')
          }
        }}
        onSave={() => {
          post('Save')
        }}
        onTogglePlay={() => {
          const value = !paused
          setPaused(value)
          post('SetPaused', value)
        }}
        onViewMidi={() => {
          setView(view === 'midi' ? 'controls' : 'midi')
        }}
        onSwitchSketch={(sketchName) => {
          post('SwitchSketch', sketchName)
        }}
      />
      <main>
        {view === 'midi' ? (
          <Midi
            hrcc={hrcc}
            inputPort={midiInputPort}
            inputPorts={midiInputPorts}
            outputPort={midiOutputPort}
            outputPorts={midiOutputPorts}
            onChangeHrcc={() => {
              const value = !hrcc
              setHrcc(value)
              post('SetHrcc', value)
              setAlertText(
                value
                  ? 'Expecting 14bit MIDI on channels 0-31'
                  : 'expecting standard 7bit MIDI messages for all CCs',
              )
            }}
            onChangeInputPort={(port) => {
              setMidiInputPort(port)
              setAlertText('changing ports at runtime is not yet supported')
            }}
            onChangeOutputPort={(port) => {
              setMidiOutputPort(port)
              setAlertText('changing ports at runtime is not yet supported')
            }}
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
