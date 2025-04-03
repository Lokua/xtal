import { useState, useEffect, useRef } from 'react'

type OscPortInputProps = {
  port: number
  onChange: (port: number) => void
}

export default function OscPortInput({
  port = 8000,
  onChange,
}: OscPortInputProps) {
  const [inputValue, setInputValue] = useState(port.toString())
  const timeoutRef = useRef<number | null>(null)
  const inputRef = useRef<HTMLInputElement>(null)

  function validatePort(value: string): boolean {
    const portNumber = parseInt(value, 10)
    if (inputRef.current) {
      if (isNaN(portNumber) || portNumber < 1024 || portNumber > 65535) {
        inputRef.current.setCustomValidity(
          'Port must be between 1024 and 65535'
        )
        inputRef.current.reportValidity()
        return false
      } else {
        inputRef.current.setCustomValidity('')
        inputRef.current.reportValidity()
        return true
      }
    }
    return false
  }

  function onChangeInput(e: React.ChangeEvent<HTMLInputElement>) {
    const value = e.target.value
    setInputValue(value)

    if (inputRef.current) {
      validatePort(value)
    }

    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current)
    }

    timeoutRef.current = setTimeout(() => {
      if (inputRef.current && validatePort(value)) {
        const portNumber = parseInt(value, 10)
        onChange(portNumber)
      }
    }, 500)
  }

  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current)
      }
    }
  }, [])

  return (
    <fieldset>
      <input
        ref={inputRef}
        id="osc-port"
        type="text"
        pattern="[0-9]*"
        value={inputValue}
        onChange={onChangeInput}
      />
      <label htmlFor="osc-port">Port</label>
    </fieldset>
  )
}
