import NumberBox from '@lokua/number-box'
import ExcludedIcon from '@material-symbols/svg-400/outlined/keep.svg?react'
import MappedIcon from '@material-symbols/svg-400/outlined/app_badging.svg?react'
import clsx from 'clsx/lite'
import { Bypassed, Control, ControlValue, Exclusions, Mappings } from './types'
import CheckboxInput from './Checkbox'
import Select from './Select'
import Separator, { VerticalSeparator } from './Separator'
import { useLocalSettings } from './LocalSettings'

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

type Props = {
  bypassed: Bypassed
  controls: Control[]
  exclusions: Exclusions
  mappings: Mappings
  showExclusions: boolean
  onChange: (control: Control, value: ControlValue) => void
  onClickRandomize: (name: string) => void
  onToggleExclusion: (name: string) => void
}

export default function Controls({
  bypassed,
  controls,
  exclusions,
  mappings,
  showExclusions,
  onChange,
  onClickRandomize,
  onToggleExclusion,
}: Props) {
  const { localSettings } = useLocalSettings()

  function excludedAndNode(name: string): [boolean, React.ReactNode] {
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

  return controls.map((c, index) => {
    if (c.kind === 'Checkbox') {
      const [excluded, nodeWithCheckbox] = excludedAndNode(c.name)

      return (
        <div key={c.name} className="control-row">
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
      const isMapped = !!mappings.find((m) => m[0] === c.name)
      const disabled = c.disabled || isBypassed || isMapped
      const [excluded, nodeWithCheckbox] = excludedAndNode(c.name)

      return (
        <div key={c.name} className="control-row">
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
              htmlFor={c.name}
              className={clsx(
                !c.disabled && !isBypassed && !excluded && 'clickable'
              )}
              onClick={() => {
                onClickRandomize(c.name)
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
                    { 16: 9.75, 17: 8.5, 18: 6.5 }[localSettings.fontSize] +
                    'rem',
                  textDecoration: isBypassed ? 'line-through' : 'none',
                }}
              >
                <span className="text">{c.name}</span>
              </span>
            </label>
          </fieldset>
        </div>
      )
    }

    if (c.kind === 'Select') {
      const [excluded, nodeWithCheckbox] = excludedAndNode(c.name)

      return (
        <div key={c.name} className="control-row">
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

    if (c.kind === 'DynamicSeparator' || c.kind === 'Separator') {
      return <Separator key={c.name || index} />
    }

    return null
  })
}
