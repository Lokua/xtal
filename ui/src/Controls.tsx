import NumberBox from '@lokua/number-box'
import CheckboxInput from './Checkbox.tsx'
import Select from './Select.tsx'
import Separator from './Separator.tsx'
import {
  Bypassed,
  Checkbox,
  Control,
  ControlValue,
  ControlWithValue,
  DynamicSeparator,
  Mappings,
  Select as SelectType,
  Slider,
} from './types.ts'

type Props = {
  bypassed: Bypassed
  controls: Control[]
  mappings: Mappings
  onChange: (
    type: string,
    name: string,
    value: ControlValue,
    updatedControls: Control[]
  ) => void
}

export default function Controls({
  bypassed,
  controls,
  mappings,
  onChange: parentOnChange,
}: Props) {
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

  return controls.map((control, index) => {
    const type = Object.keys(control)[0] as keyof Control

    if (type === 'checkbox') {
      const c = control[type] as Checkbox['checkbox']

      return (
        <fieldset key={c.name + '2'}>
          <CheckboxInput
            id={c.name}
            type="checkbox"
            checked={c.value}
            disabled={c.disabled}
            onChange={() => {
              onChange('checkbox', index, !c.value)
            }}
          />
          <label htmlFor={c.name}>{c.name}</label>
        </fieldset>
      )
    }

    if (type === 'slider') {
      const c = control[type] as Slider['slider']
      const isBypassed = c.name in bypassed
      const isMapped = !!mappings.find((m) => m[0] === c.name)
      const disabled = c.disabled || isBypassed || isMapped

      return (
        <fieldset
          key={c.name}
          title={
            isMapped
              ? 'This parameter is currently overridden by a MIDI mapping'
              : isBypassed
              ? 'This parameter is currently bypassed in a control script'
              : ''
          }
        >
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
            {isMapped && <span className="control-meta control-mapped-text" />}
            {isBypassed && (
              <span className="control-meta control-bypassed-text" />
            )}
            {c.name}
          </label>
        </fieldset>
      )
    }

    if (type === 'select') {
      const c = control[type] as SelectType['select']

      return (
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
          <label htmlFor={c.name}>{c.name}</label>
        </fieldset>
      )
    }

    if (type === 'dynamicSeparator') {
      const c = control[type] as DynamicSeparator['dynamicSeparator']
      return <Separator key={c.name} />
    }

    return null
  })
}
