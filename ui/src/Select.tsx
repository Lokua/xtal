import type { Override } from './types.ts'

type Props = Override<
  React.SelectHTMLAttributes<HTMLSelectElement>,
  {
    value: string
    options: string[] | number[]
    onChange: (value: string) => void
  }
>

export default function Select({ value, options, onChange, ...rest }: Props) {
  return (
    <span className="select-wrapper">
      <select
        value={value}
        onChange={(e) => {
          onChange(e.currentTarget.value)
        }}
        {...rest}
      >
        {options.map((option) => (
          <option key={option} value={option}>
            {option}
          </option>
        ))}
      </select>
    </span>
  )
}
