import fs from 'node:fs'
import path from 'path'

main()

function main() {
  const count = parseInt(process.argv[2], 10)
  const inner = Array.from({ length: count })
    .map((_, i) => `    ${String.fromCharCode(i + 97)}: vec4f,`)
    .join('\n')
  console.log(`\n\nstruct Params {\n${inner}\n}\n\n`)
}
