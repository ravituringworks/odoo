export type Node = { tag: string; attrs: Record<string, string>; text: string | null; children: Node[] }
export const walk = (n: Node, f: (n: Node) => void) => { f(n); n.children.forEach((c) => walk(c, f)) }
export const fieldsIn = (n: Node): string[] => { const out = new Set<string>(); walk(n, (x) => { if (x.tag === 'field' && x.attrs.name) out.add(x.attrs.name) }); return [...out] }
/** Columns of a <list> (skipping column_invisible / invisible="1" / optional="hide" ones). */
export const listCols = (list: Node): string[] =>
  list.children.filter((c) => c.tag === 'field' && c.attrs.name && !c.attrs.column_invisible && !c.attrs.invisible && c.attrs.optional !== 'hide' && c.attrs.widget !== 'handle').map((c) => c.attrs.name)

export type Filter = { name: string; label: string; domain: unknown[]; group: number; fields: string[] }
export type DateFilter = { name: string; label: string; field: string }
export type PyVars = { uid?: number; today?: string }

const iso = (d: Date) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
const shift = (today: string, days: number) => { const d = new Date(`${today}T00:00:00`); d.setDate(d.getDate() + days); return iso(d) }

/** Python-literal domain text -> JSON domain. Evaluates `uid`, `context_today()` (+/- timedelta/relativedelta days) and
 * `.strftime('%Y-%m-%d')`; returns null for anything else that needs real Python (the filter is then not offered). */
export function pyDomain(src: string | undefined, vars: PyVars = {}): unknown[] | null {
  if (!src) return null
  const today = vars.today ?? iso(new Date())
  let s = src
  // (context_today() + datetime.timedelta(days=N)).strftime('%Y-%m-%d') and relativedelta(days=N)
  s = s.replace(/\(\s*context_today\(\)\s*([+-])\s*(?:datetime\.)?(?:timedelta|relativedelta)\(\s*days\s*=\s*(-?\d+)\s*\)\s*\)(?:\.strftime\([^)]*\))?/g, (_m, sign, n) => `'${shift(today, (sign === '-' ? -1 : 1) * Number(n))}'`)
  s = s.replace(/context_today\(\)(?:\.strftime\([^)]*\))?/g, `'${today}'`)
  if (/\buid\b/.test(s)) { if (vars.uid === undefined) return null; s = s.replace(/\buid\b/g, String(vars.uid)) }
  let out = ''; let q: string | null = null
  for (const ch of s) {
    if (q) { if (ch === q) { q = null; out += '"' } else out += ch; continue }
    if (ch === "'" || ch === '"') { q = ch; out += '"'; continue }
    out += ch === '(' ? '[' : ch === ')' ? ']' : ch
  }
  out = out.replace(/\bTrue\b/g, 'true').replace(/\bFalse\b/g, 'false').replace(/\bNone\b/g, 'null').replace(/,\s*]/g, ']')
  try { const v = JSON.parse(out); return Array.isArray(v) ? v : null } catch { return null }
}

/** Field names a domain filters on (first path segment). */
export const domainFields = (d: unknown[]): string[] => d.flatMap((x) => (Array.isArray(x) && typeof x[0] === 'string' ? [String(x[0]).split('.')[0]] : []))

/** Filters of a <search> arch; consecutive <filter>s form one OR-group, <separator> starts a new one. Unevaluable domains are dropped. */
export function searchFilters(search: Node, vars: PyVars = {}): Filter[] {
  const out: Filter[] = []; let g = 0
  const visit = (kids: Node[]) => kids.forEach((k) => {
    if (k.tag === 'separator') g++
    else if (k.tag === 'filter' && k.attrs.name && k.attrs.domain && !k.attrs.context?.includes('group_by')) {
      const d = pyDomain(k.attrs.domain, vars); if (d && d.length) out.push({ name: k.attrs.name, label: k.attrs.string ?? k.attrs.name, domain: d, group: g, fields: domainFields(d) })
    } else if (k.tag === 'group') { g++; visit(k.children); g++ }
  })
  visit(search.children); return out
}

/** AND across groups, OR within a group, in Odoo prefix notation. */
export function combine(active: Filter[]): unknown[] {
  const byGroup = new Map<number, Filter[]>(); active.forEach((f) => byGroup.set(f.group, [...(byGroup.get(f.group) ?? []), f]))
  const groups = [...byGroup.values()].map((fs) => {
    const ds = fs.map((f) => f.domain); const ors = Array(Math.max(0, ds.length - 1)).fill('|')
    return [...ors, ...ds.flat()]
  })
  const ands = Array(Math.max(0, groups.length - 1)).fill('&')
  return [...ands, ...groups.flat()]
}

/** `<filter date="date_start">`: period options like Odoo's date filter menu (this/last month, quarter, year). */
export function dateFilters(search: Node): DateFilter[] {
  const out: DateFilter[] = []
  const visit = (kids: Node[]) => kids.forEach((k) => { if (k.tag === 'filter' && k.attrs.date && k.attrs.name) out.push({ name: k.attrs.name, label: k.attrs.string ?? k.attrs.name, field: k.attrs.date }); else if (k.tag === 'group') visit(k.children) })
  visit(search.children); return out
}
export const DATE_PERIODS = ['this_month', 'last_month', 'this_quarter', 'this_year', 'last_365'] as const
export type Period = (typeof DATE_PERIODS)[number]
export function periodDomain(field: string, p: Period, today: string): unknown[] {
  const [y, m] = today.split('-').map(Number); const pad = (n: number) => String(n).padStart(2, '0'); const first = (yy: number, mm: number) => `${yy}-${pad(mm)}-01`
  const ym = (yy: number, mm: number): [number, number] => { const t = yy * 12 + (mm - 1); return [Math.floor(t / 12), (t % 12) + 1] }
  const range = (a: string, b: string) => [[field, '>=', a], [field, '<', b]]
  switch (p) {
    case 'this_month': { const [ny, nm] = ym(y, m + 1); return range(first(y, m), first(ny, nm)) }
    case 'last_month': { const [py, pm] = ym(y, m - 1); return range(first(py, pm), first(y, m)) }
    case 'this_quarter': { const q = Math.floor((m - 1) / 3) * 3 + 1; const [ny, nm] = ym(y, q + 3); return range(first(y, q), first(ny, nm)) }
    case 'this_year': return range(first(y, 1), first(y + 1, 1))
    default: { const d = new Date(`${today}T00:00:00`); d.setDate(d.getDate() - 365); return [[field, '>=', iso(d)]] }
  }
}
