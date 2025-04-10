import { createContext } from 'react'

export type FontSizeChoice = 16 | 17 | 18

/**
 * Similar to GlobalSettings on the backend, yet only stored and known on the
 * frontend and not critical to overall functionality
 */
export type LocalSettings = {
  fontSize: FontSizeChoice
}

const defaultSettings: LocalSettings = {
  /** Font size in px used in html tag to scale all rem/em units */
  fontSize: 16,
}

export interface ContextProps {
  localSettings: LocalSettings
  updateLocalSettings: (patch: Partial<LocalSettings>) => void
}

const Context = createContext<ContextProps>({
  localSettings: defaultSettings,
  updateLocalSettings: () => {
    // Placeholder that will be replaced by the actual implementation
  },
})

export default Context

export const LOCAL_STORAGE_KEY = 'lattice.localSettings'

export function getStoredSettings(): LocalSettings {
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
