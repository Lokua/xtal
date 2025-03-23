export function post(event, data = null) {
  window.ipc.postMessage(
    JSON.stringify({
      event,
      data,
    })
  )
}
