import React from 'react'
import NumberBox from './NumberBox.jsx'
import Select from './Select.jsx'
import Separator from './Separator.jsx'

export default function Controls({ controls, onChange: parentOnChange }) {
  function onChange(type, index, value) {
    const updatedControls = [...controls]
    const kind = Object.keys(updatedControls[index])[0]
    updatedControls[index][kind].value = value
    const name = updatedControls[index][kind].name
    parentOnChange(type, name, value, updatedControls)
  }

  return controls.map((control, index) => {
    const type = Object.keys(control)[0]
    const c = control[type]

    if (type === 'checkbox') {
      return (
        <fieldset key={c.name}>
          <input
            id={c.name}
            type="checkbox"
            checked={c.value}
            disabled={c.disabled}
            onChange={(e) => {
              onChange('checkbox', index, e.target.checked)
            }}
          />
          <label htmlFor={c.name}>{c.name}</label>
        </fieldset>
      )
    }

    if (type === 'slider') {
      return (
        <fieldset key={c.name}>
          <input
            id={c.name}
            type="range"
            value={c.value}
            min={c.min}
            max={c.max}
            step={c.step}
            disabled={c.disabled}
            onChange={(e) => {
              onChange('slider', index, parseFloat(e.target.value))
            }}
          />
          <NumberBox
            value={c.value}
            min={c.min}
            max={c.max}
            step={c.step}
            disabled={c.disabled}
            onChange={(value) => {
              onChange('slider', index, parseFloat(value))
            }}
          />
          <label htmlFor={c.name}>{c.name}</label>
        </fieldset>
      )
    }

    if (type === 'select') {
      return (
        <fieldset key={c.name}>
          <Select
            id={c.name}
            value={c.value}
            options={c.options}
            disabled={c.disabled}
            onChange={(e) => {
              onChange('select', index, e.target.value)
            }}
          />
          <label htmlFor={c.name}>{c.name}</label>
        </fieldset>
      )
    }

    if (type === 'dynamicSeparator') {
      return <Separator key={c.name} />
    }

    return null
  })
}
