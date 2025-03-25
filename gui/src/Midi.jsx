import React from 'react'
import Select from './Select.jsx'
import Separator, { VerticalSeparator } from './Separator.jsx'
import MapMode from './MapMode.jsx'

export default function Midi({
  hrcc,
  inputPort,
  inputPorts,
  outputPort,
  outputPorts,
  mappingsEnabled,
  mappings,
  sliderNames,
  onChangeHrcc,
  onChangeInputPort,
  onChangeOutputPort,
  onChangeMappingsEnabled,
  onClickSend,
  onRemoveMapping,
  onSetCurrentlyMapping,
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
        <fieldset title="Enables live overrides of UI sliders via MIDI CCs">
          <input
            id="mappings-enabled"
            type="checkbox"
            checked={mappingsEnabled}
            onChange={onChangeMappingsEnabled}
          />
          <label htmlFor="mappings-enabled">Mappings</label>
        </fieldset>
        <VerticalSeparator style={{ margin: '0 8px' }} />
        <button
          title="Sends the state of all CCs to the MIDI output port"
          onClick={onClickSend}
        >
          Send (Resync)
        </button>
      </section>
      <Separator />
      {sliderNames.length && (
        <MapMode
          mappings={mappings}
          sliderNames={sliderNames}
          onRemoveMapping={onRemoveMapping}
          onSetCurrentlyMapping={onSetCurrentlyMapping}
        />
      )}
    </div>
  )
}
