import { useState } from 'react'

/**
 * Similar to GlobalSettings on the backend, yet only stored and known on the
 * frontend and not critical to overall functionality
 */
export type LocalSettings = {
  fontSize: 16 | 17 | 18
}

const defaultSettings: LocalSettings = {
  /** Font size in px used in html tag to scale all rem/em units */
  fontSize: 16,
}

const LOCAL_STORAGE_KEY = 'lattice.localSettings'

function getStoredSettings() {
  try {
    const storedSettings = JSON.parse(
      localStorage.getItem(LOCAL_STORAGE_KEY) || ''
    )
    if (storedSettings) {
      return {
        ...defaultSettings,
        ...storedSettings,
      }
    }
  } catch (error) {
    console.error(error)
    return { ...defaultSettings }
  }
}

export default function useLocalSettings() {
  const [localSettings, setLocalSettings] = useState<LocalSettings>(
    getStoredSettings()
  )

  function updateLocalSettings(patch: Partial<LocalSettings>) {
    const updated = {
      ...localSettings,
      ...patch,
    }
    setLocalSettings(updated)
    localStorage.setItem(LOCAL_STORAGE_KEY, JSON.stringify(updated))
  }

  return {
    localSettings,
    updateLocalSettings,
  }
}
