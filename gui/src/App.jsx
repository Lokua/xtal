import React, { useEffect, useState } from 'react'
import { post } from './util.mjs'
import Select from './Select.jsx'
import Controls from './Controls.jsx'
import Separator, { VerticalSeparator } from './Separator.jsx'
import Header from './Header.jsx'

export default function App() {
  const [view, setView] = useState('controls')
  const [isLightTheme, setIsLightTheme] = useState(true)
  const [sketchName, setSketchName] = useState('')
  const [sketchNames, setSketchNames] = useState([])
  const [midiInputPort, setMidiInputPort] = useState('')
  const [midiInputPorts, setMidiInputPorts] = useState([])
  const [midiOutputPort, setMidiOutputPort] = useState('')
  const [midiOutputPorts, setMidiOutputPorts] = useState([])
  const [controls, setControls] = useState([])
  const [tapTempoEnabled, setTapTempoEnabled] = useState(false)
  const [bpm, setBpm] = useState(134)
  const [fps, setFps] = useState(60)
  const [paused, setPaused] = useState(false)
  const [perfMode, setPerfMode] = useState(false)
  const [isRecording, setIsRecording] = useState(false)
  const [isEncoding, setIsEncoding] = useState(false)
  const [isQueued, setIsQueued] = useState(false)
  const [alertText, setAlertText] = useState('')

  useEffect(() => {
    let unsubscribe = window.latticeEvents.subscribe((event, data) => {
      console.log('[app] sub event:', event, 'data:', data)
      switch (event) {
        case 'Init': {
          setIsLightTheme(data.isLightTheme)
          setSketchName(data.sketchName)
          setSketchNames(data.sketchNames)
          setMidiInputPort(data.midiInputPort)
          setMidiOutputPort(data.midiOutputPort)
          const toIndexAndPort = ([index, port]) => `${index} - ${port}`
          setMidiInputPorts(data.midiInputPorts.map(toIndexAndPort))
          setMidiOutputPorts(data.midiOutputPorts.map(toIndexAndPort))
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
              : ''
          )
        }}
        onChangeTapTempoEnabled={() => {
          const value = !tapTempoEnabled
          setTapTempoEnabled(value)
          post('SetTapTempoEnabled', value)
          setAlertText(
            value
              ? 'Tap `Space` key to set BPM'
              : 'Sketch BPM has been restored'
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
            value ? 'Recording queued. Awaiting MIDI start message.' : ''
          )
        }}
        onRecord={() => {
          post('Record')
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
          //
        }}
        onSwitchSketch={(sketchName) => {
          post('SwitchSketch', sketchName)
        }}
      />
      <main>
        {view === 'midi' ? (
          <>
            <Select
              value={midiInputPort}
              options={midiInputPorts}
              onChange={(e) => {
                setMidiInputPort(e.target.value)
              }}
            />
            <Select
              value={midiOutputPort}
              options={midiOutputPorts}
              onChange={(e) => {
                setMidiOutputPort(e.target.value)
              }}
            />
          </>
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
