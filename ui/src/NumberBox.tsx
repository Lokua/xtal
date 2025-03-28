/**
 * Adapted from https://github.com/Lokua/number-box which needs some bugfixes
 * and a makeover.
 */
import { Component } from 'react'

type Props = {
  value: number
  min: number
  max: number
  step: number
  decimals: number
  onChange: (value: number) => void
}

type State = {
  value: number
  prevY: number
}

export default class NumberBox extends Component<Props> {
  static defaultProps = {
    value: 0,
    min: 0,
    max: 127,
    step: 1,
    decimals: 5,
  }

  static clamp(n: number, min = 0, max = 1) {
    return n < min ? min : n > max ? max : n
  }

  static isNumeric(value: number | string) {
    return value !== '' && value != null && /^-?\d+\.?\d*$/.test(String(value))
  }

  static roundToDecimal(value: number, decimals: number) {
    const tenTo = Math.pow(10, decimals)

    return Math.round(value * tenTo) / tenTo
  }

  static getDerivedStateFromProps(nextProps: Props, prevState: State) {
    if (nextProps.value !== prevState.value) {
      return {
        value: nextProps.value,
      }
    }

    return null
  }

  state = {
    value: this.props.value,
    prevY: 0,
  }

  setValue(x: number | string) {
    if (this.isEnteringFloatingPoint(x as string)) {
      this.setState({ value: x })
    } else if (NumberBox.isNumeric(x)) {
      const value = this.transformValue(parseFloat(x as string))
      this.setState({ value }, () => {
        this.props.onChange(value)
      })
    } else if (x === '') {
      this.setState({ value: x })
    }
  }

  isEnteringFloatingPoint(value: string) {
    return !!this.props.decimals && /^\d+\.$/.test(value)
  }

  transformValue(value: number) {
    return NumberBox.roundToDecimal(
      NumberBox.clamp(value, this.props.min, this.props.max),
      this.props.decimals
    )
  }

  onChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    this.setValue(e.currentTarget.value)
  }

  onBlur = () => {
    this.setValue(this.state.value)
  }

  calculateSensitivity() {
    const { step } = this.props

    // Base sensitivity inversely proportional to step size
    // Smaller steps need higher sensitivity
    return Math.max(0.2, Math.min(20, 1 / step))
  }

  onMouseDown = (e: React.MouseEvent) => {
    window.addEventListener('mousemove', this.onMouseMove)
    window.addEventListener('mouseup', this.onMouseUp)
    this.setState({ prevY: e.clientY })
  }

  onMouseMove = (e: MouseEvent) => {
    const delta = this.state.prevY - e.clientY
    const sensitivity = this.calculateSensitivity()
    const value = this.state.value + delta * this.props.step * sensitivity
    this.setState({ prevY: e.clientY }, () => {
      this.setValue(value)
    })
  }

  onMouseUp = () => {
    window.removeEventListener('mousemove', this.onMouseMove)
    window.removeEventListener('mouseup', this.onMouseUp)
  }

  onTouchStart = (e: React.TouchEvent) => {
    const touch = e.touches.item(0)
    window.addEventListener('touchmove', this.onTouchMove)
    window.addEventListener('touchend', this.onTouchEnd)
    this.setState({ prevY: touch.clientY })
  }

  onTouchMove = (e: TouchEvent) => {
    const [touch] = e.touches
    const delta = this.state.prevY - touch.clientY
    const sensitivity = this.calculateSensitivity()
    const value = this.state.value + delta * this.props.step * sensitivity
    this.setState({ prevY: touch.clientY }, () => {
      this.setValue(value)
    })
  }

  onTouchEnd = () => {
    window.removeEventListener('touchmove', this.onTouchMove)
    window.removeEventListener('touchend', this.onTouchEnd)
  }

  onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowUp') {
      this.setValue(this.state.value + this.props.step)
      e.preventDefault()
    } else if (e.key === 'ArrowDown') {
      this.setValue(this.state.value - this.props.step)
      e.preventDefault()
    } else if (e.key === 'Enter') {
      this.onBlur()
    }
  }

  render() {
    const { value, min, max, step, decimals, onChange, ...rest } = this.props

    return (
      <input
        className="number-box"
        type="text"
        value={this.state.value}
        onChange={this.onChange}
        onBlur={this.onBlur}
        onMouseDown={this.onMouseDown}
        onMouseUp={this.onMouseUp}
        onKeyDown={this.onKeyDown}
        onTouchStart={this.onTouchStart}
        onTouchEnd={this.onTouchEnd}
        {...rest}
      />
    )
  }
}
