declare global {
  interface Window {
    ipc: {
      postMessage(message: string): void
    }
  }
}

export type noop = () => void

export enum View {
  Controls,
  Midi,
}

export type ChannelAndController = [number, number]

export type Control = Checkbox | DynamicSeparator | Select | Separator | Slider

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
