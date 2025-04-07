declare global {
  interface Window {
    ipc: {
      postMessage(message: string): void
    }
  }
}

export type noop = () => void

export type Override<T, U> = Omit<T, keyof U> & U

export enum View {
  Controls,
  Default,
  Exclusions,
  Settings,
}

/**
 * Similar to GlobalSettings on the backend yet only needed on the frontend or
 * as a transitive parameter on the backend
 */
export type LocalSettings = {
  randomizationIncludesCheckboxes: boolean
  randomizationIncludesSelects: boolean
}

export type ChannelAndController = [number, number]
export type Mappings = [string, ChannelAndController][]

export type Bypassed = Record<string, number>

export type Control = Checkbox | DynamicSeparator | Select | Separator | Slider
export type ControlWithValue = Checkbox | Select | Slider
export type ControlValue = boolean | number | string

// The awkward structure with a single key is due to serde->bin-code
// limitations. It does not allow tagged enums, so we're kind of stuck with this
// shitty structure unless we want to add yet another layer of ETL
export type Checkbox = {
  checkbox: {
    name: string
    value: boolean
    disabled: boolean
  }
}

export type DynamicSeparator = {
  dynamicSeparator: {
    name: string
  }
}

export type Select = {
  select: {
    name: string
    value: string
    options: string[]
    disabled: boolean
  }
}

export type Separator = {
  // eslint-disable-next-line @typescript-eslint/no-empty-object-type
  separator: {}
}

export type Slider = {
  slider: {
    name: string
    value: number
    min: number
    max: number
    step: number
    disabled: boolean
  }
}
