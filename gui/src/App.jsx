import React, { useEffect, useState } from 'react'
import { post } from './util.mjs'
import Select from './Select.jsx'
import Controls from './Controls.jsx'

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

  useEffect(() => {
    let unsubscribe = window.latticeEvents.subscribe((e) => {
      switch (e.event) {
        case 'init': {
          console.log('init:', e.data)
          const payload = e.data.init
          const toIndexAndPort = ([index, port]) => `${index} - ${port}`
          setIsLightTheme(payload.isLightTheme)
          setSketchName(payload.sketchName)
          setSketchNames(payload.sketchNames)
          setMidiInputPort(payload.midiInputPort)
          setMidiOutputPort(payload.midiOutputPort)
          setMidiInputPorts(payload.midiInputPorts.map(toIndexAndPort))
          setMidiOutputPorts(payload.midiOutputPorts.map(toIndexAndPort))
          break
        }
        case 'loadSketch': {
          console.log('loadSketch:', e.data)
          const payload = e.data.loadSketch
          setSketchName(payload.sketchName)
          setControls(payload.controls)
          break
        }
        default: {
          break
        }
      }
    })

    post('ready')

    return () => {
      unsubscribe()
    }
  }, [])

  useEffect(() => {
    document.body.classList.add(isLightTheme ? 'light' : 'dark')
    document.body.classList.remove(isLightTheme ? 'dark' : 'light')
  }, [isLightTheme])

  function onControlChange(type, name, value, controls) {
    setControls(controls)

    const eventName =
      type === 'checkbox'
        ? 'updateControlBool'
        : type === 'slider'
        ? 'updateControlFloat'
        : 'updateControlString'

    post(eventName, {
      [eventName]: {
        name,
        value,
      },
    })
  }

  return (
    <div id="app">
      <header>
        <Select
          value={sketchName}
          options={sketchNames}
          onChange={(e) => {
            setSketchName(e.target.value)
          }}
        />
        <button
          onClick={() => {
            post('reset')
          }}
        >
          Reset
        </button>
        <button
          onClick={() => {
            post('tap')
          }}
        >
          Tap
        </button>
        <button
          onClick={() => {
            post('Debug')
          }}
        >
          Ready
        </button>
      </header>
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
          <Controls controls={controls} onChange={onControlChange} />
        )}
      </main>
    </div>
  )
}
