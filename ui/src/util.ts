export function match<T>(
  event: string,
  handlers: Record<string, () => T>
): T | undefined {
  return handlers[event]?.()
}
