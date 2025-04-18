import clsx from 'clsx/lite'
import IconButton from './IconButton'

const availableSlots = Array(10)
  .fill(0)
  .map((_, i) => String((i + 1) % 10))

type Props = {
  snapshots: string[]
  onDelete: (snapshot: string) => void
  onLoad: (snapshot: string) => void
  onSave: (snapshot: string) => void
}

export default function Snapshots({
  snapshots,
  onDelete,
  onLoad,
  onSave,
}: Props) {
  return (
    <div id="snapshots">
      {availableSlots.map((slot) => {
        const hasSnapshot = !!snapshots.find((id) => id === slot)
        return (
          <div key={slot}>
            <button
              className={clsx('slot', hasSnapshot && 'on')}
              onClick={() => {
                if (hasSnapshot) {
                  onLoad(slot)
                } else {
                  onSave(slot)
                }
              }}
            >
              [{slot}]
            </button>
            {hasSnapshot && (
              <IconButton
                name="Close"
                onClick={() => {
                  onDelete(slot)
                }}
              />
            )}
          </div>
        )
      })}
    </div>
  )
}
