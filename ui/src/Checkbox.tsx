import Check from '@material-symbols/svg-400/outlined/check.svg?react'
import type { Override } from './types.ts'
import clsx from 'clsx/lite'

type Props = Override<
  React.InputHTMLAttributes<HTMLInputElement>,
  {
    kind?: false | 'excluded'
    onChange: (value: boolean) => void
  }
>

export default function Checkbox({
  checked,
  kind = false,
  onChange,
  ...rest
}: Props) {
  return (
    <span className={clsx('checkbox-wrapper', kind)}>
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
