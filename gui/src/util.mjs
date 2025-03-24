export function post(event) {
  window.ipc.postMessage(event)
}

// export function post(event, data = null) {
//   window.ipc.postMessage(
//     JSON.stringify({
//       event,
//       data:
//         data === null
//           ? null
//           : {
//               [event]: data,
//             },
//     })
//   )
// }
