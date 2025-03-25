import React, { Component } from 'react'

export default class NumberBox extends Component {
  static defaultProps = {
    value: 0,
    min: 0,
    max: 127,
    step: 1,
    decimals: 5,
  }

  static clamp(n, min = 0, max = 1) {
    return n < min ? min : n > max ? max : n
  }

  static isNumeric(value) {
    return value !== '' && value != null && /^-?\d+\.?\d*$/.test(String(value))
  }

  static roundToDecimal(value, decimals) {
    const tenTo = Math.pow(10, decimals)

    return Math.round(value * tenTo) / tenTo
  }

  static getDerivedStateFromProps(nextProps, prevState) {
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

  setValue(x) {
    if (this.isEnteringFloatingPoint(x)) {
      this.setState({ value: x })
    } else if (NumberBox.isNumeric(x)) {
      const value = this.transformValue(parseFloat(x))

      this.setState({ value }, () => {
        this.props.onChange(value)
      })
    } else if (x === '') {
      this.setState({ value: x })
    }
  }

  isEnteringFloatingPoint(value) {
    return !!this.props.decimals && /^\d+\.$/.test(value)
  }

  transformValue(value) {
    return NumberBox.roundToDecimal(
      NumberBox.clamp(value, this.props.min, this.props.max),
      this.props.decimals
    )
  }

  safeCall(key, ...args) {
    typeof this.props[key] === 'function' && this.props[key](...args)
  }

  onChange = (e) => {
    this.setValue(e.currentTarget.value)
  }

  onBlur = (e) => {
    this.setValue(this.state.value)
    this.safeCall('onBlur', e)
  }

  calculateSensitivity() {
    const { step } = this.props

    // Base sensitivity inversely proportional to step size
    // Smaller steps need higher sensitivity
    return Math.max(0.2, Math.min(20, 1 / step))
  }

  onMouseDown = (e) => {
    window.addEventListener('mousemove', this.onMouseMove)
    window.addEventListener('mouseup', this.onMouseUp)
    this.setState({ prevY: e.clientY })
    this.safeCall('onMouseDown', e)
  }

  onMouseMove = (e) => {
    const delta = this.state.prevY - e.clientY
    const sensitivity = this.calculateSensitivity()
    const value =
      parseFloat(this.state.value) + delta * this.props.step * sensitivity

    this.setState({ prevY: e.clientY }, () => {
      this.setValue(value)
    })

    this.safeCall('onMouseMove', e)
  }

  onMouseUp = (e) => {
    window.removeEventListener('mousemove', this.onMouseMove)
    window.removeEventListener('mouseup', this.onMouseUp)
    this.safeCall('onMouseUp', e)
  }

  onTouchStart = (e) => {
    const [touch] = e.touches
    window.addEventListener('touchmove', this.onTouchMove)
    window.addEventListener('touchend', this.onTouchEnd)
    this.setState({ prevY: touch.clientY })
    this.safeCall('onTouchStart', e)
  }

  onTouchMove = (e) => {
    const [touch] = e.touches
    const delta = this.state.prevY - touch.clientY
    const sensitivity = this.calculateSensitivity()
    const value =
      parseFloat(this.state.value) + delta * this.props.step * sensitivity

    this.setState({ prevY: touch.clientY }, () => {
      this.setValue(value)
    })

    this.safeCall('onTouchMove', e)
  }

  onTouchEnd = (e) => {
    window.removeEventListener('touchmove', this.onTouchMove)
    window.removeEventListener('touchend', this.onTouchEnd)
    this.safeCall('onTouchEnd', e)
  }

  onKeyDown = (e) => {
    if (e.key === 'ArrowUp') {
      this.setValue(this.state.value + this.props.step)
      e.preventDefault()
    } else if (e.key === 'ArrowDown') {
      this.setValue(this.state.value - this.props.step)
      e.preventDefault()
    } else if (e.key === 'Enter') {
      this.onBlur()
    }

    this.safeCall('onKeyDown', e)
  }

  render() {
    const {
      value,
      min,
      max,
      step,
      decimals,
      onChange,
      onBlur,
      onMouseDown,
      onMouseUp,
      onTouchStart,
      onTouchEnd,
      onKeyDown,
      ...rest
    } = this.props

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
