import fs from 'node:fs'
import path from 'node:path'
import { xtalRoot } from './helpers.mjs'
import { parse } from 'yaml'

main()

function main() {
  try {
    const filePath = path.join(xtalRoot(), process.argv[2])
    const content = fs.readFileSync(filePath, 'utf-8')
    const doc = parse(content)
    const output = Object.entries(doc)
      .map(([key, entry]) => ({
        key,
        ...entry,
      }))
      .filter((item) => item.var)
      .sort((a, b) => a.var.localeCompare(b.var))
      .map((item) => {
        const [bank, slotNumber] = item.var.split('')
        const slot = ['x', 'y', 'z', 'w'][parseInt(slotNumber, 10) - 1]
        return `    let ${item.key} = params.${bank}.${slot};`
      })
      .join('\n')
    console.log(output)
  } catch (error) {
    console.error(error)
    process.exit(1)
  }
}
