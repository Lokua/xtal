import { useEffect, useState } from 'react'
import { Help } from './Help'

type Props = {
  alertText: string
}

export default function Console({ alertText }: Props) {
  const [helpText, setHelpText] = useState('')

  useEffect(() => {
    function onMouseOver(e: MouseEvent) {
      let currentTarget = e.target as HTMLElement | null
      let helpId = null

      while (currentTarget && !helpId) {
        helpId = currentTarget.dataset.helpId
        if (!helpId) {
          currentTarget = currentTarget.parentElement
        }
      }

      if (helpId && helpId in Help) {
        const text = Help[helpId as keyof typeof Help]
        setHelpText(text)
      } else {
        setHelpText('')
      }
    }

    document.addEventListener('mouseover', onMouseOver)

    return () => {
      document.removeEventListener('mouseover', onMouseOver)
    }
  }, [alertText])

  return <div className="console">{helpText || alertText}</div>
}
