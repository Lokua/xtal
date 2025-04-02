import type { noop } from './types.js'
import { View } from './types.ts'
import Select from './Select.js'
import Separator, { VerticalSeparator } from './Separator.js'

const transitionTimes = [32, 24, 16, 12, 8, 6, 4, 3, 2, 1.5, 1, 0.75, 5, 0.25]
type TransitionTime = (typeof transitionTimes)[number]

type HeaderProps = {
  bpm: number
  fps: number
  isEncoding: boolean
  isQueued: boolean
  isRecording: boolean
  paused: boolean
  perfMode: boolean
  sketchName: string
  sketchNames: string[]
  tapTempoEnabled: boolean
  view: View
  onAdvance: noop
  onCaptureFrame: noop
  onChangePerfMode: noop
  onChangeTapTempoEnabled: noop
  onChangeTransitionTime: (transitionTime: TransitionTime) => void
  onChangeView: noop
  onClearBuffer: noop
  onQueueRecord: noop
  onRecord: noop
  onReset: noop
  onSave: noop
  onSwitchSketch: (sketchName: string) => void
  onTogglePlay: noop
}

export default function Header({
  bpm,
  fps,
  isEncoding,
  isQueued,
  isRecording,
  paused,
  perfMode,
  sketchName,
  sketchNames,
  tapTempoEnabled,
  view,
  onAdvance,
  onCaptureFrame,
  onChangePerfMode,
  onChangeTapTempoEnabled,
  onChangeTransitionTime,
  onChangeView,
  onClearBuffer,
  onQueueRecord,
  onRecord,
  onReset,
  onSave,
  onSwitchSketch,
  onTogglePlay,
}: HeaderProps) {
  return (
    <header>
      <section>
        <button onClick={onCaptureFrame}>Image</button>
        <VerticalSeparator />
        <button onClick={onTogglePlay}>{paused ? 'Play' : 'Pause'}</button>
        <button disabled={!paused} onClick={onAdvance}>
          Advance
        </button>
        <button onClick={onReset}>Reset</button>
        <VerticalSeparator />
        <button onClick={onClearBuffer}>Clear Buf.</button>
        <VerticalSeparator />
        <button
          className={isQueued ? 'on' : ''}
          disabled={isRecording || isEncoding}
          onClick={onQueueRecord}
        >
          {isQueued ? 'QUEUED' : 'Q Rec.'}
        </button>
        <button
          className={isRecording ? 'record-button on' : 'record-button'}
          disabled={isEncoding}
          onClick={onRecord}
        >
          {isRecording ? 'STOP' : isEncoding ? 'Encoding' : 'Rec.'}
        </button>
        <VerticalSeparator />
        <div className="meter">
          FPS: <span className="meter-value">{fps.toFixed(1)}</span>
        </div>
      </section>
      <Separator style={{ margin: '2px 0' }} />
      <section>
        <Select
          value={sketchName}
          options={sketchNames}
          onChange={(e) => {
            onSwitchSketch(e.currentTarget.value)
          }}
        />
        <fieldset>
          <input
            id="perf"
            type="checkbox"
            checked={perfMode}
            onChange={onChangePerfMode}
          />
          <label htmlFor="perf">Perf.</label>
        </fieldset>
        <VerticalSeparator />
        <div className="meter">
          BPM: <span className="meter-value">{bpm.toFixed(1)}</span>
        </div>
        <fieldset>
          <input
            id="tap"
            type="checkbox"
            checked={tapTempoEnabled}
            onChange={onChangeTapTempoEnabled}
          />
          <label htmlFor="tap">Tap</label>
        </fieldset>
        <VerticalSeparator />
        <Select
          style={{ width: '48px' }}
          value="4"
          options={transitionTimes}
          onChange={(e) => {
            onChangeTransitionTime(parseFloat(e.currentTarget.value))
          }}
        />
        <VerticalSeparator />
        <button onClick={onSave}>Save</button>
        <button
          className={view === View.Midi ? 'on' : ''}
          onClick={onChangeView}
        >
          MIDI
        </button>
      </section>
    </header>
  )
}
