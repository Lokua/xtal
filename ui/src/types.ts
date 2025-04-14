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

export enum UserDir {
  Images = 'Images',
  UserData = 'UserData',
  Videos = 'Videos',
}

export enum OsDir {
  Cache = 'Cache',
  Config = 'Config',
}

export type ChannelAndController = [number, number]
export type Mappings = {
  [key: string]: ChannelAndController
}
export type Exclusions = string[]

export type Bypassed = Record<string, number>

export type ControlValue = boolean | number | string

export type ControlKind =
  | 'Checkbox'
  | 'DynamicSeparator'
  | 'Select'
  | 'Separator'
  | 'Slider'

export type RawControl = {
  kind: ControlKind
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
