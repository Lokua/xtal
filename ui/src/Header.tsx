import clsx from 'clsx/lite'

import type { noop } from './types'
import { View } from './types'

import Select from './Select'
import Separator, { VerticalSeparator } from './Separator'
import IconButton from './IconButton'

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
  showExclusions: boolean
  showSnapshots: boolean
  sketchName: string
  sketchNames: string[]
  tapTempoEnabled: boolean
  transitionTime: TransitionTime
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
  onReload: noop
  onReset: noop
  onSave: noop
  onSwitchSketch: (sketchName: string) => void
  onToggleExclusions: noop
  onTogglePlay: noop
  onToggleSnapshots: noop
}

export default function Header({
  bpm,
  fps,
  isEncoding,
  isQueued,
  isRecording,
  paused,
  perfMode,
  showExclusions,
  showSnapshots,
  sketchName,
  sketchNames,
  tapTempoEnabled,
  transitionTime,
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
  onReload,
  onReset,
  onSave,
  onSwitchSketch,
  onToggleExclusions,
  onTogglePlay,
  onToggleSnapshots,
}: HeaderProps) {
  return (
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
          value={sketchName}
          options={sketchNames}
          onChange={onSwitchSketch}
          style={{ maxWidth: '192px' }}
        />

        <IconButton data-help-id="Reload" name="Reload" onClick={onReload} />

        <IconButton
          data-help-id="Perf"
          name="Perf"
          isToggle
          on={perfMode}
          onClick={onChangePerfMode}
        />

        <VerticalSeparator />

        <div data-help-id="Bpm" className="meter">
          BPM: <span className="meter-value">{bpm.toFixed(1)}</span>
        </div>
        <IconButton
          data-help-id="Tap"
          name="Tap"
          isToggle
          on={tapTempoEnabled}
          onClick={onChangeTapTempoEnabled}
        />

        <VerticalSeparator />

        <IconButton
          data-help-id="Exclusions"
          title="Exclusions Mode"
          name="Exclusions"
          isToggle
          on={showExclusions}
          onClick={onToggleExclusions}
        />

        <IconButton
          data-help-id="Random"
          name="Random"
          onClick={onClickRandomize}
        />

        <IconButton
          data-help-id="Snapshots"
          name="Snapshots"
          isToggle
          on={showSnapshots}
          onClick={onToggleSnapshots}
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
  )
}
