'use client'
import Link from 'next/link'
import { useCallback, useEffect, useRef, useState } from 'react'
import { rpc } from '@/lib/rpc'
import { useT } from '@/lib/i18n'
import { Icon } from '@/components/Icon'

type Line = { id: number; name: string; qty: number; note?: string; state: 'new' | 'cooking' | 'done'; cancelled?: boolean }
type Ticket = { id: number; name: string; state: string; takeaway?: boolean; note?: string; create_date: string; lines: Line[] }
const act = <T,>(model: string, method: string, ids: number[], kw: Record<string, unknown>) => rpc<T>({ method, model, args: [ids, kw] })
const next: Record<Line['state'], Line['state']> = { new: 'cooking', cooking: 'done', done: 'new' }

/** Kitchen / bar preparation display: polls the register's tickets; tap a line to advance it, bump to serve the ticket. */
export default function Kitchen() {
  const { t } = useT()
  const [config, setConfig] = useState<number | null>(null); const [cats, setCats] = useState<number[]>([])
  const [tickets, setTickets] = useState<Ticket[]>([]); const [err, setErr] = useState(''); const [now, setNow] = useState(Date.now())
  const seen = useRef<Set<number> | null>(null)
  useEffect(() => { const q = new URLSearchParams(location.search); setConfig(Number(q.get('config')) || null); setCats((q.get('cat') ?? '').split(',').map(Number).filter(Boolean)) }, [])
  const beep = () => { try { const a = new AudioContext(); const o = a.createOscillator(); o.frequency.value = 880; o.connect(a.destination); o.start(); o.stop(a.currentTime + 0.15) } catch { /* audio blocked until a gesture */ } }
  const load = useCallback(() => {
    if (!config) return
    act<Ticket[]>('pos.config', 'kitchen_tickets', [config], { category_ids: cats }).then((tk) => {
      if (seen.current && tk.some((x) => !seen.current!.has(x.id))) beep()
      seen.current = new Set(tk.map((x) => x.id)); setTickets(tk); setErr('')
    }).catch((e) => setErr(String(e.message ?? e)))
  }, [config, cats])
  useEffect(() => { load(); const i = setInterval(load, 3000); const c = setInterval(() => setNow(Date.now()), 15000); return () => { clearInterval(i); clearInterval(c) } }, [load])
  const advance = async (l: Line) => { await act('pos_preparation_display.line', 'advance', [l.id], { state: next[l.state] }).catch((e) => setErr(String(e.message ?? e))); load() }
  const bump = async (id: number) => { await act('pos_preparation_display.ticket', 'bump', [id], {}).catch((e) => setErr(String(e.message ?? e))); load() }
  const age = (iso: string) => Math.max(0, Math.round((now - new Date(iso.replace(' ', 'T') + 'Z').getTime()) / 60000))
  if (!config) return <div className="pos-reg"><p className="muted">{t('pos.kds_need_config')}</p><Link href="/pos/" className="btn"><Icon name="arrow-left" size="var(--icon-md)" /></Link></div>
  return (
    <div className="kds">
      <header className="pos-top"><Link href="/pos/" className="btn"><Icon name="arrow-left" size="var(--icon-md)" /></Link><b>{t('pos.kitchen')}</b><span className="muted">{tickets.length} {t('pos.tickets')}</span><span className="grow" /></header>
      {err && <div className="err">{err}</div>}
      <div className="kds-grid">
        {tickets.map((tk) => { const m = age(tk.create_date); return (
          <div key={tk.id} className={`card kds-ticket ${m >= 10 ? 'late' : m >= 5 ? 'warn' : ''}`}>
            <div className="kds-head"><b>{tk.name}{tk.takeaway && <> <Icon name="bag" size="var(--icon-md)" /></>}</b><span className="muted">{m} min</span></div>
            {tk.note && <small><Icon name="file-text" size="var(--icon-sm)" /> {tk.note}</small>}
            {tk.lines.map((l) => <button key={l.id} className={`kds-line ${l.state} ${l.cancelled ? 'cancelled' : ''}`} onClick={() => advance(l)}><span>{l.cancelled && <><Icon name="x" size="var(--icon-sm)" /> </>}{l.qty} × {l.name}</span>{l.note && <small>{l.note}</small>}<small className="muted">{l.state === 'new' ? t('pos.kds_new') : l.state === 'cooking' ? t('pos.kds_cooking') : t('pos.kds_ready')}</small></button>)}
            <button className="btn p" onClick={() => bump(tk.id)}><Icon name="check" size="var(--icon-md)" /> {t('pos.kds_serve')}</button>
          </div>) })}
        {tickets.length === 0 && <p className="muted" style={{ padding: 30 }}>{t('pos.kds_empty')}</p>}
      </div>
    </div>
  )
}
