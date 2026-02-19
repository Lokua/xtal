import type { Override } from './types'

type OptionGroup = {
  label: string
  options: string[]
}

type Props = Override<
  React.SelectHTMLAttributes<HTMLSelectElement>,
  {
    value: string
    options?: string[] | number[]
    optionGroups?: OptionGroup[]
    onChange: (value: string) => void
  }
>

export default function Select({
  value,
  options = [],
  optionGroups,
  onChange,
  ...rest
}: Props) {
  return (
    <span className="select-wrapper">
      <select
        value={value}
        onChange={(e) => {
          onChange(e.currentTarget.value)
        }}
        {...rest}
      >
        {optionGroups && optionGroups.length > 0
          ? optionGroups.map((group) => (
              <optgroup key={group.label} label={group.label}>
                {group.options.map((option) => (
                  <option key={option} value={option}>
                    {option}
                  </option>
                ))}
              </optgroup>
            ))
          : options.map((option) => (
              <option key={option} value={option}>
                {option}
              </option>
            ))}
      </select>
    </span>
  )
}
