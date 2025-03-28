import Select from './Select.tsx'
import Separator, { VerticalSeparator } from './Separator.tsx'
import MapMode from './MapMode.tsx'
import { Mappings, noop } from './types.ts'

type Props = {
  hrcc: boolean
  inputPort: string
  inputPorts: string[]
  outputPort: string
  outputPorts: string[]
  mappingsEnabled: boolean
  mappings: Mappings
  sliderNames: string[]
  onChangeHrcc: noop
  onChangeInputPort: (port: string) => void
  onChangeOutputPort: (port: string) => void
  onChangeMappingsEnabled: () => void
  onClickSend: () => void
  onRemoveMapping: (name: string) => void
  onSetCurrentlyMapping: (name: string) => void
}

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
}: Props) {
  return (
    <div className="midi">
      <section>
        <fieldset>
          <Select
            id="inputPort"
            value={inputPort}
            options={inputPorts}
            onChange={(e: React.ChangeEvent<HTMLSelectElement>) => {
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
            onChange={(e: React.ChangeEvent<HTMLSelectElement>) => {
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
        {/* unimplemented. Leaving uncommented so I don't have to deal with 
            refactoring props and unused warnings */}
        <fieldset
          title="Enables live overrides of UI sliders via MIDI CCs"
          style={{ display: 'none' }}
        >
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
