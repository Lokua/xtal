import { Mappings, noop, OsDir, UserDir } from './types'
import Checkbox from './Checkbox'
import MapMode from './MapMode'
import OscPortInput from './OscPortInput'
import Select from './Select'
import IconButton from './IconButton'
import { FontSizeChoice, useLocalSettings } from './LocalSettings'

type SizePreset = 'Default' | 'Large' | 'Largest'

function toSizePreset(fontSize: FontSizeChoice) {
  return {
    16: 'Default',
    17: 'Large',
    18: 'Largest',
  }[fontSize] as SizePreset
}

function fromSizePreset(size: SizePreset) {
  return {
    Default: 16,
    Large: 17,
    Largest: 18,
  }[size] as FontSizeChoice
}

type Props = {
  audioDevice: string
  audioDevices: string[]
  hrcc: boolean
  imagesDir: string
  mappings: Mappings
  mappingsEnabled: boolean
  midiClockPort: string
  midiInputPort: string
  midiInputPorts: string[]
  midiOutputPort: string
  midiOutputPorts: string[]
  oscPort: number
  sliderNames: string[]
  userDataDir: string
  videosDir: string
  onChangeAudioDevice: (name: string) => void
  onChangeFolder: (kind: UserDir) => void
  onChangeHrcc: noop
  onChangeMappingsEnabled: () => void
  onChangeMidiClockPort: (port: string) => void
  onChangeMidiInputPort: (port: string) => void
  onChangeMidiOutputPort: (port: string) => void
  onChangeOscPort: (port: number) => void
  onClickSend: () => void
  onDeleteMappings: () => void
  onOpenOsDir: (osDir: OsDir) => void
  onRemoveMapping: (name: string) => void
  onSetCurrentlyMapping: (name: string) => void
}

export default function Settings({
  audioDevice,
  audioDevices,
  hrcc,
  imagesDir,
  mappings,
  mappingsEnabled,
  midiClockPort,
  midiInputPort,
  midiInputPorts,
  midiOutputPort,
  midiOutputPorts,
  oscPort,
  sliderNames,
  userDataDir,
  videosDir,
  onChangeAudioDevice,
  onChangeFolder,
  onChangeHrcc,
  onChangeMappingsEnabled,
  onChangeMidiClockPort,
  onChangeMidiInputPort,
  onChangeMidiOutputPort,
  onChangeOscPort,
  onClickSend,
  onDeleteMappings,
  onOpenOsDir,
  onRemoveMapping,
  onSetCurrentlyMapping,
}: Props) {
  const { localSettings, updateLocalSettings } = useLocalSettings()

  return (
    <div id="settings">
      <section>
        <h2>Appearance</h2>
        <fieldset>
          <Select
            id="size"
            value={String(toSizePreset(localSettings.fontSize))}
            options={['Default', 'Large', 'Largest']}
            onChange={(size) =>
              updateLocalSettings({
                fontSize: fromSizePreset(size as SizePreset),
              })
            }
          />
          <label htmlFor="size">Size</label>
        </fieldset>

        <h2>Storage</h2>
        <fieldset
          data-help-id="UserDataDir"
          className="folder-option"
          onClick={() => {
            onChangeFolder(UserDir.UserData)
          }}
        >
          <label id="sketch-data-folder">Data</label>
          <div aria-labelledby="user-data-folder">
            <IconButton name="Folder" />
            <span>{userDataDir}</span>
          </div>
        </fieldset>
        <fieldset
          data-help-id="ImagesDir"
          className="folder-option"
          onClick={() => {
            onChangeFolder(UserDir.Images)
          }}
        >
          <label id="images-folder">Images</label>
          <div aria-labelledby="images-folder-folder">
            <IconButton id="images-folder" name="Folder" />
            <span>{imagesDir}</span>
          </div>
        </fieldset>
        <fieldset
          data-help-id="VideosDir"
          className="folder-option"
          onClick={() => {
            onChangeFolder(UserDir.Videos)
          }}
        >
          <label id="videos-folder">Videos</label>
          <div aria-labelledby="videos-folder">
            <IconButton id="videos-folder" name="Folder" />
            <span>{videosDir}</span>
          </div>
        </fieldset>
        <aside>
          <button
            onClick={() => {
              onOpenOsDir(OsDir.Cache)
            }}
          >
            Open cache dir
          </button>
          <button
            onClick={() => {
              onOpenOsDir(OsDir.Config)
            }}
          >
            Open config dir
          </button>
        </aside>

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

      <section id="mappings-section" data-help-id="Mappings">
        {sliderNames.length > 0 ? (
          <>
            <MapMode
              mappings={mappings}
              mappingsEnabled={mappingsEnabled}
              sliderNames={sliderNames}
              onDeleteMappings={onDeleteMappings}
              onChangeMappingsEnabled={onChangeMappingsEnabled}
              onRemoveMapping={onRemoveMapping}
              onSetCurrentlyMapping={onSetCurrentlyMapping}
            />
          </>
        ) : (
          <div className="empty-message-container">
            <em>
              <small>
                MIDI Mappings are unavailable to sketches without Slider
                controls
              </small>
            </em>
          </div>
        )}
      </section>
    </div>
  )
}
