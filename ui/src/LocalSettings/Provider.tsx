import React, { useState, useEffect } from 'react'
import Context, {
  getStoredSettings,
  LOCAL_STORAGE_KEY,
  LocalSettings,
  ContextProps,
} from './Context'

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
