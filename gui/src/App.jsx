import React, { useEffect, useState } from 'react'
import { post } from './util.mjs'
import Select from './Select.jsx'
import Controls from './Controls.jsx'
import Separator, { VerticalSeparator } from './Separator.jsx'

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
      console.log('[app] sub e:', e)
      switch (e.event) {
        case 'init': {
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

    post('Ready')

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
      name,
      value,
    })
  }

  return (
    <div id="app">
      <header>
        <section>
          <button
            onClick={() => {
              post('capture')
            }}
          >
            Image
          </button>
          <VerticalSeparator />
          <button
            onClick={() => {
              post('pause')
            }}
          >
            Pause
          </button>
          <button
            disabled
            onClick={() => {
              post('advance')
            }}
          >
            Advance
          </button>
          <button
            onClick={() => {
              post('reset')
            }}
          >
            Reset
          </button>
          <VerticalSeparator />
          <button
            onClick={() => {
              post('clearBuf')
            }}
          >
            Clear Buf.
          </button>
          <VerticalSeparator />
          <button
            onClick={() => {
              post('qRecord')
            }}
          >
            Q Rec.
          </button>
          <button
            onClick={() => {
              post('record')
            }}
          >
            Rec.
          </button>
          <VerticalSeparator />
          <div className="meter">
            FPS: <span className="meter-value">30.0</span>
          </div>
        </section>
        <Separator style={{ margin: '2px 0' }} />
        <section>
          <Select
            value={sketchName}
            options={sketchNames}
            onChange={(e) => {
              setSketchName(e.target.value)
            }}
          />
          <fieldset>
            <input
              id="perf"
              type="checkbox"
              checked={false}
              onChange={() => {}}
            />
            <label htmlFor="perf">Perf.</label>
          </fieldset>
          <VerticalSeparator />
          <div className="meter">
            BPM: <span className="meter-value">134.0</span>
          </div>
          <button
            onClick={() => {
              post('tap')
            }}
          >
            Tap
          </button>
          <VerticalSeparator />
          <Select
            style={{ width: '48px' }}
            value="4"
            options={[32, 24, 16, 12, 8, 6, 4, 3, 2, 1.5, 1, 0.75, 5, 0.25]}
            onChange={() => {}}
          />
          <VerticalSeparator />
          <button
            onClick={() => {
              post('save')
            }}
          >
            Save
          </button>
          <button
            onClick={() => {
              post('viewMidi')
            }}
          >
            MIDI
          </button>
        </section>
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
