import Select from './Select.tsx'
import Separator, { VerticalSeparator } from './Separator.tsx'
import MapMode from './MapMode.tsx'
import { Mappings, noop } from './types.ts'

type Props = {
  clockPort: string
  hrcc: boolean
  inputPort: string
  inputPorts: string[]
  outputPort: string
  outputPorts: string[]
  mappingsEnabled: boolean
  mappings: Mappings
  sliderNames: string[]
  onChangeClockPort: (port: string) => void
  onChangeHrcc: noop
  onChangeInputPort: (port: string) => void
  onChangeOutputPort: (port: string) => void
  onChangeMappingsEnabled: () => void
  onClickSend: () => void
  onRemoveMapping: (name: string) => void
  onSetCurrentlyMapping: (name: string) => void
}

const VSep = () => <VerticalSeparator style={{ margin: '0 8px' }} />

export default function Midi({
  clockPort,
  hrcc,
  inputPort,
  inputPorts,
  outputPort,
  outputPorts,
  mappingsEnabled,
  mappings,
  sliderNames,
  onChangeClockPort,
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
      <header>
        <section>
          <fieldset>
            <Select
              id="clockPort"
              value={clockPort}
              options={inputPorts}
              onChange={(e: React.ChangeEvent<HTMLSelectElement>) => {
                onChangeClockPort(e.currentTarget.value)
              }}
            />
            <label htmlFor="clockPort">Clock Port</label>
          </fieldset>
          <fieldset>
            <Select
              id="inputPort"
              value={inputPort}
              options={inputPorts}
              onChange={(e: React.ChangeEvent<HTMLSelectElement>) => {
                onChangeInputPort(e.currentTarget.value)
              }}
            />
            <label htmlFor="inputPorts">Input Port</label>
          </fieldset>
          <fieldset>
            <Select
              id="outputPort"
              value={outputPort}
              options={outputPorts}
              onChange={(e: React.ChangeEvent<HTMLSelectElement>) => {
                onChangeOutputPort(e.currentTarget.value)
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
          {/* unimplemented for the time being */}
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
          <VSep />
          <button
            title="Sends the state of all CCs to the MIDI output port"
            onClick={onClickSend}
          >
            Send (Resync)
          </button>
        </section>
      </header>
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
