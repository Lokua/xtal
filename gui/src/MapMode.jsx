import React, { useEffect, useState } from 'react'

export default function MapMode({
  sliderNames,
  mappings,
  onRemoveMapping,
  onSetCurrentlyMapping,
}) {
  const [currentlyMapping, setCurrentlyMapping] = useState('')

  useEffect(() => {
    document.addEventListener('click', onOutsideClick)
    document.addEventListener('keydown', onPressEnter)
    return () => {
      document.removeEventListener('click', onOutsideClick)
      document.removeEventListener('keydown', onPressEnter)
    }
  }, [currentlyMapping])

  function onOutsideClick(e) {
    if (currentlyMapping && !e.target.classList.contains('map-button')) {
      clearCurrentlyMapping()
    }
  }

  function onPressEnter(e) {
    if (e.code === 'Enter') {
      clearCurrentlyMapping()
    }
  }

  function findMapping(name) {
    return mappings.find((m) => m[0] === name)
  }

  function clearCurrentlyMapping() {
    setCurrentlyMapping('')
    onSetCurrentlyMapping('')
  }

  function onClickMap(name) {
    if (currentlyMapping !== name) {
      setCurrentlyMapping(name)
      onSetCurrentlyMapping(name)
    }
  }

  return (
    <div className="map-mode">
      {sliderNames.map((name) => {
        const mapping = findMapping(name)
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
            <span>
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
                <button
                  onClick={() => {
                    onRemoveMapping(name)
                    clearCurrentlyMapping()
                  }}
                >
                  &times;
                </button>
              )}
            </span>
          </React.Fragment>
        )
      })}
    </div>
  )
}
