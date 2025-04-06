import clsx from 'clsx/lite'

import type { noop } from './types.js'
import { View } from './types.ts'

import Select from './Select.js'
import Separator, { VerticalSeparator } from './Separator.tsx'
import IconButton from './IconButton.tsx'
import { Help } from './Help.tsx'

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
  transitionTime: TransitionTime
  useIcons: boolean
  view: View
  onAdvance: noop
  onCaptureFrame: noop
  onChangePerfMode: noop
  onChangeTapTempoEnabled: noop
  onChangeTransitionTime: (transitionTime: TransitionTime) => void
  onChangeView: noop
  onClearBuffer: noop
  onClickRandomize: noop
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
  transitionTime,
  useIcons,
  view,
  onAdvance,
  onCaptureFrame,
  onChangePerfMode,
  onChangeTapTempoEnabled,
  onChangeTransitionTime,
  onChangeView,
  onClearBuffer,
  onClickRandomize,
  onQueueRecord,
  onRecord,
  onReset,
  onSave,
  onSwitchSketch,
  onTogglePlay,
}: HeaderProps) {
  return useIcons ? (
    <header>
      <section>
        <IconButton
          data-help-id="Play"
          name={paused ? 'Play' : 'Pause'}
          isToggle
          onClick={onTogglePlay}
        />
        <IconButton
          data-help-id="Advance"
          name="Advance"
          disabled={!paused}
          onClick={onAdvance}
        />
        <IconButton data-help-id="Reset" name="Reset" onClick={onReset} />

        <VerticalSeparator />
        <IconButton data-help-id="Clear" name="Clear" onClick={onClearBuffer} />
        <VerticalSeparator />

        <IconButton
          data-help-id="Image"
          name="Image"
          onClick={onCaptureFrame}
        />
        <IconButton
          data-help-id="Queue"
          name={isQueued ? 'Queued' : 'Queue'}
          disabled={isRecording || isEncoding}
          on={isQueued}
          isToggle
          onClick={onQueueRecord}
        />
        <IconButton
          data-help-id="Record"
          name={isRecording ? 'StopRecording' : 'Record'}
          disabled={isEncoding}
          className={clsx(
            isRecording && 'is-recording',
            isEncoding && 'is-encoding',
            isQueued && !isRecording && 'queued'
          )}
          isToggle
          onClick={onRecord}
        />

        <VerticalSeparator />

        <div data-help-id="Fps" className="meter">
          FPS: <span className="meter-value">{fps.toFixed(1)}</span>
        </div>

        <VerticalSeparator />

        <IconButton data-help-id="Save" name="Save" onClick={onSave} />
        <IconButton
          data-help-id="Settings"
          name="Settings"
          on={view === View.Settings}
          isToggle
          onClick={onChangeView}
        />
      </section>

      <Separator style={{ margin: '2px 0' }} />

      <section>
        <Select
          data-help-id="Sketch"
          id="sketch"
          value={sketchName}
          options={sketchNames}
          onChange={onSwitchSketch}
          style={{ maxWidth: '192px' }}
        />

        <IconButton
          data-help-id="Perf"
          name="Perf"
          on={perfMode}
          isToggle
          onClick={onChangePerfMode}
        />

        <VerticalSeparator />

        <div data-help-id="Bpm" className="meter">
          BPM: <span className="meter-value">{bpm.toFixed(1)}</span>
        </div>
        <IconButton
          data-help-id="Tap"
          name="Tap"
          on={tapTempoEnabled}
          isToggle
          onClick={onChangeTapTempoEnabled}
        />

        <VerticalSeparator />

        <IconButton
          data-help-id="Random"
          name="Random"
          onClick={onClickRandomize}
        />

        <fieldset>
          <Select
            data-help-id="TransitionTime"
            id="transition-time"
            style={{ width: '48px' }}
            value={transitionTime.toString()}
            options={transitionTimes}
            onChange={(value) => {
              onChangeTransitionTime(parseFloat(value))
            }}
          />
        </fieldset>
      </section>
    </header>
  ) : (
    <header>
      <section>
        <button data-help-id="Image" onClick={onCaptureFrame}>
          Image
        </button>
        <VerticalSeparator />

        <button data-help-id="Play" onClick={onTogglePlay}>
          {paused ? 'Play' : 'Pause'}
        </button>
        <button data-help-id="Advance" disabled={!paused} onClick={onAdvance}>
          Advance
        </button>
        <button data-help-id="Reset" onClick={onReset}>
          Reset
        </button>
        <VerticalSeparator />

        <button data-help-id="Clear" onClick={onClearBuffer}>
          Clear Buf.
        </button>
        <VerticalSeparator />

        <button
          data-help-id="Queue"
          className={isQueued ? 'on' : ''}
          disabled={isRecording || isEncoding}
          onClick={onQueueRecord}
        >
          {isQueued ? 'QUEUED' : 'Q Rec.'}
        </button>
        <button
          data-help-id="Record"
          className={isRecording ? 'record-button on' : 'record-button'}
          disabled={isEncoding}
          onClick={onRecord}
        >
          {isRecording ? 'STOP' : isEncoding ? 'Encoding' : 'Rec.'}
        </button>
        <VerticalSeparator />

        <div data-help-id="Fps" className="meter">
          FPS: <span className="meter-value">{fps.toFixed(1)}</span>
        </div>
      </section>

      <Separator style={{ margin: '2px 0' }} />

      <section>
        <Select
          data-help-id="Sketch"
          value={sketchName}
          options={sketchNames}
          onChange={onSwitchSketch}
          style={{ width: '128px' }}
        />
        <fieldset data-help-id="Perf">
          <input
            id="perf"
            type="checkbox"
            checked={perfMode}
            onChange={onChangePerfMode}
          />
          <label htmlFor="perf">Perf.</label>
        </fieldset>
        <VerticalSeparator />

        <div data-help-id="Bpm" className="meter">
          BPM: <span className="meter-value">{bpm.toFixed(1)}</span>
        </div>
        <fieldset data-help-id="Tap">
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
          data-help-id="TransitionTime"
          style={{ width: '48px' }}
          value={transitionTime.toString()}
          options={transitionTimes}
          onChange={(value) => {
            onChangeTransitionTime(parseFloat(value))
          }}
        />
        <button data-help-id="Random" onClick={onClickRandomize}>
          ?
        </button>
        <VerticalSeparator />

        <button data-help-id="Save" title={Help.Save} onClick={onSave}>
          Save
        </button>
        <button
          data-help-id="Settings"
          className={view === View.Settings ? 'on' : ''}
          onClick={onChangeView}
        >
          Conf.
        </button>
      </section>
    </header>
  )
}
