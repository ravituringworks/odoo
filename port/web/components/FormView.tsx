'use client'
import { useT } from '@/lib/i18n'
import { useEffect, useReducer, useState } from 'react'
import { call } from '@/lib/rpc'
import { useAsync, useFields } from '@/lib/hooks'
import { BUTTONS, formSections, toVals, type Fields, type Rec } from '@/lib/model'
import { Field } from './Field'
import { Lines, lineCommands, type LineState } from './Lines'
import { ArchForm } from './ArchForm'
import type { Node } from '@/lib/arch'

type State = { draft: Rec; orig: Rec; lines: Record<string, LineState>; error?: string; busy: boolean }
type Act = { t: 'load'; rec: Rec; lines: Record<string, LineState> } | { t: 'set'; k: string; v: unknown } | { t: 'lines'; k: string; s: LineState } | { t: 'err'; e?: string } | { t: 'busy'; b: boolean }
const reduce = (s: State, a: Act): State => {
  switch (a.t) {
    case 'load': return { draft: a.rec, orig: a.rec, lines: a.lines, busy: false }
    case 'set': return { ...s, draft: { ...s.draft, [a.k]: a.v } }
    case 'lines': return { ...s, lines: { ...s.lines, [a.k]: a.s } }
    case 'err': return { ...s, error: a.e, busy: false }
    case 'busy': return { ...s, busy: a.b, error: undefined }
  }
}

export function FormView({ model, id }: { model: string; id?: number }) {
  const { t } = useT()
  const { data: fields, error: fe } = useFields(model)
  const [st, dispatch] = useReducer(reduce, { draft: {}, orig: {}, lines: {}, busy: false })
  const [rev, setRev] = useState(0)
  const { main, x2many: genericX2m } = fields ? formSections(fields) : { main: [], x2many: [] }
  const view = useAsync(() => call<Node | null>(model, 'get_view', ['form']).catch(() => null), [model])
  const arch = view.data ?? null
  const ported = useAsync(() => call<string[]>(model, 'ported').catch(() => []), [model]).data ?? []
  // with an arch, every one2many referenced by the view is edited inline; otherwise fall back to the generated section list
  const x2many = arch && fields ? Object.keys(fields).filter((n) => fields[n].type === 'one2many' && fields[n].relation && JSON.stringify(arch).includes(`"name":"${n}"`)) : genericX2m
  const load = useAsync(async () => {
    if (!fields) return
    const lines: Record<string, LineState> = {}
    let rec: Rec = {}
    if (id) {
      rec = (await call<Rec[]>(model, 'read', [[id]]))[0] ?? {}
      for (const x of x2many) { const ids = (rec[x] as number[]) ?? []; lines[x] = { removed: [], rows: ids.length ? await call<Rec[]>(fields[x].relation!, 'read', [ids]) : [] } }
    } else {
      for (const x of x2many) lines[x] = { rows: [], removed: [] }
      rec = await call<Rec>(model, 'default_get', [[]]).catch(() => ({}))   // new record: static + ambient defaults
    }
    dispatch({ t: 'load', rec, lines })
  }, [model, id, !!fields, rev])
  useEffect(() => { if (load.error) dispatch({ t: 'err', e: load.error }) }, [load.error])

  const save = async (): Promise<number | undefined> => {
    if (!fields) return
    dispatch({ t: 'busy', b: true })
    try {
      const vals: Rec = toVals(fields, st.draft, st.orig)
      for (const x of x2many) { const c = lineCommands(await childFields(fields, x), st.lines[x]); if (c.length) vals[x] = c }
      const rid = id ?? (await call<number>(model, 'create', [vals]))
      if (id && Object.keys(vals).length) await call(model, 'write', [[id], vals])
      if (!id) { location.replace(`/form/?model=${model}&id=${rid}`); return rid }
      setRev((r) => r + 1); return rid
    } catch (e) { dispatch({ t: 'err', e: (e as Error).message }) }
  }
  const act = async (method: string) => {
    dispatch({ t: 'busy', b: true })
    try { await call(model, method, [[id]]); setRev((r) => r + 1) } catch (e) { dispatch({ t: 'err', e: (e as Error).message }) }
  }
  const dirty = !!fields && (Object.keys(toVals(fields, st.draft, st.orig)).length > 0 || Object.values(st.lines).some((l) => l.removed.length || l.rows.some((r) => r.__dirty)))
  const buttons = (BUTTONS[model] ?? []).filter((b) => id && b.when(st.draft))
  const stateMeta = fields?.state
  // Discard: drop unsaved edits (existing record: reload from server; new record: leave the form)
  const discard = () => { if (id) setRev((r) => r + 1); else if (history.length > 1) history.back(); else location.href = '/' }
  const discardBtn = (dirty || !id) ? <button className="btn" disabled={st.busy} onClick={discard}>{id ? t('form.discard') : t('form.cancel')}</button> : null
  // generic BaseModel actions available on every saved record
  const hasActive = !!fields && fields.active?.store
  const duplicate = async () => { dispatch({ t: 'busy', b: true }); try { const [nid] = await call<number[]>(model, 'copy', [[id]]); location.href = `/form/?model=${model}&id=${nid}` } catch (e) { dispatch({ t: 'err', e: (e as Error).message }) } }
  const generic = id ? (<>
    <button className="btn" disabled={st.busy || dirty} onClick={duplicate}>{t('form.duplicate')}</button>
    {hasActive && <button className="btn" disabled={st.busy || dirty} onClick={() => act(st.draft.active === false ? 'action_unarchive' : 'action_archive')}>{st.draft.active === false ? t('form.unarchive') : t('form.archive')}</button>}
  </>) : null
  // computed counters/flags the server does not provide read as 0/false, like an empty record would
  const neutral: Record<string, unknown> = {}
  if (fields) for (const [n, f] of Object.entries(fields)) if (!(n in st.draft)) { if (['integer', 'float', 'monetary'].includes(f.type)) neutral[n] = 0; else if (f.type === 'boolean') neutral[n] = false }
  const ctx = fields && arch ? { fields, rec: { ...neutral, ...st.draft }, set: (k: string, v: unknown) => dispatch({ t: 'set', k, v }), lines: st.lines, setLines: (k: string, s: LineState) => dispatch({ t: 'lines', k, s }), act, busy: st.busy, dirty, roAll: false, ported } : null
  if (ctx && arch) {
    return (
      <div>
        <div className="bar"><button className="btn p" disabled={!dirty || st.busy} onClick={save}>{t('form.save')}</button>{discardBtn}{generic}</div>
        {(fe || st.error) && <div className="err">{fe ?? st.error}</div>}
        <div className="card form"><ArchForm arch={arch} c={ctx} /></div>
      </div>
    )
  }
  return (
    <div>
      <div className="bar">
        <button className="btn p" disabled={!dirty || st.busy} onClick={save}>{t('form.save')}</button>
        {discardBtn}
        {buttons.map((b) => <button key={b.method} className={`btn ${b.primary ? 'p' : ''}`} disabled={st.busy || dirty} onClick={() => act(b.method)}>{b.label}</button>)}
        <span className="grow" />
        {stateMeta?.type === 'selection' && <div className="status">{stateMeta.selection.map(([k, l]) => <span key={k} className={st.draft.state === k ? 'on' : ''}>{l}</span>)}</div>}
      </div>
      {(fe || st.error) && <div className="err">{fe ?? st.error}</div>}
      <div className="card form">
        <div className="grid">
          {fields && main.map((n) => (
            <div key={n} className={`fld ${fields[n].required ? 'req' : ''}`}>
              <label>{fields[n].string}</label>
              <Field name={n} meta={fields[n]} value={st.draft[n]} onChange={(v) => dispatch({ t: 'set', k: n, v })} readonly={!!id && ['name'].includes(n) && model === 'account.move' ? true : undefined} />
            </div>
          ))}
        </div>
        {fields && x2many.map((x) => (
          <section key={x} style={{ marginTop: 22 }}>
            <h3 style={{ margin: '0 0 8px' }}>{fields[x].string}</h3>
            {st.lines[x] && <Lines comodel={fields[x].relation!} inverse={fields[x].relation_field} state={st.lines[x]} onChange={(s) => dispatch({ t: 'lines', k: x, s })} readonly={!!id && st.draft.state !== undefined && !['draft', 'sent'].includes(String(st.draft.state))} />}
          </section>
        ))}
      </div>
    </div>
  )
}

async function childFields(parent: Fields, x: string) { return (await call<Fields>(parent[x].relation!, 'fields_get')) as Record<string, { type: string; store: boolean; readonly: boolean }> }
