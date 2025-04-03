import MapMode from './MapMode'
import Select from './Select'
import { Mappings, noop } from './types'

type Props = {
  audioDevice: string
  audioDevices: string[]
  hrcc: boolean
  mappings: Mappings
  mappingsEnabled: boolean
  midiClockPort: string
  midiInputPort: string
  midiInputPorts: string[]
  midiOutputPort: string
  midiOutputPorts: string[]
  sliderNames: string[]
  onChangeAudioDevice: (name: string) => void
  onChangeHrcc: noop
  onChangeMappingsEnabled: () => void
  onChangeMidiClockPort: (port: string) => void
  onChangeMidiInputPort: (port: string) => void
  onChangeMidiOutputPort: (port: string) => void
  onClickSend: () => void
  onRemoveMapping: (name: string) => void
  onSetCurrentlyMapping: (name: string) => void
}

export default function Settings({
  audioDevice,
  audioDevices,
  hrcc,
  mappings,
  mappingsEnabled,
  midiClockPort,
  midiInputPort,
  midiInputPorts,
  midiOutputPort,
  midiOutputPorts,
  sliderNames,
  onChangeAudioDevice,
  onChangeHrcc,
  onChangeMappingsEnabled,
  onChangeMidiClockPort,
  onChangeMidiInputPort,
  onChangeMidiOutputPort,
  onClickSend,
  onRemoveMapping,
  onSetCurrentlyMapping,
}: Props) {
  return (
    <div id="settings">
      <section>
        <h2>MIDI</h2>
        <button
          title="Sends the state of all CCs to the MIDI output port"
          onClick={onClickSend}
        >
          Send
        </button>
        <fieldset>
          <Select
            id="clock-port"
            value={midiClockPort}
            options={midiInputPorts}
            onChange={onChangeMidiClockPort}
          />
          <label htmlFor="clock-port">Clock Port</label>
        </fieldset>
        <fieldset>
          <Select
            id="input-port"
            value={midiInputPort}
            options={midiInputPorts}
            onChange={onChangeMidiInputPort}
          />
          <label htmlFor="input-port">Input Port</label>
        </fieldset>
        <fieldset>
          <Select
            id="output-port"
            value={midiOutputPort}
            options={midiOutputPorts}
            onChange={onChangeMidiOutputPort}
          />
          <label htmlFor="output-port">Output Port</label>
        </fieldset>
        <fieldset title="Enable high resolution (14bit) MIDI for controls 0-31">
          <input
            id="hrcc"
            type="checkbox"
            checked={hrcc}
            onChange={onChangeHrcc}
          />
          <label htmlFor="hrcc">HRCC</label>
        </fieldset>
        <h2>Audio</h2>
        <fieldset>
          <Select
            id="audio-device"
            value={audioDevice}
            options={audioDevices}
            onChange={onChangeAudioDevice}
          />
          <label htmlFor="audio-device">Device</label>
        </fieldset>
      </section>
      <section>
        {sliderNames.length && (
          <>
            <h2>MIDI Mappings</h2>
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
            <MapMode
              mappings={mappings}
              sliderNames={sliderNames}
              onRemoveMapping={onRemoveMapping}
              onSetCurrentlyMapping={onSetCurrentlyMapping}
            />
          </>
        )}
      </section>
    </div>
  )
}
