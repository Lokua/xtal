import clsx from 'clsx/lite'
import IconButton from './IconButton'

const availableSlots = Array(10)
  .fill(0)
  .map((_, i) => String((i + 1) % 10))

type Props = {
  disabled: boolean
  snapshots: string[]
  onDelete: (snapshot: string) => void
  onLoad: (snapshot: string) => void
  onSave: (snapshot: string) => void
}

export default function Snapshots({
  disabled,
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
          <div key={slot} className="snapshot-slot">
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
      {disabled && (
        <div className="snapshot-sequence-overlay">
          Snapshot Sequence in progress.
          <br />
          Disable to edit snapshots
        </div>
      )}
    </div>
  )
}
