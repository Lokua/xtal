import React from 'react'

export default function Select({ value, options, onChange, ...rest }) {
  return (
    <span className="select-wrapper">
      <select value={value} onChange={onChange} {...rest}>
        {options.map((option) => (
          <option key={option} value={option}>
            {option}
          </option>
        ))}
      </select>
    </span>
  )
}
