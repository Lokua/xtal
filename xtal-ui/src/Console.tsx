import { useEffect, useState } from 'react'
import { Help } from './Help'
import IconButton from './IconButton'

type Props = {
  alertText: string
  bpm: number
  isRecordingOrEncoding: boolean
  showHelp: boolean
  onToggleShowHelp: (negatedShowHelp: boolean) => void
}

function formatDuration(elapsedMs: number): string {
  const totalSeconds = Math.floor(elapsedMs / 1000)
  const hours = Math.floor(totalSeconds / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  const seconds = totalSeconds % 60

  return [hours, minutes, seconds]
    .map((value) => String(value).padStart(2, '0'))
    .join(':')
}

export default function Console({
  alertText,
  bpm,
  isRecordingOrEncoding,
  showHelp,
  onToggleShowHelp,
}: Props) {
  const [helpText, setHelpText] = useState('')
  const [elapsedMs, setElapsedMs] = useState(0)
  const [statusText, setStatusText] = useState('')

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

  useEffect(() => {
    if (!isRecordingOrEncoding) {
      setElapsedMs(0)
      setStatusText('')
      return
    }

    setElapsedMs(0)
    setStatusText('')
    const startedAt = Date.now()
    let animationFrameId = 0
    const updateElapsed = () => {
      setElapsedMs(Date.now() - startedAt)
      animationFrameId = window.requestAnimationFrame(updateElapsed)
    }
    animationFrameId = window.requestAnimationFrame(updateElapsed)

    return () => {
      window.cancelAnimationFrame(animationFrameId)
    }
  }, [isRecordingOrEncoding])

  useEffect(() => {
    if (!isRecordingOrEncoding) {
      return
    }

    const lowerAlertText = alertText.toLowerCase()
    const isRecordingOrEncodingMessage =
      lowerAlertText.includes('record') || lowerAlertText.includes('encod')

    if (isRecordingOrEncodingMessage) {
      setStatusText(alertText)
    }
  }, [alertText, isRecordingOrEncoding])

  const beatsElapsed =
    isRecordingOrEncoding && bpm > 0
      ? Math.floor((elapsedMs / 1000) * (bpm / 60))
      : 0
  const bar = Math.floor(beatsElapsed / 4)
  const beat = beatsElapsed % 4
  const statusLabel = statusText || 'Recording / Encoding...'

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
      {showHelp ? (
        helpText
      ) : isRecordingOrEncoding ? (
        <div className="console-status">
          <div>{statusLabel}</div>
          <div>{`Time: ${formatDuration(elapsedMs)}`}</div>
          <div>{`Bar:${bar} Beat:${beat}`}</div>
        </div>
      ) : (
        alertText
      )}
    </div>
  )
}
