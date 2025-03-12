import fs from 'node:fs'
import path from 'path'
import { fileURLToPath } from 'url'

main()

async function main() {
  try {
    const root = path.resolve(`${getScriptPath(import.meta.url)}/..`)
    const imagesDir = path.join(root, '/images')
    const indexFile = path.join(imagesDir, '_index.json')
    const outputFile = path.join(root, 'index.md')

    const imageFiles = fs.readdirSync(imagesDir).filter(isSupportedImage)
    const imageIndex = JSON.parse(fs.readFileSync(indexFile, 'utf-8'))

    imageIndex.items = imageIndex.items
      .filter((item) => imageFiles.includes(item.filename))
      .toSorted((a, b) => new Date(b.created_at) - new Date(a.created_at))

    let markdown = 'Files sorted from most to least recent\n\n'
    for (const { filename } of imageIndex.items) {
      markdown += `## ${filename}\n\n`
      markdown += `<img src="images/${filename}" alt="${filename}">\n\n`
    }

    fs.writeFileSync(outputFile, markdown, 'utf-8')
    fs.writeFileSync(indexFile, JSON.stringify(imageIndex, null, 2), 'utf-8')

    console.log(`Successfully created ${outputFile}`)
  } catch (error) {
    console.error('Error:', error)
    process.exit(1)
  }
}

function isSupportedImage(filePath) {
  const extension = path.extname(filePath).toLowerCase().substring(1)
  return ['jpg', 'jpeg', 'png', 'gif'].includes(extension)
}

function getScriptPath(importMetaUrl) {
  return path.dirname(fileURLToPath(importMetaUrl))
}
