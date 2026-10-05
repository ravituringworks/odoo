// Safe evaluator for Odoo view expressions (invisible / readonly / required): `state != 'draft' and not locked`.
// Recursive descent over a tiny grammar; no eval/Function. Unknown identifiers evaluate to false.
type Tok = { t: 'num' | 'str' | 'id' | 'op' | 'lp' | 'rp' | 'lb' | 'rb' | 'comma'; v: string }
const RE = /\s*(?:(\d+(?:\.\d+)?)|'([^']*)'|"([^"]*)"|([A-Za-z_][\w.]*)|(==|!=|<=|>=|<|>)|(\()|(\))|(\[)|(\])|(,))/y

function lex(src: string): Tok[] | null {
  const out: Tok[] = []; RE.lastIndex = 0; let m: RegExpExecArray | null
  while (RE.lastIndex < src.length && (m = RE.exec(src))) {
    if (m[1] !== undefined) out.push({ t: 'num', v: m[1] }); else if (m[2] !== undefined) out.push({ t: 'str', v: m[2] }); else if (m[3] !== undefined) out.push({ t: 'str', v: m[3] })
    else if (m[4] !== undefined) out.push({ t: 'id', v: m[4] }); else if (m[5]) out.push({ t: 'op', v: m[5] }); else if (m[6]) out.push({ t: 'lp', v: '(' }); else if (m[7]) out.push({ t: 'rp', v: ')' })
    else if (m[8]) out.push({ t: 'lb', v: '[' }); else if (m[9]) out.push({ t: 'rb', v: ']' }); else if (m[10]) out.push({ t: 'comma', v: ',' })
  }
  return src.slice(RE.lastIndex).trim() === '' ? out : null
}

type Val = unknown
const norm = (v: Val): Val => (Array.isArray(v) && v.length === 2 && typeof v[0] === 'number' && typeof v[1] === 'string' ? v[0] : v)  // many2one -> id
const truthy = (v: Val) => (Array.isArray(v) ? v.length > 0 : !!v)

export function evalExpr(src: string | undefined | null, rec: Record<string, unknown>): boolean {
  if (src === undefined || src === null || src === '') return false
  const s = src.trim()
  if (s === '1' || s === 'True' || s === 'true') return true
  if (s === '0' || s === 'False' || s === 'false') return false
  const toks = lex(s); if (!toks) return false
  let i = 0; let failed = false
  const peek = () => toks[i]; const eat = () => toks[i++]
  const isId = (v: string) => peek()?.t === 'id' && peek()!.v === v

  const atom = (): Val => {
    const t = eat(); if (!t) { failed = true; return undefined }
    if (t.t === 'num') return Number(t.v); if (t.t === 'str') return t.v
    if (t.t === 'lp' || t.t === 'lb') {
      const close = t.t === 'lp' ? 'rp' : 'rb'; const items: Val[] = []; let comma = false
      while (peek() && peek()!.t !== close) { items.push(or()); if (peek()?.t === 'comma') { eat(); comma = true } }
      eat(); return t.t === 'lp' && !comma && items.length === 1 ? items[0] : items
    }
    if (t.t === 'id') {
      if (t.v === 'True') return true; if (t.v === 'False' || t.v === 'None') return false
      return norm(rec[t.v])
    }
    return undefined
  }
  const cmp = (): Val => {
    const l = atom()
    if (isId('not') && toks[i + 1]?.v === 'in') { i += 2; const r = atom(); return !(Array.isArray(r) && r.includes(l)) }
    if (isId('in')) { eat(); const r = atom(); return Array.isArray(r) && r.includes(l) }
    if (isId('is')) { eat(); const neg = isId('not'); if (neg) eat(); const r = atom(); const eq = l === r || (!l && !r); return neg ? !eq : eq }
    if (peek()?.t === 'op') {
      const op = eat().v; const r = atom()
      const a = l as never, b = r as never
      switch (op) { case '==': return l === r || (!l && !r && l !== 0 && r !== 0); case '!=': return !(l === r || (!l && !r && l !== 0 && r !== 0)); case '<': return a < b; case '>': return a > b; case '<=': return a <= b; default: return a >= b }
    }
    return l
  }
  const not = (): Val => { if (isId('not')) { eat(); return !truthy(not()) } return cmp() }
  const and = (): Val => { let l = not(); while (isId('and')) { eat(); const r = not(); l = truthy(l) && truthy(r) } return l }
  const or = (): Val => { let l = and(); while (isId('or')) { eat(); const r = and(); l = truthy(l) || truthy(r) } return l }
  try { const r = or(); return !failed && i === toks.length && truthy(r) } catch { return false }
}
