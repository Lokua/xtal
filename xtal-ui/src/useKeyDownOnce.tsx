import { useRef, useEffect } from 'react'

type KeyDownOnceCallback = (e: KeyboardEvent) => void

export default function useKeyDownOnce(onKeyDownOnce: KeyDownOnceCallback) {
  const pressedKeysRef = useRef<Set<string>>(new Set())

  useEffect(
    function () {
      function onKeyDown(e: KeyboardEvent) {
        if (!pressedKeysRef.current.has(e.code)) {
          pressedKeysRef.current.add(e.code)
          onKeyDownOnce(e)
        }
      }

      function onKeyUp(e: KeyboardEvent) {
        pressedKeysRef.current.delete(e.code)
      }

      window.addEventListener('keydown', onKeyDown)
      window.addEventListener('keyup', onKeyUp)

      return function () {
        window.removeEventListener('keydown', onKeyDown)
        window.removeEventListener('keyup', onKeyUp)
      }
    },
    [onKeyDownOnce]
  )

  return {
    pressedKeys: pressedKeysRef.current,
  }
}
