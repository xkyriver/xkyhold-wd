// Fails when a string shown by the window has no pt-PT translation, or when
// the translation keeps a placeholder out or adds one. Strings are found as
// the first argument of t() / translate() and in tables marked i18n-keys.
import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'

const root = new URL('../src/', import.meta.url).pathname
const files = []
const walk = (dir) => {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name)
    if (statSync(path).isDirectory()) walk(path)
    else if (/\.(svelte|ts)$/.test(name) && !name.startsWith('i18n.pt-PT')) files.push(path)
  }
}
walk(root)

const dict = readFileSync(join(root, 'lib/i18n.pt-PT.ts'), 'utf8')
const entries = new Map()
for (const m of dict.matchAll(/^\s*(?:'((?:[^'\\]|\\.)*)'|(\w+)):\s*\n?\s*'((?:[^'\\]|\\.)*)',/gm)) {
  entries.set((m[1] ?? m[2]).replace(/\\'/g, "'"), m[3].replace(/\\'/g, "'"))
}

const keys = new Set()
for (const file of files) {
  const text = readFileSync(file, 'utf8')
  for (const m of text.matchAll(/\b(?:t|translate\(\s*\w+\s*,)\(?\s*'((?:[^'\\]|\\.)*)'/g)) keys.add(m[1].replace(/\\'/g, "'"))
  // Labels kept in tables and translated later with t(label).
  for (const block of text.matchAll(/\/\/ i18n-keys\n([\s\S]*?)\n\s*\}/g)) {
    // Objects with a `label` only translate the label; plain maps translate every value.
    const pattern = /\blabel:/.test(block[1]) ? /\blabel:\s*'((?:[^'\\]|\\.)*)'/g : /:\s*'((?:[^'\\]|\\.)*)'/g
    for (const m of block[1].matchAll(pattern)) keys.add(m[1].replace(/\\'/g, "'"))
  }
}

const vars = (s) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort().join(',')
const problems = []
for (const key of keys) {
  if (!entries.has(key)) problems.push(`missing: ${key}`)
  else if (vars(key) !== vars(entries.get(key))) problems.push(`placeholders differ: ${key}`)
  else if (/\bvocê\b/i.test(entries.get(key))) problems.push(`"você" in: ${key}`)
}
if (problems.length) {
  console.error(problems.join('\n'))
  process.exit(1)
}
console.log(`i18n: ${keys.size} strings, all translated to pt-PT`)
