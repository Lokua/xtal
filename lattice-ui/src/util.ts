// `navigator.platform` is deprecated but only supported in Chromium
export const isMac = navigator.platform.toLowerCase().includes('mac')

export function setCssBeat(bpm: number) {
  // 1/8 note beats instead of 1/4 because we are using `alternate` in our CSS
  // animations
  const time = 60_000 / bpm / 2
  console.debug('Setting CSS --beat to', `${time}ms`)
  document.documentElement.style.setProperty('--beat', `${time}ms`)
}

/**
 * Write multiline strings with code-aware indentation without worrying about
 * excess white space before
 */
export function format(s: string): string {
  return s
    .split('\n')
    .filter((s) => s)
    .map((line) => line.trim())
    .join(' ')
}
