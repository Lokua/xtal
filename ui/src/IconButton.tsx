import type { Override } from './types.ts'
import Advance from '@material-symbols/svg-400/outlined/start.svg?react'
import Camera from '@material-symbols/svg-400/outlined/camera.svg?react'
import Clear from '@material-symbols/svg-400/outlined/ink_eraser.svg?react'
import Image from '@material-symbols/svg-400/outlined/image.svg?react'
import Pause from '@material-symbols/svg-400/outlined/pause.svg?react'
import Perf from '@material-symbols/svg-400/outlined/lock.svg?react'
import Play from '@material-symbols/svg-400/outlined/play_arrow.svg?react'
import Queue from '@material-symbols/svg-400/outlined/add_to_queue.svg?react'
import Queued from '@material-symbols/svg-400/outlined/queue_play_next.svg?react'
import Random from '@material-symbols/svg-400/outlined/shuffle.svg?react'
import Reset from '@material-symbols/svg-400/outlined/undo.svg?react'
import Record from '@material-symbols/svg-400/outlined/screen_record.svg?react'
import Save from '@material-symbols/svg-400/outlined/save.svg?react'
import Settings from '@material-symbols/svg-400/outlined/settings.svg?react'
import StopRecording from '@material-symbols/svg-400/outlined/stop_circle.svg?react'
import Tap from '@material-symbols/svg-400/outlined/touch_app.svg?react'
import clsx from 'clsx'

const icons = {
  Advance,
  Camera,
  Clear,
  Image,
  Pause,
  Perf,
  Play,
  Queue,
  Queued,
  Random,
  Reset,
  Record,
  Save,
  Settings,
  StopRecording,
  Tap,
}

type Props = Override<
  React.ButtonHTMLAttributes<HTMLButtonElement>,
  {
    name: keyof typeof icons
    on?: boolean
  }
>

export default function IconButton({
  name,
  className,
  disabled,
  on = false,
  ...rest
}: Props) {
  const Icon = icons[name]

  return (
    <button
      className={clsx(
        'icon-button',
        on && !disabled && 'on',
        `${name}-icon`,
        className
      )}
      disabled={disabled}
      {...rest}
    >
      <Icon />
    </button>
  )
}
