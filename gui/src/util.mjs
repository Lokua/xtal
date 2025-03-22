export function postText(text) {
  window.ipc.postMessage(text)
}
