import fs from 'node:fs'
import path from 'path'

main()

function main() {
  const [filename] = process.argv.slice(2)
  const code = fs.readFileSync(`${process.cwd()}/${filename}`, 'utf-8')
  const regex = /#\[uniforms\s*\(\s*count\s*=\s*(\d+)\s*\)\]/
  const match = code.match(regex)
  if (match) {
    const count = parseInt(match[1], 10)
    const inner = Array.from({ length: count })
      .map((_, i) => `    ${String.fromCharCode(i + 97)}: vec4f,`)
      .join('\n')
    console.log(`\n\nstruct Params {\n${inner}\n}\n\n`)
  }
}
