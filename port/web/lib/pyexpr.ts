// Tolerant conversion of the Python-literal domains/contexts found in Odoo window actions into JSON values.
export type Env = { active_id?: number; active_ids?: number[]; uid?: number }

/** `[('campaign_id', '=', active_id)]` -> `[["campaign_id","=",5]]`; returns null when it cannot be understood. */
export function pyToJson(src: unknown, env: Env = {}): unknown {
  if (typeof src !== 'string') return src ?? null
  let s = src.trim(); if (!s) return null
  const ids = JSON.stringify(env.active_ids ?? (env.active_id !== undefined ? [env.active_id] : []))
  s = s.replace(/\bactive_ids\b/g, ids).replace(/\bactive_id\b/g, env.active_id === undefined ? 'null' : String(env.active_id)).replace(/\buid\b/g, env.uid === undefined ? 'null' : String(env.uid))
  s = s.replace(/\bTrue\b/g, 'true').replace(/\bFalse\b/g, 'false').replace(/\bNone\b/g, 'null')
  // quotes: 'abc' -> "abc" (without touching apostrophes inside double-quoted strings), tuples -> arrays
  let out = ''; let q: string | null = null
  for (let i = 0; i < s.length; i++) {
    const ch = s[i]
    if (q) { if (ch === '\\') { out += ch + (s[++i] ?? ''); continue } if (ch === q) { q = null; out += '"'; continue } out += ch === '"' && q === "'" ? '\\"' : ch; continue }
    if (ch === "'" || ch === '"') { q = ch; out += '"'; continue }
    out += ch === '(' ? '[' : ch === ')' ? ']' : ch
  }
  out = out.replace(/,\s*([\]}])/g, '$1')                    // trailing commas
  try { return JSON.parse(out) } catch { return null }
}

/** Extract the xmlid from a button name like `%(mass_mailing.action_x)d`. */
export const actionRef = (name: string | undefined): string | null => { const m = name?.match(/^%\(([\w.]+)\)d$/); return m ? m[1] : null }
