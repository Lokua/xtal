// `navigator.platform` is deprecated but only supported in Chromium
export const isMac = navigator.platform.toLowerCase().includes('mac')

export function setCssBeat(bpm: number) {
  // 2 beats instead of 1 because we are using `alternate` in our CSS animations
  const time = 60_000 / bpm / 2
  console.debug('Setting CSS --beat to', `${time}ms`)
  document.documentElement.style.setProperty('--beat', `${time}ms`)
}
