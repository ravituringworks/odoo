'use client'
import { useEffect, useState } from 'react'
import { call } from '@/lib/rpc'
import { display, m2oId, type FieldMeta } from '@/lib/model'
import { resolveTyped } from '@/lib/m2o'
import { Icon } from '@/components/Icon'

type P = { name: string; meta: FieldMeta; value: unknown; onChange: (v: unknown) => void; readonly?: boolean; placeholder?: string }

function M2O({ meta, value, onChange, readonly }: Omit<P, 'name'>) {
  const [q, setQ] = useState(display(value)); const [opts, setOpts] = useState<[number, string][]>([]); const [open, setOpen] = useState(false)
  useEffect(() => setQ(display(value)), [value])
  useEffect(() => {
    if (!open || !meta.relation) return
    const t = setTimeout(() => call<[number, string][]>(meta.relation!, 'name_search', [q === display(value) ? '' : q], { limit: 8 }).then(setOpts).catch(() => setOpts([])), 150)
    return () => clearTimeout(t)
  }, [q, open, meta.relation, value])
  // typed text is not a value: on leaving the field it must resolve to one record, otherwise it reverts to what is selected
  const settle = () => {
    if (q === display(value) || !meta.relation) return
    if (!q.trim()) { setQ(''); return }
    call<[number, string][]>(meta.relation, 'name_search', [q], { limit: 5 }).then((o) => { const hit = resolveTyped(q, o); if (hit) { onChange(hit); setQ(hit[1]) } else setQ(display(value)) }).catch(() => setQ(display(value)))
  }
  if (readonly) return <div className="ro">{display(value)}</div>
  return (
    <div style={{ position: 'relative' }}>
      <input style={{ width: '100%' }} value={q} onFocus={() => setOpen(true)} onBlur={() => setTimeout(() => { setOpen(false); settle() }, 150)} onChange={(e) => { setQ(e.target.value); if (!e.target.value) onChange(false) }} />
      {open && opts.length > 0 && (
        <div className="card" style={{ position: 'absolute', zIndex: 5, left: 0, right: 0, top: '100%', maxHeight: 220, overflow: 'auto' }}>
          {opts.map(([id, name]) => <div key={id} style={{ padding: '6px 10px', cursor: 'pointer' }} onMouseDown={() => { onChange([id, name]); setQ(name); setOpen(false) }}>{name}</div>)}
        </div>
      )}
    </div>
  )
}

/** many2many as removable tags with a name-search picker. Value = list of ids; names are looked up from the comodel. */
function M2M({ meta, value, onChange, readonly }: Omit<P, 'name'>) {
  const ids: number[] = Array.isArray(value) ? (value as unknown[]).filter((x): x is number => typeof x === 'number') : []
  const [names, setNames] = useState<Record<number, string>>({}); const [q, setQ] = useState(''); const [opts, setOpts] = useState<[number, string][]>([]); const [open, setOpen] = useState(false)
  const key = ids.join(',')
  useEffect(() => {
    const need = ids.filter((i) => !(i in names)); if (!need.length || !meta.relation) return
    call<{ id: number; name?: string; display_name?: string }[]>(meta.relation, 'read', [need, ['name']]).then((r) => setNames((o) => ({ ...o, ...Object.fromEntries(r.map((x) => [x.id, x.name ?? x.display_name ?? `#${x.id}`])) }))).catch(() => setNames((o) => ({ ...o, ...Object.fromEntries(need.map((i) => [i, `#${i}`])) })))
  }, [key]) // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    if (!open || !meta.relation) return
    const t = setTimeout(() => call<[number, string][]>(meta.relation!, 'name_search', [q], { limit: 8 }).then((o) => setOpts(o.filter(([i]) => !ids.includes(i)))).catch(() => setOpts([])), 150)
    return () => clearTimeout(t)
  }, [q, open, key]) // eslint-disable-line react-hooks/exhaustive-deps
  const chips = ids.map((i) => <span key={i} className="pill tag">{names[i] ?? '…'}{!readonly && <button type="button" className="tag-x" onClick={() => onChange(ids.filter((x) => x !== i))}>×</button>}</span>)
  if (readonly) return <div className="ro tags">{chips}</div>
  return (
    <div style={{ position: 'relative' }}><div className="tags m2m">{chips}<input value={q} placeholder="+" onFocus={() => setOpen(true)} onBlur={() => setTimeout(() => setOpen(false), 150)} onChange={(e) => setQ(e.target.value)} /></div>
      {open && opts.length > 0 && <div className="card" style={{ position: 'absolute', zIndex: 5, left: 0, right: 0, top: '100%', maxHeight: 220, overflow: 'auto' }}>{opts.map(([id, name]) => <div key={id} style={{ padding: '6px 10px', cursor: 'pointer' }} onMouseDown={() => { onChange([...ids, id]); setNames((o) => ({ ...o, [id]: name })); setQ('') }}>{name}</div>)}</div>}
    </div>
  )
}

export function Field({ name, meta, value, onChange, readonly, placeholder }: P) {
  const ro = readonly ?? meta.readonly
  const set = onChange
  if (meta.type === 'many2many' && meta.relation) return <M2M meta={meta} value={value} onChange={set} readonly={ro} />
  if (meta.type === 'many2one') return <M2O meta={meta} value={value} onChange={set} readonly={ro} />
  if (ro) return <div className="ro">{meta.type === 'boolean' ? (value ? <Icon name="check" size="var(--icon-md)" /> : '') : meta.type === 'selection' ? (meta.selection.find(([k]) => k === value)?.[1] ?? display(value)) : display(value)}</div>
  switch (meta.type) {
    case 'boolean': return <input type="checkbox" checked={!!value} onChange={(e) => set(e.target.checked)} style={{ justifySelf: 'start' }} />
    case 'integer': return <input type="number" step="1" value={value === false ? '' : String(value ?? '')} onChange={(e) => set(e.target.value === '' ? false : parseInt(e.target.value, 10))} />
    case 'float': case 'monetary': return <input type="number" step="any" value={value === false ? '' : String(value ?? '')} onChange={(e) => set(e.target.value === '' ? false : parseFloat(e.target.value))} />
    case 'date': return <input type="date" value={value ? String(value).slice(0, 10) : ''} onChange={(e) => set(e.target.value || false)} />
    case 'datetime': return <input type="datetime-local" value={value ? String(value).replace(' ', 'T').slice(0, 16) : ''} onChange={(e) => set(e.target.value ? e.target.value.replace('T', ' ') + ':00' : false)} />
    case 'selection': return <select value={String(value || '')} onChange={(e) => set(e.target.value || false)}><option value="" />{meta.selection.map(([k, l]) => <option key={k} value={k}>{l}</option>)}</select>
    case 'text': case 'html': return <textarea rows={3} placeholder={placeholder} value={String(value || '')} onChange={(e) => set(e.target.value || false)} />
    default: return <input placeholder={placeholder} value={String(value || '')} onChange={(e) => set(e.target.value || false)} />
  }
}
export const m2oValue = m2oId
