import React, { useEffect, useState } from 'react'

import { post } from './util.mjs'
import Select from './Select.jsx'

export default function App() {
  const [view, setView] = useState('controls')
  const [isLightTheme, setIsLightTheme] = useState(true)
  const [sketchName, setSketchName] = useState('')
  const [sketchNames, setSketchNames] = useState([])
  const [midiInputPort, setMidiInputPort] = useState('')
  const [midiInputPorts, setMidiInputPorts] = useState([])
  const [midiOutputPort, setMidiOutputPort] = useState('')
  const [midiOutputPorts, setMidiOutputPorts] = useState([])

  useEffect(() => {
    let unsubscribe = window.latticeEvents.subscribe((e) => {
      switch (e.event) {
        case 'init': {
          console.log(e.data)
          const toIndexAndPort = ([index, port]) => `${index} - ${port}`
          setIsLightTheme(e.data.init.isLightTheme)
          setSketchName(e.data.init.sketchName)
          setSketchNames(e.data.init.sketchNames)
          setMidiInputPort(e.data.init.midiInputPort)
          setMidiOutputPort(e.data.init.midiOutputPort)
          setMidiInputPorts(e.data.init.midiInputPorts.map(toIndexAndPort))
          setMidiOutputPorts(e.data.init.midiOutputPorts.map(toIndexAndPort))
          break
        }
        default: {
          break
        }
      }
    })

    // Tell parent we're ready to receive the `init` event
    post('ready')

    return () => {
      unsubscribe()
    }
  }, [])

  return (
    <div id="app" data-theme="dark">
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
            post('ready')
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
          <div>TODO: controls</div>
        )}
      </main>
    </div>
  )
}
