import { useEffect, useState } from 'react'
import { Help } from './Help'

type Props = {
  alertText: string
}

const USE_SINGLE_PANEL = true

export default function Console({ alertText }: Props) {
  const [helpText, setHelpText] = useState('')

  useEffect(() => {
    if (USE_SINGLE_PANEL && alertText) {
      setHelpText(alertText)
    }

    function onMouseOver(e: MouseEvent) {
      if (e.target) {
        const element = e.target as HTMLElement
        const helpId = element.dataset.helpId
        if (helpId !== undefined && helpId in Help) {
          const text = Help[helpId as keyof typeof Help]
          setHelpText(text)
        } else {
          setHelpText('')
        }
      }
    }

    document.addEventListener('mouseover', onMouseOver)

    return () => {
      document.removeEventListener('mouseover', onMouseOver)
    }
  }, [alertText])

  return USE_SINGLE_PANEL ? (
    <div className="console-single-panel">{helpText || alertText}</div>
  ) : (
    <div className="console">
      <section>{alertText}</section>
      <section>{helpText}</section>
    </div>
  )
}
