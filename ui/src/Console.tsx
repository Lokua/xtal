import { useEffect, useState } from 'react'
import { Help } from './Help'

type Props = {
  alertText: string
}

export default function Console({ alertText }: Props) {
  const [helpText, setHelpText] = useState('')

  useEffect(() => {
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

  return <div className="console">{helpText || alertText}</div>
}
