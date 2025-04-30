import { ReactNode, useEffect, useState } from 'react'
import NumberBox from '@lokua/number-box'
import ExcludedIcon from '@material-symbols/svg-400/outlined/keep.svg?react'
import MappedIcon from '@material-symbols/svg-400/outlined/app_badging.svg?react'
import clsx from 'clsx/lite'

import { Bypassed, Control, ControlValue, Exclusions, Mappings } from './types'

import CheckboxInput from './Checkbox'
import Select from './Select'
import Separator, { VerticalSeparator } from './Separator'
import { useLocalSettings } from './LocalSettings'
import Snapshots from './Snapshots'
import { isMac } from './util'

const ExcludedIndicator = () => (
  <span
    className="indicator-icon"
    title="This control is currently excluded from Randomization"
  >
    <ExcludedIcon />
  </span>
)

const MappedIndicator = () => (
  <span
    className="indicator-icon"
    title="This control is currently overridden by a MIDI Mapping"
  >
    <MappedIcon />
  </span>
)

export type Props = {
  bypassed: Bypassed
  controls: Control[]
  exclusions: Exclusions
  mappings: Mappings
  mappingsEnabled: boolean
  showExclusions: boolean
  showSnapshots: boolean
  singleTransitionControlName: string
  snapshots: string[]
  transitionInProgress: boolean
  onChange: (control: Control, value: ControlValue) => void
  onClickRandomize: (name: string) => void
  onClickRevert: (control: Control) => void
  onDeleteSnapshot: (snapshot: string) => void
  onLoadSnapshot: (snapshot: string) => void
  onToggleExclusion: (name: string) => void
  onSaveSnapshot: (snapshot: string) => void
}

export default function Controls({
  bypassed,
  controls,
  exclusions,
  mappings,
  mappingsEnabled,
  showExclusions,
  showSnapshots,
  singleTransitionControlName,
  snapshots,
  transitionInProgress,
  onChange,
  onClickRandomize,
  onClickRevert,
  onDeleteSnapshot,
  onLoadSnapshot,
  onToggleExclusion,
  onSaveSnapshot,
}: Props) {
  const [platformModPressed, setPlatformModPressed] = useState(false)
  const { localSettings } = useLocalSettings()

  useEffect(() => {
    function keyHandler(e: KeyboardEvent) {
      setPlatformModPressed(isMac ? e.metaKey : e.ctrlKey)
    }

    document.addEventListener('keydown', keyHandler)
    document.addEventListener('keyup', keyHandler)

    return () => {
      document.removeEventListener('keydown', keyHandler)
      document.removeEventListener('keyup', keyHandler)
    }
  }, [])

  function excludedAndNode(name: string): [boolean, ReactNode] {
    const excluded = exclusions.includes(name)

    if (!showExclusions) {
      return [excluded, null]
    }

    return [
      excluded,
      <>
        <CheckboxInput
          checked={excluded}
          onChange={() => {
            onToggleExclusion(name)
          }}
        />
        <VerticalSeparator />
      </>,
    ]
  }

  const controlClass = (name: string, excluded: boolean) =>
    clsx(
      'control-row',
      ((!excluded && transitionInProgress) ||
        singleTransitionControlName === name) &&
        'in-transition'
    )

  return (
    <div id="main-view">
      {showSnapshots && (
        <header>
          <Snapshots
            snapshots={snapshots}
            onDelete={onDeleteSnapshot}
            onLoad={onLoadSnapshot}
            onSave={onSaveSnapshot}
          />
        </header>
      )}
      <main>
        {controls.map((c, index) => {
          if (c.kind === 'Checkbox') {
            const [excluded, nodeWithCheckbox] = excludedAndNode(c.name)

            return (
              <div key={c.name} className={controlClass(c.name, excluded)}>
                {nodeWithCheckbox}
                <fieldset>
                  <CheckboxInput
                    id={c.name}
                    type="checkbox"
                    checked={c.value as boolean}
                    disabled={c.disabled}
                    onChange={() => {
                      onChange(c, !c.value)
                    }}
                  />
                  <label htmlFor={c.name}>
                    {excluded && <ExcludedIndicator />}
                    <span>{c.name}</span>
                  </label>
                </fieldset>
              </div>
            )
          }

          if (c.kind === 'Slider') {
            const isBypassed = c.name in bypassed
            const isMapped = mappingsEnabled && c.name in mappings
            const disabled = c.disabled || isBypassed || isMapped
            const [excluded, nodeWithCheckbox] = excludedAndNode(c.name)

            return (
              <div key={c.name} className={controlClass(c.name, excluded)}>
                {nodeWithCheckbox}
                <fieldset key={c.name}>
                  <input
                    id={c.name}
                    type="range"
                    value={c.value as number}
                    min={c.min}
                    max={c.max}
                    step={c.step}
                    disabled={disabled}
                    onChange={(e) => {
                      onChange(c, e.currentTarget.valueAsNumber)
                    }}
                  />
                  <NumberBox
                    data-help-id="NumberBox"
                    className="number-box"
                    value={c.value as number}
                    min={c.min}
                    max={c.max}
                    step={c.step}
                    disabled={disabled}
                    onChange={(value) => {
                      onChange(c, value)
                    }}
                  />
                  <label
                    data-help-id="ControlLabel"
                    htmlFor={c.name}
                    className={clsx(!c.disabled && !isBypassed && 'clickable')}
                    onClick={() => {
                      if (platformModPressed) {
                        onClickRevert(c)
                      } else {
                        onClickRandomize(c.name)
                      }
                    }}
                  >
                    {excluded && <ExcludedIndicator />}
                    {isMapped && <MappedIndicator />}
                    <span
                      title={
                        isBypassed
                          ? 'This control is currently bypassed/overwritten in a Control Script'
                          : ''
                      }
                      style={{
                        width:
                          (showExclusions ? -1.625 : 0) +
                          (excluded ? -0.875 : 0) +
                          (isMapped ? -0.875 : 0) +
                          { 16: 9.75, 17: 8.5, 18: 6.5 }[
                            localSettings.fontSize
                          ] +
                          'rem',
                        textDecoration: isBypassed ? 'line-through' : 'none',
                      }}
                    >
                      <span
                        className={clsx('text', platformModPressed && 'revert')}
                      >
                        {c.name}
                      </span>
                    </span>
                  </label>
                </fieldset>
              </div>
            )
          }

          if (c.kind === 'Select') {
            const [excluded, nodeWithCheckbox] = excludedAndNode(c.name)

            return (
              <div key={c.name} className={controlClass(c.name, excluded)}>
                {nodeWithCheckbox}
                <fieldset key={c.name}>
                  <Select
                    id={c.name}
                    value={c.value as string}
                    options={c.options}
                    disabled={c.disabled}
                    onChange={(value) => {
                      onChange(c, value)
                    }}
                  />
                  <label
                    data-help-id="ControlLabel"
                    htmlFor={c.name}
                    className={clsx(!c.disabled && !excluded && 'clickable')}
                    onClick={() => {
                      onClickRandomize(c.name)
                    }}
                  >
                    {excluded && <ExcludedIndicator />}
                    <span className="text">{c.name}</span>
                  </label>
                </fieldset>
              </div>
            )
          }

          if (c.kind === 'Separator') {
            return (
              <div className="separator-control-container">
                <small>{c.name}</small>
                <Separator key={c.name || index} />
              </div>
            )
          }

          return null
        })}
      </main>
    </div>
  )
}
