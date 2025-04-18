import React, { useState, useEffect } from 'react'
import Context, {
  LocalSettings,
  ContextProps,
  defaultSettings,
} from './Context'

const LOCAL_STORAGE_KEY = 'xtal.localSettings'

function getStoredSettings(): LocalSettings {
  try {
    const storedSettings = localStorage.getItem(LOCAL_STORAGE_KEY)

    if (storedSettings) {
      return {
        ...defaultSettings,
        ...JSON.parse(storedSettings),
      }
    }
  } catch (error) {
    console.error('Error loading local settings:', error)
  }

  return {
    ...defaultSettings,
  }
}

export default function Provider({ children }: { children: React.ReactNode }) {
  const [localSettings, setLocalSettings] = useState<LocalSettings>(
    getStoredSettings()
  )

  useEffect(() => {
    document.documentElement.style.fontSize = `${localSettings.fontSize}px`
  }, [localSettings.fontSize])

  const updateLocalSettings = (patch: Partial<LocalSettings>) => {
    const updated = {
      ...localSettings,
      ...patch,
    }
    setLocalSettings(updated)
    localStorage.setItem(LOCAL_STORAGE_KEY, JSON.stringify(updated))
  }

  const contextValue: ContextProps = {
    localSettings,
    updateLocalSettings,
  }

  return <Context.Provider value={contextValue}>{children}</Context.Provider>
}
