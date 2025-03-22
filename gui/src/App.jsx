import React, { useEffect, useState } from 'react'
import { postText } from './util.mjs'

export default function App() {
  const [inputPort, setInputPort] = useState('')
  const [count, setCount] = useState(0)

  useEffect(() => {
    let unsubscribe = window.latticeEvents.subscribe((e) => {
      setCount((count) => count + 1)
      console.log(e)
    })

    return () => {
      unsubscribe()
    }
  }, [])

  return (
    <>
      <h1>Count: {count}</h1>
      <Select
        value={inputPort}
        options={window.latticeData.inputPorts}
        onChange={(e) => {
          postText(e.target.value)
          setInputPort(e.target.value)
        }}
      />
    </>
  )
}

function Select({ value, options, onChange }) {
  return (
    <select value={value} onChange={onChange}>
      {options.map((option) => (
        <option key={option} value={option}>
          {option}
        </option>
      ))}
    </select>
  )
}
