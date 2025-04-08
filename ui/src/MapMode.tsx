import React, { useEffect, useState } from 'react'
import { Mappings } from './types'
import IconButton from './IconButton'

type MapModeProps = {
  sliderNames: string[]
  mappings: Mappings
  onRemoveMapping: (name: string) => void
  onSetCurrentlyMapping: (name: string) => void
}

export default function MapMode({
  sliderNames,
  mappings,
  onRemoveMapping,
  onSetCurrentlyMapping,
}: MapModeProps) {
  const [currentlyMapping, setCurrentlyMapping] = useState('')

  useEffect(() => {
    document.addEventListener('click', onOutsideClick)
    document.addEventListener('keydown', onKeyDown)

    return () => {
      document.removeEventListener('click', onOutsideClick)
      document.removeEventListener('keydown', onKeyDown)
    }

    function onOutsideClick(e: MouseEvent) {
      if (
        currentlyMapping &&
        !(e.target as HTMLButtonElement)?.classList?.contains('map-button')
      ) {
        clearCurrentlyMapping()
      }
    }

    function onKeyDown(e: KeyboardEvent) {
      if (e.code === 'Enter') {
        clearCurrentlyMapping()
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentlyMapping])

  function findMapping(name: string) {
    return mappings.find((m) => m[0] === name)
  }

  function clearCurrentlyMapping() {
    setCurrentlyMapping('')
    onSetCurrentlyMapping('')
  }

  function onClickMap(name: string) {
    if (currentlyMapping !== name) {
      setCurrentlyMapping(name)
      onSetCurrentlyMapping(name)
    }
  }

  return (
    <div className="map-mode">
      {sliderNames.map((name) => {
        const mapping = findMapping(name)!
        const isMapped = !!mapping
        const isMapping = currentlyMapping === name
        let text = ''

        if (!isMapping && !isMapped) {
          text = 'MAP'
        } else if (isMapping && !isMapped) {
          text = '...'
        } else {
          text = mapping[1].join('/')
        }

        return (
          <React.Fragment key={name}>
            <label>{isMapped ? <b>{name}</b> : name}</label>
            <span style={{ display: 'inline-flex' }}>
              <button
                className={
                  isMapping
                    ? 'map-button mapping'
                    : isMapped
                    ? 'map-button'
                    : 'map-button inactive'
                }
                onClick={() => {
                  onClickMap(name)
                }}
              >
                {text}
              </button>
              {isMapped && (
                <IconButton
                  name="Close"
                  onClick={() => {
                    onRemoveMapping(name)
                    clearCurrentlyMapping()
                  }}
                />
              )}
            </span>
          </React.Fragment>
        )
      })}
    </div>
  )
}
