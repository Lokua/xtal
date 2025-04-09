import clsx from 'clsx/lite'
import IconButton from './IconButton'

const availableSlots = Array(10)
  .fill(0)
  .map((_, i) => String((i + 1) % 10))

type Props = {
  snapshots: string[]
  onDelete: (snapshot: string) => void
  onDeleteAll: () => void
  onLoad: (snapshot: string) => void
  onSave: (snapshot: string) => void
}

export default function Snapshots({
  snapshots,
  onDelete,
  onDeleteAll,
  onLoad,
  onSave,
}: Props) {
  return (
    <div id="snapshots">
      {availableSlots.map((slot, i) => {
        const hasSnapshot = snapshots[i] === slot
        return (
          <div key={slot}>
            <button
              className={clsx(hasSnapshot && 'on')}
              onClick={() => {
                if (hasSnapshot) {
                  onLoad(slot)
                } else {
                  onSave(slot)
                }
              }}
            >
              [{slot}] {hasSnapshot ? 'Load' : 'Save'}
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
      <footer style={{ marginTop: '48px', textAlign: 'center' }}>
        <button onClick={onDeleteAll} disabled={snapshots.length === 0}>
          Delete All
        </button>
      </footer>
    </div>
  )
}
