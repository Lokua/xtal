import { useEffect, useState } from 'react'
import { Help } from './Help'
import IconButton from './IconButton'

type Props = {
  alertText: string
  showHelp: boolean
  onToggleShowHelp: (negatedShowHelp: boolean) => void
}

export default function Console({
  alertText,
  showHelp,
  onToggleShowHelp,
}: Props) {
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
  }, [])

  return (
    <div className="console">
      <IconButton
        name="Help"
        title="When on, hover over elements to view help information.
          When off, the console will show system alerts."
        on={showHelp}
        onClick={() => {
          onToggleShowHelp(!showHelp)
        }}
      />
      {showHelp ? helpText : alertText}
    </div>
  )
}
