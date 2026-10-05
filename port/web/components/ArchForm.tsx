'use client'
import { rpc } from '@/lib/rpc'
import { actionRef, pyToJson } from '@/lib/pyexpr'
import { useEffect, useState } from 'react'
import type { Fields, Rec } from '@/lib/model'
import { evalExpr } from '@/lib/expr'
import { listCols, type Node } from '@/lib/arch'
import { Field } from './Field'
import { Lines, type LineState } from './Lines'
import { groupsVisible } from '@/lib/m2o'

export type Ctx = {
  fields: Fields; rec: Rec; set: (k: string, v: unknown) => void; lines: Record<string, LineState>; setLines: (k: string, s: LineState) => void
  act: (method: string) => void; busy: boolean; dirty: boolean; roAll: boolean; ported: string[]
}
// punctuation-only text (the parentheses around `( <field/> )`) is decoration: never render it, so hidden fields leave no stray marks
const meaningful = (s: string | null | undefined): s is string => !!s && /[\p{L}\p{N}]/u.test(s)
const hidden = (n: Node, c: Ctx) => evalExpr(n.attrs.invisible, c.rec)
const label = (n: Node, c: Ctx) => n.attrs.string ?? c.fields[n.attrs.name]?.string ?? n.attrs.name

function FieldNode({ n, c, bare }: { n: Node; c: Ctx; bare?: boolean }) {
  const name = n.attrs.name; const meta = c.fields[name]
  if (!meta || hidden(n, c)) return null
  const ro = c.roAll || evalExpr(n.attrs.readonly, c.rec) || meta.readonly
  const req = meta.required || evalExpr(n.attrs.required, c.rec)
  if (meta.type === 'one2many' && meta.relation) {
    const list = n.children.find((x) => x.tag === 'list' || x.tag === 'tree')
    return <Lines comodel={meta.relation} inverse={meta.relation_field} state={c.lines[name] ?? { rows: [], removed: [] }} onChange={(s) => c.setLines(name, s)} readonly={c.roAll || evalExpr(n.attrs.readonly, c.rec)} columns={list ? listCols(list) : undefined} />
  }
  if ((meta.type === 'many2many' && !meta.relation) || meta.type === 'binary' || meta.type === 'image' || meta.type === 'properties') return null
  const input = <Field name={name} meta={meta} value={c.rec[name]} onChange={(v) => c.set(name, v)} readonly={ro} placeholder={n.attrs.placeholder} />
  if (bare || n.attrs.nolabel === '1') return input
  return <div className={`fld ${req ? 'req' : ''}`}><label>{label(n, c)}</label>{input}</div>
}

function Button({ n, c }: { n: Node; c: Ctx }) {
  if (hidden(n, c) || n.attrs.type !== 'object' || !n.attrs.name || /[%(]/.test(n.attrs.name) || !c.ported.includes(n.attrs.name)) return null   // dead buttons (method not ported) are not shown
  const primary = /btn-primary/.test(n.attrs.class ?? '')
  return <button className={`btn ${primary ? 'p' : ''}`} disabled={c.busy || c.dirty} onClick={() => c.act(n.attrs.name)}>{n.attrs.string ?? n.attrs.name}</button>
}

function M2OStatusbar({ n, c }: { n: Node; c: Ctx }) {
  const meta = c.fields[n.attrs.name]; const [opts, setOpts] = useState<[number, string][]>([])
  useEffect(() => { if (meta?.relation) rpc<{ id: number; name: string }[]>({ method: 'search_read', model: meta.relation, args: [[]], kwargs: { fields: ['name'], order: 'sequence, id', limit: 30 } }).then((r) => setOpts(r.map((x) => [x.id, x.name]))).catch(() => setOpts([])) }, [meta?.relation])
  const cur = Array.isArray(c.rec[n.attrs.name]) ? (c.rec[n.attrs.name] as [number, string])[0] : c.rec[n.attrs.name]
  const ro = c.roAll || meta?.readonly
  return <div className="status">{opts.map(([id, l]) => <span key={id} className={cur === id ? 'on' : ''} style={ro ? undefined : { cursor: 'pointer' }} onClick={() => !ro && c.set(n.attrs.name, [id, l])}>{l}</span>)}</div>
}

function Statusbar({ n, c }: { n: Node; c: Ctx }) {
  const meta = c.fields[n.attrs.name]; if (!meta) return null
  if (meta.type === 'many2one') return <M2OStatusbar n={n} c={c} />
  const opts = meta.type === 'selection' ? meta.selection.map(([k, l]) => ({ k, l })) : []
  const visible = n.attrs.statusbar_visible?.split(',')
  return <div className="status">{opts.filter((o) => !visible || visible.includes(o.k) || c.rec[n.attrs.name] === o.k).map((o) => <span key={o.k} className={c.rec[n.attrs.name] === o.k ? 'on' : ''}>{o.l}</span>)}</div>
}

function Header({ n, c }: { n: Node; c: Ctx }) {
  const bar = n.children.find((x) => x.tag === 'field' && x.attrs.widget === 'statusbar')
  return (
    <div className="bar">
      <span className="muted" />
      {n.children.filter((x) => x.tag === 'button').map((b, i) => <Button key={i} n={b} c={c} />)}
      <span className="grow" />
      {bar && <Statusbar n={bar} c={c} />}
    </div>
  )
}

/** Open the window action behind a stat button, filtered to the current record (`active_id` in its domain/context). */
async function openAction(ref: string, id: number) {
  const a = await rpc<{ name?: string; model: string; domain?: string; context?: string }>({ method: 'action_get', args: [ref] }).catch(() => null); if (!a?.model) return
  const who = await rpc<{ uid: number } | null>({ method: 'whoami' }).catch(() => null)
  const env = { active_id: id, uid: who?.uid }
  const q = new URLSearchParams({ model: a.model, title: a.name ?? a.model })
  const dom = pyToJson(a.domain, env); if (Array.isArray(dom) && dom.length) q.set('domain', JSON.stringify(dom))
  const ctx = pyToJson(a.context, env); if (ctx && typeof ctx === 'object') q.set('context', JSON.stringify(ctx))
  location.href = `/list/?${q}`
}
function StatButtons({ n, c }: { n: Node; c: Ctx }) {
  const items = n.children.filter((b) => b.tag === 'button' && !hidden(b, c) && b.children.some((x) => x.tag === 'field' && x.attrs.name in c.fields))   // stat buttons of uninstalled modules reference unknown fields
  return (
    <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', justifyContent: 'flex-end', marginBottom: 12 }}>
      {items.map((b, i) => {
        const f = b.children.find((x) => x.tag === 'field'); const v = f ? c.rec[f.attrs.name] : undefined
        const label = f?.attrs.string ?? b.attrs.string ?? (f ? c.fields[f.attrs.name]?.string : b.attrs.name)
        const ref = actionRef(b.attrs.name); const widget = f?.attrs.widget
        const val = typeof v === 'number' ? (widget === 'monetary' || c.fields[f!.attrs.name]?.type === 'monetary' ? v.toFixed(2) : String(v)) : typeof v === 'string' ? v : null
        return <button key={i} className="btn stat" disabled={!ref || !c.rec.id} title={label} onClick={() => ref && openAction(ref, c.rec.id as number)}>{val !== null ? <b>{val}</b> : null}<span>{label}</span></button>
      })}
    </div>
  )
}

function Group({ n, c }: { n: Node; c: Ctx }) {
  if (hidden(n, c)) return null
  const cols = Number(n.attrs.col ?? 2)
  const kids = n.children.filter((x) => x.tag !== 'attribute')
  const nested = kids.some((k) => k.tag === 'group')
  return (
    <div style={{ margin: '0 0 14px' }}>
      {n.attrs.string && <div className="muted" style={{ fontWeight: 600, margin: '0 0 6px', textTransform: 'uppercase', fontSize: 12 }}>{n.attrs.string}</div>}
      <div className={nested ? 'grid' : ''} style={nested ? { gridTemplateColumns: `repeat(${Math.max(1, cols)}, minmax(0,1fr))` } : { display: 'grid', gap: 10 }}>
        {kids.map((k, i) => <Render key={i} n={k} c={c} />)}
      </div>
    </div>
  )
}

function Notebook({ n, c }: { n: Node; c: Ctx }) {
  const pages = n.children.filter((p) => p.tag === 'page' && p.attrs.string && !hidden(p, c) && groupsVisible(p.attrs.groups))
  const [i, setI] = useState(0)
  if (!pages.length) return null
  const cur = pages[Math.min(i, pages.length - 1)]
  return (
    <div style={{ marginTop: 18 }}>
      <div style={{ display: 'flex', gap: 2, borderBottom: '1px solid var(--line)', marginBottom: 12, flexWrap: 'wrap' }}>
        {pages.map((p, k) => <a key={k} onClick={() => setI(k)} style={{ padding: '6px 14px', cursor: 'pointer', borderBottom: k === i ? '2px solid var(--brand)' : '2px solid transparent', color: k === i ? 'var(--brand)' : 'var(--txt)' }}>{p.attrs.string ?? p.attrs.name}</a>)}
      </div>
      {cur.children.map((k, j) => <Render key={j} n={k} c={c} />)}
    </div>
  )
}

export function Render({ n, c }: { n: Node; c: Ctx }): React.ReactElement | null {
  if (hidden(n, c) || !groupsVisible(n.attrs.groups)) return null
  switch (n.tag) {
    case '#text': return meaningful(n.text) ? <span>{n.text} </span> : null
    case 'header': return <Header n={n} c={c} />
    case 'sheet': case 'form': case 'section': return <>{n.children.map((k, i) => <Render key={i} n={k} c={c} />)}</>
    case 'group': return <Group n={n} c={c} />
    case 'notebook': return <Notebook n={n} c={c} />
    case 'field': return <FieldNode n={n} c={c} />
    case 'label': return n.attrs.for && c.fields[n.attrs.for] ? <div className="muted">{n.attrs.string ?? c.fields[n.attrs.for].string}</div> : null
    case 'separator': return <div style={{ borderBottom: '1px solid var(--line)', margin: '10px 0' }}>{n.attrs.string}</div>
    case 'h1': case 'h2': case 'h3': return <h2 style={{ margin: '4px 0 10px' }}>{n.children.map((k, i) => <Render key={i} n={k} c={c} />)}{meaningful(n.text) ? n.text : null}</h2>
    case 'div':
      if (n.attrs.name === 'button_box') return <StatButtons n={n} c={c} />
      if (/oe_chatter|o_attachment_preview/.test(n.attrs.class ?? '') || n.attrs.name === 'chatter') return null
      return <div>{meaningful(n.text) ? n.text : null}{n.children.map((k, i) => <Render key={i} n={k} c={c} />)}</div>
    case 'span': case 'p': case 'strong': case 'b': return <span>{meaningful(n.text) ? n.text : null}{n.children.map((k, i) => <Render key={i} n={k} c={c} />)}</span>
    case 'button': return <Button n={n} c={c} />
    default: return null   // chatter, widget, kanban-only tags, templates, etc.
  }
}

export function ArchForm({ arch, c }: { arch: Node; c: Ctx }) {
  return <div>{arch.children.map((k, i) => <Render key={i} n={k} c={c} />)}</div>
}
