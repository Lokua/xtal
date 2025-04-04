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
  const precision = numberOfDecimals(step)
  const internalValue = useRef(value)
  const inputRef = useRef<HTMLInputElement | null>(null)
  const prevY = useRef(0)
  const [shiftHeld, setShiftHeld] = useState(false)
  const [inputValue, setInputValue] = useState(value.toFixed(precision))
  const [editing, setEditing] = useState(false)

  useEffect(() => {
    internalValue.current = value
    setInputValue(value.toFixed(precision))
  }, [value, precision])

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
      setInputValue(finalValue.toFixed(precision))
      onChange(finalValue)
    }

    prevY.current = clientY
  }

  function onMouseDown(e: React.MouseEvent) {
    if (!editing) {
      // e.preventDefault to bypass user-select which interferes with dragging.
      // Disabling user-select or webkit-user-select at runtime via inline style
      // doesn't seem to work (TODO: we could try toggling classes?)
      e.preventDefault()
      window.addEventListener('mousemove', onMouseMove)
      window.addEventListener('mouseup', onMouseUp)
      prevY.current = e.clientY
    }
  }

  function onMouseMove(e: MouseEvent) {
    applyDelta(e.clientY)
  }

  function onMouseUp() {
    window.removeEventListener('mousemove', onMouseMove)
    window.removeEventListener('mouseup', onMouseUp)
  }

  function onTouchStart(e: React.TouchEvent) {
    if (!editing) {
      e.preventDefault()
      window.addEventListener('touchmove', onTouchMove as EventListener)
      window.addEventListener('touchend', onTouchEnd)
      prevY.current = e.touches[0].clientY
    }
  }

  function onTouchMove(e: TouchEvent) {
    applyDelta(e.touches[0].clientY)
  }

  function onTouchEnd() {
    window.removeEventListener('touchmove', onTouchMove as EventListener)
    window.removeEventListener('touchend', onTouchEnd)
  }

  function onChangeInput(e: React.ChangeEvent<HTMLInputElement>) {
    setInputValue(e.target.value)
  }

  function onBlur(e: React.FocusEvent<HTMLInputElement>) {
    const parsedValue = parseFloat(e.target.value)
    if (!isNaN(parsedValue)) {
      const stepsFromMin = Math.round((parsedValue - min) / step)
      const steppedValue = min + stepsFromMin * step
      const boundedValue = Math.min(max, Math.max(min, steppedValue))
      internalValue.current = boundedValue
      setInputValue(boundedValue.toFixed(precision))
      onChange(boundedValue)
    } else {
      setInputValue(internalValue.current.toFixed(precision))
    }
    setEditing(false)
  }

  function onDoubleClick() {
    setEditing(true)
    inputRef.current?.focus()
  }

  function onKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (editing && e.key === 'Enter') {
      inputRef.current?.blur()
    }
  }

  return (
    <input
      {...rest}
      type="text"
      ref={(element) => {
        inputRef.current = element
      }}
      value={inputValue}
      onMouseDown={onMouseDown}
      onTouchStart={onTouchStart}
      onChange={onChangeInput}
      onBlur={onBlur}
      onDoubleClick={onDoubleClick}
      onKeyDown={onKeyDown}
      style={{
        cursor: editing ? 'text' : 'ns-resize',
      }}
    />
  )
}
