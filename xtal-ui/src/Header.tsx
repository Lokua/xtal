import type { KeyboardEvent as ReactKeyboardEvent } from 'react'

import NumberBox from '@lokua/number-box'
import CaretDown from
  '@material-symbols/svg-400/outlined/keyboard_arrow_down.svg?react'
import CaretUp from
  '@material-symbols/svg-400/outlined/keyboard_arrow_up.svg?react'
import clsx from 'clsx/lite'

import type { noop } from './types'
import { View } from './types'

import Select from './Select'
import Separator, { VerticalSeparator } from './Separator'
import IconButton from './IconButton'

const transitionTimes = [
  32, 24, 16, 12, 8, 6, 4, 3, 2, 1.5, 1, 0.75, 0.5, 0.25, 0.0,
]

// TODO: Move input key filtering and arrow stepping into @lokua/number-box.
const bpmInputControlKeys = new Set([
  'ArrowDown',
  'ArrowLeft',
  'ArrowRight',
  'ArrowUp',
  'Backspace',
  'Delete',
  'End',
  'Enter',
  'Escape',
  'Home',
  'Tab',
])

type TransitionTime = (typeof transitionTimes)[number]
type OptionGroup = {
  label: string
  options: string[]
}

type HeaderProps = {
  bpm: number
  fps: number
  isEncoding: boolean
  isQueued: boolean
  isRecording: boolean
  monitorPreviewEnabled: boolean
  paused: boolean
  perfMode: boolean
  showExclusions: boolean
  showSnapshots: boolean
  sketchName: string
  sketchOptionGroups: OptionGroup[]
  tapTempoEnabled: boolean
  transitionTime: TransitionTime
  view: View
  onAdvance: noop
  onCaptureFrame: noop
  onChangeBpm: (bpm: number) => void
  onChangeMonitorPreview: noop
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

function isValidBpmInputKey(event: ReactKeyboardEvent<HTMLInputElement>) {
  return (
    event.metaKey ||
    event.ctrlKey ||
    event.altKey ||
    bpmInputControlKeys.has(event.key) ||
    /^[0-9.]$/.test(event.key)
  )
}

export default function Header({
  bpm,
  fps,
  isEncoding,
  isQueued,
  isRecording,
  monitorPreviewEnabled,
  paused,
  perfMode,
  showExclusions,
  showSnapshots,
  sketchName,
  sketchOptionGroups,
  tapTempoEnabled,
  transitionTime,
  view,
  onAdvance,
  onCaptureFrame,
  onChangeBpm,
  onChangeMonitorPreview,
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
  function changeBpmBy(amount: number) {
    onChangeBpm(Math.min(999, Math.max(1, bpm + amount)))
  }

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
            isQueued && !isRecording && 'queued',
          )}
          isToggle
          onClick={onRecord}
        />

        <VerticalSeparator />

        <div data-help-id="Fps" className="meter">
          FPS: <span className="meter-value">{fps.toFixed(1)}</span>
        </div>

        <VerticalSeparator />

        <IconButton
          data-help-id="Monitor"
          name="Monitor"
          isToggle
          on={monitorPreviewEnabled}
          onClick={onChangeMonitorPreview}
        />
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
          optionGroups={sketchOptionGroups}
          onChange={onSwitchSketch}
          onKeyDown={(event) => {
            // Printable keys are one character; navigation keys such as
            // ArrowDown and Enter retain their native select behavior.
            if (event.key.length === 1) {
              event.preventDefault()
            }
          }}
          style={{ maxWidth: '164px' }}
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

        <fieldset data-help-id="Bpm" className="bpm-control">
          <label htmlFor="bpm">BPM:</label>
          <NumberBox
            id="bpm"
            className="number-box"
            value={bpm}
            min={1}
            max={999}
            step={0.1}
            readOnly={!tapTempoEnabled}
            disabled={!tapTempoEnabled}
            onKeyDown={(event) => {
              if (
                (event.metaKey || event.ctrlKey) &&
                event.key.toLowerCase() === 'a'
              ) {
                event.preventDefault()
                event.stopPropagation()
                event.currentTarget.select()
                return
              }

              if (!isValidBpmInputKey(event)) {
                event.preventDefault()
                return
              }

              if (event.key === 'Enter') {
                event.currentTarget.blur()
                return
              }

              if (!tapTempoEnabled) {
                return
              }

              if (event.key === 'ArrowUp') {
                event.preventDefault()
                changeBpmBy(1)
              } else if (event.key === 'ArrowDown') {
                event.preventDefault()
                changeBpmBy(-1)
              }
            }}
            onChange={onChangeBpm}
          />
          <div className="bpm-stepper">
            <button
              type="button"
              aria-label="Increase BPM"
              disabled={!tapTempoEnabled}
              onMouseDown={(event) => {
                event.preventDefault()
              }}
              onClick={() => {
                changeBpmBy(1)
              }}
            >
              <CaretUp />
            </button>
            <button
              type="button"
              aria-label="Decrease BPM"
              disabled={!tapTempoEnabled}
              onMouseDown={(event) => {
                event.preventDefault()
              }}
              onClick={() => {
                changeBpmBy(-1)
              }}
            >
              <CaretDown />
            </button>
          </div>
        </fieldset>
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
            style={{ width: '52px' }}
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
