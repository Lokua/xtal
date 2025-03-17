import path from 'path'
import { fileURLToPath } from 'url'

export function latticeRoot() {
  return path.join(getScriptPath(import.meta.url), '..')
}

export function getScriptPath(importMetaUrl) {
  return path.dirname(fileURLToPath(importMetaUrl))
}
