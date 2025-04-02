import { useEffect, useRef, useState } from 'react'
import type { Override } from './types.ts'

type Props = Override<
  React.InputHTMLAttributes<HTMLInputElement>,
  {
    value: number
    onChange: (value: number) => void
    min?: number
    max?: number
    step?: number
    sensitivity?: number
  }
>

function numberOfDecimals(step: number) {
  const x = step.toString()
  if (x.indexOf('.') > -1) {
    return x.length - x.indexOf('.') - 1
  }
  return 0
}

export default function NumberBox({
  value,
  min = 0,
  max = 100,
  step = 1,
  sensitivity = 200,
  onChange,
  ...rest
}: Props) {
  const internalValue = useRef(value)
  const inputRef = useRef<HTMLInputElement | null>(null)
  const prevY = useRef(0)
  const [shiftHeld, setShiftHeld] = useState(false)
  const precision = numberOfDecimals(step)

  useEffect(() => {
    internalValue.current = value
  }, [value])

  useEffect(() => {
    function onKeydown(e: KeyboardEvent) {
      if (e.key === 'Shift') {
        setShiftHeld(true)
      }
    }

    function onKeyup(e: KeyboardEvent) {
      if (e.key === 'Shift') {
        setShiftHeld(false)
      }
    }

    window.addEventListener('keydown', onKeydown)
    window.addEventListener('keyup', onKeyup)

    return () => {
      window.removeEventListener('keydown', onKeydown)
      window.removeEventListener('keyup', onKeyup)
    }
  }, [])

  function applyDelta(clientY: number) {
    const delta = prevY.current - clientY
    let newValue: number

    if (shiftHeld) {
      const direction = delta > 0 ? 1 : delta < 0 ? -1 : 0
      newValue =
        direction !== 0
          ? internalValue.current + step * direction
          : internalValue.current
    } else {
      const valuePerPixel = (max - min) / sensitivity
      const rawChange = delta * valuePerPixel
      newValue = internalValue.current + rawChange
    }

    const boundedValue = Math.min(max, Math.max(min, newValue))

    const finalValue = shiftHeld
      ? boundedValue
      : Math.round((boundedValue - min) / step) * step + min

    if (finalValue !== internalValue.current) {
      internalValue.current = finalValue
      onChange(finalValue)
    }

    prevY.current = clientY
  }

  function onMouseDown(e: React.MouseEvent) {
    window.addEventListener('mousemove', onMouseMove)
    window.addEventListener('mouseup', onMouseUp)
    prevY.current = e.clientY
  }

  function onMouseMove(e: MouseEvent) {
    applyDelta(e.clientY)
  }

  function onMouseUp() {
    window.removeEventListener('mousemove', onMouseMove)
    window.removeEventListener('mouseup', onMouseUp)
  }

  function onTouchStart(e: React.TouchEvent) {
    window.addEventListener('touchmove', onTouchMove as EventListener)
    window.addEventListener('touchend', onTouchEnd)
    prevY.current = e.touches[0].clientY
  }

  function onTouchMove(e: TouchEvent) {
    applyDelta(e.touches[0].clientY)
  }

  function onTouchEnd() {
    window.removeEventListener('touchmove', onTouchMove as EventListener)
    window.removeEventListener('touchend', onTouchEnd)
  }

  function onChangeInput(e: React.ChangeEvent<HTMLInputElement>) {
    const newValue = parseFloat(e.target.value)
    if (!isNaN(newValue)) {
      const stepsFromMin = Math.round((newValue - min) / step)
      const steppedValue = min + stepsFromMin * step
      const boundedValue = Math.min(max, Math.max(min, steppedValue))
      internalValue.current = boundedValue
      onChange(boundedValue)
    }
  }

  return (
    <input
      {...rest}
      type="text"
      ref={(element) => {
        inputRef.current = element
      }}
      value={internalValue.current.toFixed(precision)}
      onMouseDown={onMouseDown}
      onTouchStart={onTouchStart}
      onChange={onChangeInput}
    />
  )
}
