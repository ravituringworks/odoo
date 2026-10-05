// Resilient list loading: an unsupported field must never blank a whole screen. On an "unknown field / not stored" error
// the offending piece (a column, the sort key, or a domain leaf) is neutralised, the request is retried, and what was
// skipped is reported so it can be shown to the user (and fixed upstream).
export type Call = (model: string, method: string, args: unknown[], kwargs?: Record<string, unknown>) => Promise<unknown>
export type Req = { domain: unknown[]; fields: string[]; order?: string; limit: number; offset: number }
export type Result<R> = { recs: R[]; total: number; notes: string[] }

const TRUE_LEAF = ['1', '=', 1]   // Odoo's always-true leaf; keeps prefix-notation structure valid when a leaf is dropped
export const badField = (msg: string): string | null => /unknown field `([\w.]+)`/.exec(msg)?.[1] ?? /field [\w.]+?\.(\w+) is not stored/.exec(msg)?.[1] ?? null

const neutralise = (domain: unknown[], name: string): { domain: unknown[]; hit: boolean } => {
  let hit = false
  const out = domain.map((t) => {
    if (Array.isArray(t) && typeof t[0] === 'string' && t[0].split('.').includes(name)) { hit = true; return TRUE_LEAF }
    return t
  })
  return { domain: out, hit }
}

export async function robustSearchRead<R>(call: Call, model: string, req: Req): Promise<Result<R>> {
  let { domain, fields, order } = req; const notes: string[] = []
  for (let attempt = 0; attempt < 6; attempt++) {
    try {
      const recs = (await call(model, 'search_read', [], { domain, fields, limit: req.limit, offset: req.offset, order })) as R[]
      const total = (await call(model, 'search_count', [domain])) as number
      return { recs, total, notes }
    } catch (e) {
      const name = badField((e as Error).message ?? ''); if (!name) throw e
      if (fields.includes(name)) { fields = fields.filter((f) => f !== name); notes.push(`${name} (column)`) }
      else if (order && order.split(/[ ,]+/).includes(name)) { order = undefined; notes.push(`${name} (sort)`) }
      else { const r = neutralise(domain, name); if (!r.hit) throw e; domain = r.domain; notes.push(`${name} (filter)`) }
    }
  }
  throw new Error('too many unsupported fields in this view')
}
