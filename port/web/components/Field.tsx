'use client'
import { useEffect, useState } from 'react'
import { call } from '@/lib/rpc'
import { display, m2oId, type FieldMeta } from '@/lib/model'

type P = { name: string; meta: FieldMeta; value: unknown; onChange: (v: unknown) => void; readonly?: boolean; placeholder?: string }

function M2O({ meta, value, onChange, readonly }: Omit<P, 'name'>) {
  const [q, setQ] = useState(display(value)); const [opts, setOpts] = useState<[number, string][]>([]); const [open, setOpen] = useState(false)
  useEffect(() => setQ(display(value)), [value])
  useEffect(() => {
    if (!open || !meta.relation) return
    const t = setTimeout(() => call<[number, string][]>(meta.relation!, 'name_search', [q === display(value) ? '' : q], { limit: 8 }).then(setOpts).catch(() => setOpts([])), 150)
    return () => clearTimeout(t)
  }, [q, open, meta.relation, value])
  if (readonly) return <div className="ro">{display(value)}</div>
  return (
    <div style={{ position: 'relative' }}>
      <input style={{ width: '100%' }} value={q} onFocus={() => setOpen(true)} onBlur={() => setTimeout(() => setOpen(false), 150)} onChange={(e) => { setQ(e.target.value); if (!e.target.value) onChange(false) }} />
      {open && opts.length > 0 && (
        <div className="card" style={{ position: 'absolute', zIndex: 5, left: 0, right: 0, top: '100%', maxHeight: 220, overflow: 'auto' }}>
          {opts.map(([id, name]) => <div key={id} style={{ padding: '6px 10px', cursor: 'pointer' }} onMouseDown={() => { onChange([id, name]); setQ(name); setOpen(false) }}>{name}</div>)}
        </div>
      )}
    </div>
  )
}

export function Field({ name, meta, value, onChange, readonly, placeholder }: P) {
  const ro = readonly ?? meta.readonly
  const set = onChange
  if (meta.type === 'many2one') return <M2O meta={meta} value={value} onChange={set} readonly={ro} />
  if (ro) return <div className="ro">{meta.type === 'boolean' ? (value ? '✓' : '') : meta.type === 'selection' ? (meta.selection.find(([k]) => k === value)?.[1] ?? display(value)) : display(value)}</div>
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
