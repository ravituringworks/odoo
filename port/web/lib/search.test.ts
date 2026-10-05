import { badField, robustSearchRead, type Call } from './search'
const eq = (a: unknown, b: unknown, m: string) => { if (JSON.stringify(a) !== JSON.stringify(b)) { console.error('FAIL', m, JSON.stringify(a), JSON.stringify(b)); process.exit(1) } }
eq(badField('unknown field `start` on model `mrp.production`'), 'start', 'unknown field'); eq(badField('invalid domain: field crm.lead.message_needaction is not stored; cannot search'), 'message_needaction', 'not stored'); eq(badField('boom'), null, 'other errors are not swallowed')

;(async () => {
  const seen: unknown[] = []
  const fake: Call = async (_m, method, args, kw) => {
    const dom = JSON.stringify((kw?.domain ?? args[0]) ?? []); seen.push([method, dom, kw?.fields, kw?.order])
    if (dom.includes('department_id')) throw new Error('unknown field `department_id` on model `hr.expense`')
    if ((kw?.fields as string[] | undefined)?.includes('start')) throw new Error('unknown field `start` on model `mrp.production`')
    if (kw?.order === 'bogus asc') throw new Error('unknown field `bogus` on model `x`')
    return method === 'search_count' ? 3 : [{ id: 1 }]
  }
  // a bad domain leaf is neutralised (structure preserved), a bad column dropped, a bad sort key dropped
  const r = await robustSearchRead<{ id: number }>(fake, 'hr.expense', { domain: ['|', ['department_id', '=', 1], ['state', '=', 'draft']], fields: ['name', 'start'], order: 'bogus asc', limit: 40, offset: 0 })
  eq(r.total, 3, 'total'); eq(r.recs, [{ id: 1 }], 'recs'); eq([...r.notes].sort(), ['bogus (sort)', 'department_id (filter)', 'start (column)'], 'notes')
  const last = seen.filter((s) => (s as string[])[0] === 'search_read').pop() as unknown[]
  eq(last[1], JSON.stringify(['|', ['1', '=', 1], ['state', '=', 'draft']]), 'prefix structure intact'); eq(last[2], ['name'], 'column removed')
  // unrelated errors still surface
  let threw = false; try { await robustSearchRead(async () => { throw new Error('access denied') }, 'm', { domain: [], fields: [], limit: 1, offset: 0 }) } catch { threw = true }
  eq(threw, true, 'real errors propagate')
  // an unfixable unknown field (not ours to remove) propagates instead of looping forever
  let threw2 = false; try { await robustSearchRead(async () => { throw new Error('unknown field `zzz` on model `m`') }, 'm', { domain: [], fields: ['a'], limit: 1, offset: 0 }) } catch { threw2 = true }
  eq(threw2, true, 'unfixable propagates'); console.log('search: ok')
})()
