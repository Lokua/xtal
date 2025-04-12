import Advance from '@material-symbols/svg-400/outlined/skip_next.svg?react'
import Camera from '@material-symbols/svg-400/outlined/camera.svg?react'
import Clear from '@material-symbols/svg-400/outlined/remove_from_queue.svg?react'
import Close from '@material-symbols/svg-400/outlined/close.svg?react'
import DeleteMappings from '@material-symbols/svg-400/outlined/delete.svg?react'
import DisableMappings from '@material-symbols/svg-400/outlined/block.svg?react'
import Folder from '@material-symbols/svg-400/outlined/folder.svg?react'
import Image from '@material-symbols/svg-400/outlined/image.svg?react'
import Pause from '@material-symbols/svg-400/outlined/pause.svg?react'
import Perf from '@material-symbols/svg-400/outlined/lock.svg?react'
import Play from '@material-symbols/svg-400/outlined/play_arrow.svg?react'
import Queue from '@material-symbols/svg-400/outlined/timer_play.svg?react'
import Queued from '@material-symbols/svg-400/outlined/timer_play.svg?react'
import Random from '@material-symbols/svg-400/outlined/shuffle.svg?react'
import Reload from '@material-symbols/svg-400/outlined/refresh.svg?react'
import Reset from '@material-symbols/svg-400/outlined/undo.svg?react'
import Record from '@material-symbols/svg-400/outlined/fiber_manual_record.svg?react'
import Recording from '@material-symbols/svg-400/outlined/fiber_manual_record-fill.svg?react'
import Save from '@material-symbols/svg-400/outlined/save.svg?react'
import Settings from '@material-symbols/svg-400/outlined/settings.svg?react'
import Snapshots from '@material-symbols/svg-400/outlined/auto_awesome_motion.svg?react'
import StopRecording from '@material-symbols/svg-400/outlined/stop_circle.svg?react'
import Tap from '@material-symbols/svg-400/outlined/touch_app.svg?react'
import Exclusions from '@material-symbols/svg-400/outlined/checklist.svg?react'
import clsx from 'clsx/lite'

import type { Override } from './types.ts'

const icons = {
  Advance,
  Camera,
  Clear,
  Close,
  DeleteMappings,
  DisableMappings,
  Exclusions,
  Folder,
  Image,
  Pause,
  Perf,
  Play,
  Queue,
  Queued,
  Random,
  Reload,
  Reset,
  Record,
  Recording,
  Save,
  Settings,
  Snapshots,
  StopRecording,
  Tap,
}

type Props = Override<
  React.ButtonHTMLAttributes<HTMLButtonElement>,
  {
    name: keyof typeof icons
    on?: boolean
    isToggle?: boolean
  }
>

export default function IconButton({
  name,
  className,
  disabled,
  on = false,
  isToggle = false,
  ...rest
}: Props) {
  const Icon = icons[name]

  return (
    <button
      className={clsx(
        'icon-button',
        on && !disabled && 'on',
        `${name}-icon`,
        className,
        isToggle && 'toggle'
      )}
      disabled={disabled}
      {...rest}
    >
      <Icon />
    </button>
  )
}
