'use client'
import { useT } from '@/lib/i18n'
import { call } from '@/lib/rpc'
import { useFields } from '@/lib/hooks'
import { listColumns, type Rec } from '@/lib/model'
import { Field } from './Field'
import { Icon } from '@/components/Icon'

export type LineState = { rows: Rec[]; removed: number[] }
// Inline one2many editor; new rows carry no id and become (0,0,vals), edits become (1,id,vals), removals (2,id).
export function Lines({ comodel, inverse, state, onChange, readonly, columns }: { comodel: string; inverse: string | null; state: LineState; onChange: (s: LineState) => void; readonly: boolean; columns?: string[] }) {
  const { t } = useT()
  const { data: fields } = useFields(comodel)
  if (!fields) return <p className="muted">Loading…</p>
  const cols = columns?.filter((c) => c in fields && c !== inverse).slice(0, 8) ?? listColumns(fields, 9).filter((c) => c !== inverse && !fields[c].readonly || fields[c]?.required).filter((c) => c !== inverse).slice(0, 6)
  const upd = (i: number, c: string, v: unknown) => {
    const next = state.rows.map((r, j) => (j === i ? { ...r, [c]: v, __dirty: true } : r))
    onChange({ ...state, rows: next })
    // server-side onchange (price from product, subtotals...) merged back asynchronously; no-op if the model has none
    const { id: _id, __dirty, ...cur } = next[i] as Rec & { __dirty?: boolean }
    void __dirty; void _id
    call<Record<string, unknown>>(comodel, 'onchange', [cur]).then((patch) => {
      if (patch && Object.keys(patch).length) onChange({ ...state, rows: next.map((r, j) => (j === i ? { ...r, ...patch, __dirty: true } : r)) })
    }).catch(() => undefined)
  }
  return (
    <div className="card tbl">
      <table>
        <thead><tr>{cols.map((c) => <th key={c}>{fields[c].string}</th>)}<th /></tr></thead>
        <tbody>
          {state.rows.map((r, i) => (
            <tr key={String(r.id ?? `n${i}`)} style={{ cursor: 'default' }}>
              {cols.map((c) => <td key={c}><Field name={c} meta={fields[c]} value={r[c]} onChange={(v) => upd(i, c, v)} readonly={readonly || (fields[c].readonly && !fields[c].store)} /></td>)}
              <td>{!readonly && <button className="btn" onClick={() => onChange({ rows: state.rows.filter((_, j) => j !== i), removed: r.id ? [...state.removed, r.id as number] : state.removed })}><Icon name="x" size="var(--icon-md)" /></button>}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {!readonly && <div style={{ padding: 8 }}><button className="btn" onClick={() => onChange({ ...state, rows: [...state.rows, { __dirty: true }] })}>{t('form.add_line')}</button></div>}
    </div>
  )
}

export const lineCommands = (fields: Record<string, { type: string; store: boolean; readonly: boolean }>, s: LineState) => [
  ...s.removed.map((id) => [2, id]),
  ...s.rows.filter((r) => r.__dirty).map((r) => {
    const vals = Object.fromEntries(Object.entries(r).filter(([k]) => k in fields && fields[k].store && !fields[k].readonly && k !== 'id').map(([k, v]) => [k, fields[k].type === 'many2one' ? (Array.isArray(v) ? v[0] : v) : v]))
    return r.id ? [1, r.id, vals] : [0, 0, vals]
  }),
]
