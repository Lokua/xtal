import NumberBox from '@lokua/number-box'
import {
  Bypassed,
  Checkbox,
  Control,
  ControlValue,
  ControlWithValue,
  DynamicSeparator,
  Exclusions,
  Mappings,
  Select as SelectType,
  Slider,
} from './types.ts'
import CheckboxInput from './Checkbox.tsx'
import Select from './Select.tsx'
import Separator, { VerticalSeparator } from './Separator.tsx'
import ExcludedIcon from '@material-symbols/svg-400/outlined/keep.svg?react'
import MappedIcon from '@material-symbols/svg-400/outlined/app_badging.svg?react'

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
  onChange: (
    type: string,
    name: string,
    value: ControlValue,
    updatedControls: Control[]
  ) => void
  onToggleExclusion: (name: string) => void
}

export default function Controls({
  bypassed,
  controls,
  exclusions,
  mappings,
  showExclusions,
  onChange: parentOnChange,
  onToggleExclusion,
}: Props) {
  // TODO: ETL on controls so we don't have to deal with this awkward bincode
  // structure
  function onChange(type: string, index: number, value: ControlValue) {
    const updatedControls = [...controls] as ControlWithValue[]
    const kind = Object.keys(
      updatedControls[index]
    )[0] as keyof ControlWithValue
    const control = updatedControls[index][kind] as {
      value: ControlValue
      name: string
    }
    control.value = value
    const name = control.name
    parentOnChange(type, name, value, updatedControls)
  }

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
          kind={excluded && 'excluded'}
          onChange={() => {
            onToggleExclusion(name)
          }}
        />
        <VerticalSeparator />
      </>,
    ]
  }

  return controls.map((control, index) => {
    const type = Object.keys(control)[0] as keyof Control

    if (type === 'checkbox') {
      const c = control[type] as Checkbox['checkbox']
      const [excluded, nodeWithCheckbox] = excludedAndNode(c.name)

      return (
        <div key={c.name} className="control-row">
          {nodeWithCheckbox}
          <fieldset>
            <CheckboxInput
              id={c.name}
              type="checkbox"
              checked={c.value}
              disabled={c.disabled}
              onChange={() => {
                onChange('checkbox', index, !c.value)
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

    if (type === 'slider') {
      const c = control[type] as Slider['slider']
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
              value={c.value}
              min={c.min}
              max={c.max}
              step={c.step}
              disabled={disabled}
              onChange={(e) => {
                onChange('slider', index, e.currentTarget.valueAsNumber)
              }}
            />
            <NumberBox
              className="number-box"
              value={c.value}
              min={c.min}
              max={c.max}
              step={c.step}
              disabled={disabled}
              onChange={(value) => {
                onChange('slider', index, value)
              }}
            />
            <label htmlFor={c.name}>
              {excluded && <ExcludedIndicator />}
              {isMapped && <MappedIndicator />}
              <span
                title={
                  isBypassed
                    ? 'This control is currently bypassed and overwritten in a Control Script'
                    : ''
                }
                style={{
                  width:
                    (showExclusions ? -26 : 0) +
                    (isBypassed ? -14 : 0) +
                    (excluded ? -14 : 0) +
                    (isMapped ? -14 : 0) +
                    156 +
                    'px',
                  textDecoration: isBypassed ? 'line-through' : 'none',
                }}
              >
                {c.name}
              </span>
            </label>
          </fieldset>
        </div>
      )
    }

    if (type === 'select') {
      const c = control[type] as SelectType['select']
      const [excluded, nodeWithCheckbox] = excludedAndNode(c.name)

      return (
        <div key={c.name} className="control-row">
          {nodeWithCheckbox}
          <fieldset key={c.name}>
            <Select
              id={c.name}
              value={c.value}
              options={c.options}
              disabled={c.disabled}
              onChange={(value) => {
                onChange('select', index, value)
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

    if (type === 'dynamicSeparator') {
      const c = control[type] as DynamicSeparator['dynamicSeparator']
      return <Separator key={c.name} />
    }

    return null
  })
}
