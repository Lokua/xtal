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
  randomizationIncludesCheckboxes: boolean
  randomizationIncludesSelects: boolean
  useIcons: boolean
  onChangeAudioDevice: (name: string) => void
  onChangeHrcc: noop
  onChangeMappingsEnabled: () => void
  onChangeMidiClockPort: (port: string) => void
  onChangeMidiInputPort: (port: string) => void
  onChangeMidiOutputPort: (port: string) => void
  onChangeOscPort: (port: number) => void
  onChangeRandomizationIncludesCheckboxes: () => void
  onChangeRandomizationIncludesSelects: () => void
  onChangeUseIcons: (useIcons: boolean) => void
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
  randomizationIncludesCheckboxes,
  randomizationIncludesSelects,
  useIcons,
  onChangeAudioDevice,
  onChangeHrcc,
  onChangeMappingsEnabled,
  onChangeMidiClockPort,
  onChangeMidiInputPort,
  onChangeMidiOutputPort,
  onChangeOscPort,
  onChangeRandomizationIncludesCheckboxes,
  onChangeRandomizationIncludesSelects,
  onChangeUseIcons,
  onClickSend,
  onRemoveMapping,
  onSetCurrentlyMapping,
}: Props) {
  return (
    <div id="settings">
      <section>
        <h2>Appearance</h2>
        <fieldset>
          <Checkbox
            id="use-icons"
            type="checkbox"
            checked={useIcons}
            onChange={() => {
              onChangeUseIcons(!useIcons)
            }}
          />
          <label htmlFor="use-icons">Use Icons</label>
        </fieldset>

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
          <Checkbox
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

        <h2>OSC</h2>
        <OscPortInput port={oscPort} onChange={onChangeOscPort} />

        <h2>Randomization</h2>
        <fieldset>
          <Checkbox
            id="include-checkboxes"
            type="checkbox"
            checked={randomizationIncludesCheckboxes}
            onChange={onChangeRandomizationIncludesCheckboxes}
          />
          <label htmlFor="include-checkboxes">Include Checkboxes</label>
        </fieldset>
        <fieldset>
          <Checkbox
            id="include-selects"
            type="checkbox"
            checked={randomizationIncludesSelects}
            onChange={onChangeRandomizationIncludesSelects}
          />
          <label htmlFor="include-selects">Include Selects</label>
        </fieldset>
      </section>

      <section>
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
