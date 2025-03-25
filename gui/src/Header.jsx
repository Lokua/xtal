import React from 'react'
import Select from './Select.jsx'
import Separator, { VerticalSeparator } from './Separator.jsx'

const transitionTimes = [32, 24, 16, 12, 8, 6, 4, 3, 2, 1.5, 1, 0.75, 5, 0.25]

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
  onChangeTransitionTime,
  onChangeTapTempoEnabled,
  onClearBuffer,
  onQueueRecord,
  onRecord,
  onReset,
  onSave,
  onSwitchSketch,
  onTogglePlay,
  onViewMidi,
}) {
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
          FPS: <span className="meter-value">{fps}</span>
        </div>
      </section>
      <Separator style={{ margin: '2px 0' }} />
      <section>
        <Select
          value={sketchName}
          options={sketchNames}
          onChange={(e) => {
            onSwitchSketch(e.target.value)
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
          BPM: <span className="meter-value">{bpm}</span>
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
          onChange={onChangeTransitionTime}
        />
        <VerticalSeparator />
        <button onClick={onSave}>Save</button>
        <button className={view === 'midi' ? 'on' : ''} onClick={onViewMidi}>
          MIDI
        </button>
      </section>
    </header>
  )
}
