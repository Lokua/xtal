import React from 'react'
import Select from './Select.jsx'

export default function Midi({
  hrcc,
  inputPort,
  inputPorts,
  outputPort,
  outputPorts,
  onChangeHrcc,
  onChangeInputPort,
  onChangeOutputPort,
}) {
  return (
    <>
      <fieldset>
        <Select
          id="inputPort"
          value={inputPort}
          options={inputPorts}
          onChange={(e) => {
            onChangeInputPort(e.target.value)
          }}
        />
        <label htmlFor="inputPorts">Input Port</label>
      </fieldset>
      <fieldset>
        <Select
          id="outputPort"
          value={outputPort}
          options={outputPorts}
          onChange={(e) => {
            onChangeOutputPort(e.target.value)
          }}
        />
        <label htmlFor="outputPort">Output Port</label>
      </fieldset>
      <fieldset>
        <input
          id="hrcc"
          type="checkbox"
          checked={hrcc}
          onChange={onChangeHrcc}
        />
        <label htmlFor="hrcc">Hi-Res</label>
      </fieldset>
    </>
  )
}
