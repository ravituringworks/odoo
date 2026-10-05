'use client'
import Link from 'next/link'
import { useCallback, useEffect, useRef, useState } from 'react'
import { call, rpc } from '@/lib/rpc'
import { useT } from '@/lib/i18n'
import { Markdown } from '@/lib/md'
import { Icon } from '@/components/Icon'

type Proposal = { id: string; kind: 'create' | 'write' | 'call' | 'install'; model: string; ids?: number[]; values?: Record<string, unknown>; method?: string; summary: string; state?: 'applied' | 'rejected' | 'error'; error?: string }
type Trace = { tool: string; ok: boolean; error?: string | null }
type Msg = { role: 'user' | 'assistant'; content: string; trace?: Trace[]; proposals?: Proposal[]; error?: boolean }
const KEY = 'odoo-rs-ai-chat'
export const AI_TOGGLE = 'odoo-rs-ai-toggle'

const load = (): Msg[] => { try { return JSON.parse(localStorage.getItem(KEY) || '[]') } catch { return [] } }
const screenContext = () => { const p = new URLSearchParams(location.search); const model = p.get('model'); return model ? { model, id: p.get('id') ? Number(p.get('id')) : undefined, screen: location.pathname.includes('form') ? 'form' : 'list' } : {} }

export function AiPanel() {
  const { t } = useT()
  const [open, setOpen] = useState(false); const [msgs, setMsgs] = useState<Msg[]>([]); const [input, setInput] = useState(''); const [busy, setBusy] = useState(false)
  const [ready, setReady] = useState<boolean | null>(null); const end = useRef<HTMLDivElement>(null)
  useEffect(() => { setMsgs(load()) }, [])
  useEffect(() => { try { localStorage.setItem(KEY, JSON.stringify(msgs.slice(-40))) } catch { /* ignore */ } end.current?.scrollIntoView({ block: 'end' }) }, [msgs, open])
  useEffect(() => {
    const toggle = () => setOpen((o) => !o)
    const key = (e: KeyboardEvent) => { if ((e.metaKey || e.ctrlKey) && e.key === '/') { e.preventDefault(); toggle() } else if (e.key === 'Escape') setOpen(false) }
    window.addEventListener(AI_TOGGLE, toggle); window.addEventListener('keydown', key)
    return () => { window.removeEventListener(AI_TOGGLE, toggle); window.removeEventListener('keydown', key) }
  }, [])
  useEffect(() => { if (open && ready === null) rpc<{ ai: { enabled: boolean; model: string } }>({ method: 'settings_get' }).then((s) => setReady(s.ai.enabled && !!s.ai.model)).catch(() => setReady(false)) }, [open, ready])

  const send = useCallback(async (text: string) => {
    const q = text.trim(); if (!q || busy) return
    const next: Msg[] = [...msgs, { role: 'user', content: q }]; setMsgs(next); setInput(''); setBusy(true)
    try {
      const r = await rpc<{ reply: string; trace: Trace[]; proposals: Proposal[] }>({ method: 'ai_chat', args: [next.slice(-20).map(({ role, content, proposals }) => ({ role, content: proposals?.length ? `${content}\n\n[Approval cards shown to the user under this reply: ${proposals.map((p) => `${p.summary} — ${p.state ?? 'waiting for approval'}`).join('; ')}]` : content }))], kwargs: { context: screenContext() } })
      setMsgs([...next, { role: 'assistant', content: r.reply, trace: r.trace, proposals: r.proposals }])
    } catch (e) { setMsgs([...next, { role: 'assistant', content: (e as Error).message, error: true }]) } finally { setBusy(false) }
  }, [msgs, busy])

  const decide = async (mi: number, pi: number, approve: boolean) => {
    const upd = (p: Partial<Proposal>) => setMsgs((ms) => ms.map((m, i) => (i === mi ? { ...m, proposals: m.proposals?.map((x, j) => (j === pi ? { ...x, ...p } : x)) } : m)))
    const p = msgs[mi].proposals![pi]
    if (!approve) return upd({ state: 'rejected' })
    try {   // runs with the user's own session: access rules still apply
      if (p.kind === 'install') { await rpc({ method: 'install_module', args: [String(p.values?.module ?? '')] }); upd({ state: 'applied' }); setTimeout(() => location.reload(), 800); return }   // new menus appear after a reload
      if (p.kind === 'create') await call(p.model, 'create', [p.values ?? {}]); else if (p.kind === 'write') await call(p.model, 'write', [p.ids ?? [], p.values ?? {}]); else await call(p.model, p.method ?? '', [p.ids ?? []])
      upd({ state: 'applied' })
    } catch (e) { upd({ state: 'error', error: (e as Error).message }) }
  }

  if (!open) return <button className="btn p fab" onClick={() => setOpen(true)} title="Ctrl/⌘ + /"><Icon name="sparkles" size="var(--icon-md)" /> {t('nav.ai')}</button>
  const sugg = [t('ai.suggest1'), t('ai.suggest2'), t('ai.suggest3')]
  return (
    <aside className="drawer" role="dialog" aria-label={t('ai.title')}>
      <div className="drawer-head"><b><Icon name="sparkles" size="var(--icon-md)" /> {t('ai.title')}</b><span className="grow" /><button className="btn" onClick={() => { setMsgs([]); setReady(null) }}>{t('ai.clear')}</button><button className="btn" onClick={() => setOpen(false)} aria-label={t('common.close')}><Icon name="x" size="var(--icon-md)" /></button></div>
      <div className="drawer-msgs">
        {ready === false && <div className="err">{t('ai.not_configured')} <Link href="/settings/#ai" onClick={() => setOpen(false)}>{t('ai.open_settings')}</Link></div>}
        {msgs.length === 0 && ready !== false && <div style={{ display: 'grid', gap: 6 }}>{sugg.map((s) => <button key={s} className="btn" style={{ textAlign: 'start' }} onClick={() => send(s)}>{s}</button>)}<p className="hint">{t('ai.untrusted')}</p></div>}
        {msgs.map((m, i) => (
          <div key={i} className={`msg ${m.role === 'user' ? 'user' : 'bot'}`} style={m.error ? { borderColor: 'var(--err)', color: 'var(--err)' } : undefined}>
            {m.role === 'user' ? m.content : <Markdown text={m.content} />}
            {m.trace && m.trace.length > 0 && <div className="muted" style={{ fontSize: 11, marginTop: 6 }}>{t('ai.tools_used')}: {m.trace.map((x, j) => <span key={j} className="pill" style={{ marginInlineEnd: 4 }} title={x.error ?? ''}>{x.tool}{x.ok ? null : <> <Icon name="x" size="var(--icon-sm)" /></>}</span>)}</div>}
            {m.proposals?.map((p, j) => (
              <div key={p.id} className="prop">
                <b>{t('ai.proposed')}</b>: {p.summary}
                <pre>{JSON.stringify({ [p.kind]: p.model, ids: p.ids, values: p.values, method: p.method }, null, 1)}</pre>
                {p.state ? <span className={`pill ${p.state === 'applied' ? 'sale' : 'cancel'}`}>{p.state === 'applied' ? t('ai.applied') : p.state === 'rejected' ? t('ai.rejected') : p.error}</span>
                  : <div style={{ display: 'flex', gap: 6 }}><button className="btn p" onClick={() => decide(i, j, true)}>{t('ai.approve')}</button><button className="btn" onClick={() => decide(i, j, false)}>{t('ai.reject')}</button></div>}
              </div>
            ))}
          </div>
        ))}
        {busy && <div className="msg bot muted">{t('ai.thinking')}</div>}
        <div ref={end} />
      </div>
      <div className="drawer-input">
        <textarea rows={1} value={input} placeholder={t('ai.placeholder')} onChange={(e) => setInput(e.target.value)} onKeyDown={(e) => { if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) { e.preventDefault(); send(input) } }} />
        <button className="btn p" disabled={busy || !input.trim()} onClick={() => send(input)}>{t('ai.send')}</button>
      </div>
    </aside>
  )
}
