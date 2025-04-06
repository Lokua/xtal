import Check from '@material-symbols/svg-400/outlined/check.svg?react'
import type { Override } from './types.ts'

type Props = Override<
  React.InputHTMLAttributes<HTMLInputElement>,
  {
    onChange: (value: boolean) => void
  }
>

export default function Checkbox({ checked, onChange, ...rest }: Props) {
  return (
    <span className="checkbox-wrapper">
      <input
        type="checkbox"
        {...rest}
        checked={checked}
        onChange={() => {
          onChange(!checked)
        }}
      />
      {checked && <Check />}
    </span>
  )
}
