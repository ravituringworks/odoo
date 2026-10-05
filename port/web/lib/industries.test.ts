import { readFileSync } from 'node:fs'
import { APPS } from './apps'
import { CATEGORIES, INDUSTRIES, industriesByCategory, industryBySlug, searchIndustries, trialApps } from './industries'
const eq = (a: unknown, b: unknown, m: string) => { if (JSON.stringify(a) !== JSON.stringify(b)) { console.error('FAIL', m, a, b); process.exit(1) } }

// the app landing content must mirror the server's trial catalog (names, categories, Community vs Enterprise)
const rs = readFileSync(new URL('../crates/odoo-app/src/trial.rs'.replace('../', '../../'), import.meta.url), 'utf8')
const server = [...rs.matchAll(/a\("([^"]+)", "([^"]+)", (Some|None)\(?/g)].map((m) => `${m[1]}|${m[2]}|${m[3] === 'Some'}`)
eq(APPS.map((a) => `${a.name}|${a.category}|${a.community}`), server, 'apps match trial catalog')
eq(new Set(APPS.map((a) => a.slug)).size, APPS.length, 'app slugs unique')
for (const a of APPS) { if (a.features.length < 3 || !a.tagline) { console.error('thin app', a.name); process.exit(1) } }

eq(new Set(INDUSTRIES.map((i) => i.slug)).size, INDUSTRIES.length, 'industry slugs unique')
for (const i of INDUSTRIES) {
  if (!CATEGORIES.some((c) => c.name === i.category)) { console.error('unknown category', i.slug, i.category); process.exit(1) }
  for (const n of i.apps) if (!APPS.some((a) => a.name === n)) { console.error('unknown app', i.slug, n); process.exit(1) }
  if (trialApps(i).length === 0) { console.error('nothing installable', i.slug); process.exit(1) }
}
eq(industriesByCategory().reduce((n, [, l]) => n + l.length, 0), INDUSTRIES.length, 'every industry has a category')
eq(industryBySlug('restaurant')?.apps.includes('Restaurant'), true, 'restaurant uses the Restaurant app')
eq(searchIndustries('wine').map((i) => i.slug), ['vineyard', 'wine-merchant'], 'search')
console.log(`industries: ${INDUSTRIES.length} industries, ${APPS.length} apps ok`)
import { preselect } from './trial'
const cat = APPS.map((a) => ({ name: a.name, category: a.category, icon: '', available: a.community }))
eq(preselect('?industry=restaurant', cat)?.apps, ['Restaurant', 'Point of Sale', 'Inventory', 'Purchase', 'Employees', 'Accounting', 'Website'], 'industry preselects its apps')
eq(preselect('?app=crm', cat), { label: 'CRM', apps: ['CRM'] }, 'app preselects itself')
eq(preselect('?app=sign', cat), null, 'enterprise-only apps are not preselected')
eq(preselect('?industry=nope', cat), null, 'unknown slug')
console.log('preselect ok')
