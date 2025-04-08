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
  Snapshots,
}

export type ChannelAndController = [number, number]
export type Mappings = [string, ChannelAndController][]
export type Exclusions = string[]

export type Bypassed = Record<string, number>

export type ControlValue = boolean | number | string

export type RawControl = {
  kind: 'Checkbox' | 'DynamicSeparator' | 'Select' | 'Separator' | 'Slider'
  name: string
  value: string
  disabled: boolean
  options: string[]
  min: number
  max: number
  step: number
}

export type Control = Omit<RawControl, 'value'> & {
  value: ControlValue
  // hack to force typescript to see RawControl and Control as different types,
  // otherwise we'd be free to pass RawControl around as if it was a Control -
  // not good
  isRawControl: false
}
