export function post(event, data) {
  if (data !== undefined) {
    window.ipc.postMessage(
      JSON.stringify({
        [event]: data,
      }),
    )
  } else {
    window.ipc.postMessage(JSON.stringify(event))
  }
}

export function match(condition, lookup) {
  return lookup[condition]?.()
}
