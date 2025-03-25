import React from 'react'
import Select from './Select.jsx'
import { VerticalSeparator } from './Separator.jsx'

export default function Midi({
  hrcc,
  inputPort,
  inputPorts,
  outputPort,
  outputPorts,
  onChangeHrcc,
  onChangeInputPort,
  onChangeOutputPort,
  onClickSend,
}) {
  return (
    <div className="midi">
      <section>
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
        <VerticalSeparator style={{ margin: '0 8px' }} />
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
      </section>
      <section>
        <fieldset>
          <input
            id="hrcc"
            type="checkbox"
            checked={hrcc}
            onChange={onChangeHrcc}
          />
          <label htmlFor="hrcc">Hi-Res</label>
        </fieldset>
        <VerticalSeparator style={{ margin: '0 8px' }} />
        <button onClick={onClickSend}>Send</button>
      </section>
    </div>
  )
}
