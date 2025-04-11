import { createContext } from 'react'

export type FontSizeChoice = 16 | 17 | 18

/**
 * Similar to GlobalSettings on the backend, yet only stored and known on the
 * frontend and not critical to overall functionality
 */
export type LocalSettings = {
  /** Font size in px used in html tag to scale all rem/em units */
  fontSize: FontSizeChoice
}

export const defaultSettings: LocalSettings = {
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
