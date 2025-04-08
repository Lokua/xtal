import { Mappings, noop } from './types'
import Checkbox from './Checkbox'
import MapMode from './MapMode'
import OscPortInput from './OscPortInput'
import Select from './Select'

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
  oscPort: number
  sliderNames: string[]
  onChangeAudioDevice: (name: string) => void
  onChangeHrcc: noop
  onChangeMappingsEnabled: () => void
  onChangeMidiClockPort: (port: string) => void
  onChangeMidiInputPort: (port: string) => void
  onChangeMidiOutputPort: (port: string) => void
  onChangeOscPort: (port: number) => void
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
  oscPort,
  sliderNames,
  onChangeAudioDevice,
  onChangeHrcc,
  onChangeMappingsEnabled,
  onChangeMidiClockPort,
  onChangeMidiInputPort,
  onChangeMidiOutputPort,
  onChangeOscPort,
  onClickSend,
  onRemoveMapping,
  onSetCurrentlyMapping,
}: Props) {
  return (
    <div id="settings">
      <section>
        <h2>MIDI</h2>
        <button data-help-id="Send" onClick={onClickSend}>
          Send
        </button>
        <fieldset data-help-id="MidiClockPort">
          <Select
            id="clock-port"
            value={midiClockPort}
            options={midiInputPorts}
            onChange={onChangeMidiClockPort}
          />
          <label htmlFor="clock-port">Clock Port</label>
        </fieldset>
        <fieldset data-help-id="MidiInputPort">
          <Select
            id="input-port"
            value={midiInputPort}
            options={midiInputPorts}
            onChange={onChangeMidiInputPort}
          />
          <label htmlFor="input-port">Input Port</label>
        </fieldset>
        <fieldset data-help-id="MidiOutputPort">
          <Select
            id="output-port"
            value={midiOutputPort}
            options={midiOutputPorts}
            onChange={onChangeMidiOutputPort}
          />
          <label htmlFor="output-port">Output Port</label>
        </fieldset>
        <fieldset data-help-id="Hrcc">
          <Checkbox
            id="hrcc"
            type="checkbox"
            checked={hrcc}
            onChange={onChangeHrcc}
          />
          <label htmlFor="hrcc">HRCC</label>
        </fieldset>

        <h2>Audio</h2>
        <fieldset data-help-id="Audio">
          <Select
            id="audio-device"
            value={audioDevice}
            options={audioDevices}
            onChange={onChangeAudioDevice}
          />
          <label htmlFor="audio-device">Device</label>
        </fieldset>

        <h2>OSC</h2>
        <OscPortInput
          data-help-id="OscPort"
          port={oscPort}
          onChange={onChangeOscPort}
        />
      </section>

      <section data-help-id="Mappings">
        {sliderNames.length > 0 ? (
          <>
            <h2>MIDI Mappings</h2>
            <fieldset
              title="Enables live overrides of UI sliders via MIDI CCs"
              style={{ display: 'none' }}
            >
              <Checkbox
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
        ) : (
          <div className="empty-message-container">
            <em>
              MIDI Mappings are unavailable to sketches without Slider controls
            </em>
          </div>
        )}
      </section>
    </div>
  )
}
