import { readFileSync } from 'node:fs'
import { APPS } from './apps'
import { APP_ICON, ICONS, SEGMENT_ICON, iconForApp } from './icons'
import { CATEGORIES } from './industries'
import { SITES } from './portal'
const eq = (a: unknown, b: unknown, m: string) => { if (JSON.stringify(a) !== JSON.stringify(b)) { console.error('FAIL', m, a, b); process.exit(1) } }
for (const a of APPS) if (!APP_ICON[a.name]) { console.error('no icon for app', a.name); process.exit(1) }
for (const c of CATEGORIES) if (!SEGMENT_ICON[c.name]) { console.error('no icon for segment', c.name); process.exit(1) }
for (const s of SITES) if (!(s.icon in ICONS)) { console.error('unknown site icon', s.key, s.icon); process.exit(1) }
for (const [name, paths] of Object.entries(ICONS)) {
  if (!paths.length) { console.error('empty icon', name); process.exit(1) }
  for (const d of paths) if (!/^[MmLlHhVvCcSsQqTtAaZz0-9eE.,\s-]+$/.test(d) || !/^[Mm]/.test(d)) { console.error('bad path in', name, d); process.exit(1) }
}
for (const n of Object.values({ ...APP_ICON, ...SEGMENT_ICON })) if (!(n in ICONS)) { console.error('mapped to missing icon', n); process.exit(1) }
eq(iconForApp('Nope'), 'box', 'fallback')
// no emoji or pictographic glyphs anywhere in the UI sources: thin SVG icons only (⌘ is a keyboard-shortcut label, and POS keypad
// keys keep their glyphs as logic tokens but are rendered as icons)
import { readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'
const glyph = /[\u{1F000}-\u{1FAFF}\u{2190}-\u{21FF}\u{2300}-\u{23FF}\u{25A0}-\u{27BF}\u{2B00}-\u{2BFF}\u{FE0F}]/u
const walk = (d: string): string[] => readdirSync(d).flatMap((f) => { const p = join(d, f); return statSync(p).isDirectory() ? (f === 'i18n' || f === 'vibe' || f === 'node_modules' ? [] : walk(p)) : /\.(tsx?|css)$/.test(f) && !/\.test\./.test(f) ? [p] : [] })
const root = new URL('..', import.meta.url).pathname
const tokens = /'⌫'|'✓'/   // keypad logic tokens
for (const f of [...walk(join(root, 'app')), ...walk(join(root, 'components')), ...walk(join(root, 'lib'))]) {
  if (f.endsWith('icons.test.ts')) continue
  const hit = readFileSync(f, 'utf8').split('\n').find((l) => glyph.test(l.replace(/⌘/g, '')) && !tokens.test(l))
  if (hit) { console.error('glyph left in', f, hit.trim().slice(0, 90)); process.exit(1) }
}
console.log(`icons: ${Object.keys(ICONS).length} ok`)
