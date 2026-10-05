'use client'
import { useT } from '@/lib/i18n'
import Link from 'next/link'
import { useState } from 'react'
import { call } from '@/lib/rpc'
import { useAsync, useFields } from '@/lib/hooks'
import { display, listColumns, type Domain, type Rec } from '@/lib/model'
import { combine, dateFilters, DATE_PERIODS, listCols, periodDomain, searchFilters, type Node, type Period } from '@/lib/arch'
import { rpc } from '@/lib/rpc'
import { robustSearchRead } from '@/lib/search'

const PAGE = 40
export const Pill = ({ v }: { v: string }) => <span className={`pill ${v.replace(/[^a-z_]/g, '_')}`}>{v.replace(/_/g, ' ')}</span>

export function ListView({ model, title, domain }: { model: string; title: string; domain: Domain }) {
  const { t } = useT()
  const { data: fields, error: fe } = useFields(model)
  const [q, setQ] = useState(''); const [page, setPage] = useState(0); const [order, setOrder] = useState<string | undefined>()
  const view = useAsync(() => call<Node | null>(model, 'get_view', ['list']).catch(() => null), [model])
  const SCALAR = ['char', 'text', 'integer', 'float', 'monetary', 'boolean', 'date', 'datetime', 'selection', 'many2one']
  const archCols = fields && view.data ? listCols(view.data).filter((c) => fields[c]?.store && SCALAR.includes(fields[c].type)) : []
  const search = useAsync(() => call<Node | null>(model, 'get_view', ['search']).catch(() => null), [model])
  const me = useAsync(() => rpc<{ uid: number } | null>({ method: 'whoami' }).catch(() => null), []).data
  const today = new Date().toISOString().slice(0, 10)
  // Offer only filters the server can actually evaluate: every field in the domain must exist and be searchable
  // (computed fields without a `search` implementation would otherwise raise an error when the chip is clicked).
  const usable = (f: { fields: string[] }) => !!fields && f.fields.every((n) => fields[n]?.searchable !== false && n in fields)
  const filters = search.data && fields ? searchFilters(search.data, { uid: me?.uid, today }).filter(usable) : []
  const dfilters = search.data && fields ? dateFilters(search.data).filter((d) => fields[d.field]?.searchable !== false && d.field in fields) : []
  const [on, setOn] = useState<string[]>([]); const [periods, setPeriods] = useState<Record<string, Period | ''>>({})
  const cols = fields ? (archCols.length ? archCols : listColumns(fields)) : []
  const nameField = fields && ('name' in fields ? 'name' : 'display_name' in fields ? 'display_name' : null)
  const pdom = dfilters.flatMap((d) => (periods[d.name] ? periodDomain(d.field, periods[d.name] as Period, today) : []))
  const fdom = [...combine(filters.filter((f) => on.includes(f.name))), ...pdom]
  // top-level siblings are an implicit AND in Odoo domains, so concatenation is exact
  const dom: Domain = [...domain, ...fdom, ...(q && nameField ? [[nameField, 'ilike', q]] : [])]
  const rows = useAsync(
    () => (fields ? robustSearchRead<Rec>(call as never, model, { domain: dom, fields: cols, limit: PAGE, offset: page * PAGE, order }) : Promise.resolve(undefined)),
    [model, JSON.stringify(dom), page, order, !!fields, JSON.stringify(cols)],
  )
  const total = rows.data?.total ?? 0
  return (
    <>
      <div className="bar">
        <h1>{title}</h1>
        <Link className="btn p" href={`/form/?model=${model}`}>{t('list.new')}</Link>
        <input placeholder={t('list.search')} value={q} onChange={(e) => { setQ(e.target.value); setPage(0) }} />
        <span className="grow" />
        <span className="pager">{total ? `${page * PAGE + 1}–${Math.min((page + 1) * PAGE, total)} / ${total}` : '0'}
          <button className="btn" disabled={page === 0} onClick={() => setPage(page - 1)}>‹</button>
          <button className="btn" disabled={(page + 1) * PAGE >= total} onClick={() => setPage(page + 1)}>›</button></span>
      </div>
      {(filters.length > 0 || dfilters.length > 0) && (
        <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap', marginBottom: 10 }}>
          {dfilters.map((d) => <select key={d.name} value={periods[d.name] ?? ''} onChange={(e) => { setPage(0); setPeriods({ ...periods, [d.name]: e.target.value as Period | '' }) }} style={{ padding: '2px 8px', borderRadius: 999 }}><option value="">{d.label}</option>{DATE_PERIODS.map((p) => <option key={p} value={p}>{d.label}: {t(`list.period.${p}` as never)}</option>)}</select>)}
          {filters.map((f) => <a key={f.name} className={`pill ${on.includes(f.name) ? 'sale' : ''}`} style={{ cursor: 'pointer' }} onClick={() => { setPage(0); setOn(on.includes(f.name) ? on.filter((x) => x !== f.name) : [...on, f.name]) }}>{f.label}</a>)}
        </div>
      )}
      {!!rows.data?.notes.length && <div className="warn">{t('list.ignored')} {rows.data.notes.join(', ')}</div>}
      {(fe || rows.error) && <div className="err">{fe ?? rows.error}</div>}
      <div className="card tbl">
        <table>
          <thead><tr>{cols.map((c) => <th key={c} onClick={() => setOrder(order === `${c} asc` ? `${c} desc` : `${c} asc`)} style={{ cursor: 'pointer' }}>{fields![c].string}{order?.startsWith(c) ? (order.endsWith('asc') ? ' ▲' : ' ▼') : ''}</th>)}</tr></thead>
          <tbody>
            {rows.data?.recs.map((r) => (
              <tr key={String(r.id)} onClick={() => (location.href = `/form/?model=${model}&id=${r.id}`)}>
                {cols.map((c) => <td key={c}>{fields![c].type === 'selection' && r[c] ? <Pill v={String(r[c])} /> : fields![c].type === 'boolean' ? (r[c] ? '✓' : '') : display(r[c])}</td>)}
              </tr>
            ))}
            {!rows.loading && rows.data?.recs.length === 0 && <tr><td colSpan={cols.length || 1} className="muted">{t('list.empty')}</td></tr>}
          </tbody>
        </table>
      </div>
    </>
  )
}
