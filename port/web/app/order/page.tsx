'use client'
import { useCallback, useEffect, useMemo, useState } from 'react'
import { rpc } from '@/lib/rpc'
import { useT } from '@/lib/i18n'
import { uuid } from '@/lib/pos'

type Product = { id: number; name: string; price: number; description?: string; category_ids: number[] }
type Menu = { name: string; mode: 'consultation' | 'mobile' | 'kiosk'; service_mode: 'counter' | 'table'; takeaway: boolean; open: boolean; currency: string; table: number | null; categories: { id: number; name: string; parent_id: unknown }[]; products: Product[] }
type Status = { uuid: string; number: string; total: number; stage: 'received' | 'preparing' | 'ready'; paid: boolean }
type Q = { config: number; token: string; table?: number; db?: string }

const call = <T,>(q: Q, method: string, kw: Record<string, unknown>) => rpc<T>({ method, args: [q.config, { access_token: q.token, table: q.table, ...kw }], token: null, db: q.db ?? null })
const KEY = (c: number) => `kiosk_last_${c}`
const stages = ['received', 'preparing', 'ready'] as const

/** Public self-ordering: QR menu / mobile ordering / kiosk. No login; the register's access token is the credential. */
export default function Order() {
  const { t } = useT()
  const [q, setQ] = useState<Q | null>(null); const [menu, setMenu] = useState<Menu | null>(null); const [err, setErr] = useState('')
  const [cat, setCat] = useState<number | null>(null); const [cart, setCart] = useState<Record<number, { qty: number; note?: string }>>({})
  const [open, setOpen] = useState(false); const [takeaway, setTakeaway] = useState(false); const [stand, setStand] = useState('')
  const [status, setStatus] = useState<Status | null>(null); const [busy, setBusy] = useState(false); const [orderId, setOrderId] = useState(() => `kiosk-${uuid().replace(/-/g, '')}`)
  useEffect(() => {
    const p = new URLSearchParams(location.search); const config = Number(p.get('config')); const token = p.get('token') ?? ''
    if (!config || !token) { setErr(t('kiosk.bad_link')); return }
    const nq: Q = { config, token, table: Number(p.get('table')) || undefined, db: p.get('db') ?? undefined }; setQ(nq)
    call<Menu>(nq, 'kiosk_menu', {}).then(setMenu).catch((e) => setErr(String(e.message ?? e)))
    try { const last = JSON.parse(localStorage.getItem(KEY(config)) ?? 'null') as { uuid: string; at: number } | null; if (last && Date.now() - last.at < 4 * 3600e3) call<Status>(nq, 'kiosk_status', { uuid: last.uuid }).then(setStatus).catch(() => {}) } catch { /* no history */ }
  }, []) // eslint-disable-line react-hooks/exhaustive-deps
  // follow the order until it is ready
  useEffect(() => {
    if (!q || !status || (status.stage === 'ready' && status.paid)) return
    const i = setInterval(() => call<Status>(q, 'kiosk_status', { uuid: status.uuid }).then(setStatus).catch(() => {}), 4000); return () => clearInterval(i)
  }, [q, status])
  // kiosks go back to the welcome screen after a finished order
  useEffect(() => { if (menu?.mode !== 'kiosk' || !status) return; const h = setTimeout(() => setStatus(null), 30000); return () => clearTimeout(h) }, [menu?.mode, status])

  const money = useCallback((n: number) => `${menu?.currency ?? '$'}${n.toFixed(2)}`, [menu])
  const lines = useMemo(() => (menu?.products ?? []).filter((p) => cart[p.id]?.qty > 0).map((p) => ({ p, ...cart[p.id] })), [menu, cart])
  const total = lines.reduce((s, l) => s + l.p.price * l.qty, 0); const count = lines.reduce((s, l) => s + l.qty, 0)
  const set = (id: number, d: number) => setCart((c) => ({ ...c, [id]: { ...c[id], qty: Math.max(0, Math.min(50, (c[id]?.qty ?? 0) + d)) } }))
  const shown = (menu?.products ?? []).filter((p) => cat === null || p.category_ids.includes(cat))
  const canOrder = menu?.mode !== 'consultation' && menu?.open

  const place = async () => {
    if (!q || !lines.length) return; setBusy(true); setErr('')
    try {
      await call(q, 'kiosk_order', { uuid: orderId, takeaway, stand, lines: lines.map((l) => ({ product_id: l.p.id, qty: l.qty, note: l.note })) })
      const s = await call<Status>(q, 'kiosk_status', { uuid: orderId }); setStatus(s); setCart({}); setOpen(false); setOrderId(`kiosk-${uuid().replace(/-/g, '')}`)
      try { localStorage.setItem(KEY(q.config), JSON.stringify({ uuid: s.uuid, at: Date.now() })) } catch { /* private mode */ }
    } catch (e) { setErr(String((e as Error).message ?? e)) } finally { setBusy(false) }
  }

  if (err && !menu) return <div className="ko"><div className="err">{err}</div></div>
  if (!menu) return <div className="ko muted">{t('common.loading')}</div>
  if (status) return (
    <div className="ko ko-done"><h1>{t('kiosk.thanks')}</h1><div className="ko-num">{status.number}</div>
      <div className="ko-steps">{stages.map((s, i) => <div key={s} className={`ko-step ${stages.indexOf(status.stage) >= i ? 'on' : ''}`}><span>{i + 1}</span>{t(`kiosk.stage_${s}`)}</div>)}</div>
      <p className="muted">{status.paid ? t('kiosk.paid') : menu.service_mode === 'table' ? t('kiosk.pay_table') : t('kiosk.pay_counter')} · {money(status.total)}</p>
      <button className="btn p" onClick={() => setStatus(null)}>{t('kiosk.order_more')}</button></div>)
  return (
    <div className={`ko ${menu.mode === 'kiosk' ? 'ko-big' : ''}`}>
      <header className="ko-head"><b>{menu.name}</b>{menu.table && <span className="pill">{t('pos.table')} {q?.table}</span>}</header>
      {!menu.open && <div className="err">{t('kiosk.closed')}</div>}
      {menu.mode === 'consultation' && <p className="muted" style={{ padding: '0 14px' }}>{t('kiosk.view_only')}</p>}
      <div className="pos-cats ko-cats"><button className={`btn ${cat === null ? 'p' : ''}`} onClick={() => setCat(null)}>{t('pos.all')}</button>{menu.categories.filter((c) => !c.parent_id).map((c) => <button key={c.id} className={`btn ${cat === c.id ? 'p' : ''}`} onClick={() => setCat(c.id)}>{c.name}</button>)}</div>
      <div className="ko-grid">{shown.map((p) => (
        <div key={p.id} className="card ko-prod"><b>{p.name}</b>{p.description && <small className="muted">{p.description}</small>}<span>{money(p.price)}</span>
          {canOrder && (cart[p.id]?.qty ? <div className="ko-step-btns"><button className="btn" onClick={() => set(p.id, -1)}>−</button><b>{cart[p.id].qty}</b><button className="btn" onClick={() => set(p.id, 1)}>+</button></div> : <button className="btn p" onClick={() => set(p.id, 1)}>{t('kiosk.add')}</button>)}</div>))}
        {shown.length === 0 && <p className="muted">{t('pos.no_products')}</p>}</div>
      {canOrder && count > 0 && !open && <button className="btn p ko-bar" onClick={() => setOpen(true)}>{t('kiosk.view_order')} · {count} · {money(total)}</button>}
      {open && <div className="pos-modal" onClick={() => setOpen(false)}><div className="card ko-sheet" onClick={(e) => e.stopPropagation()}><h3>{t('kiosk.your_order')}</h3>
        {lines.map((l) => <div key={l.p.id} className="pos-payrow"><span>{l.qty} × {l.p.name}</span><b>{money(l.p.price * l.qty)}</b><span className="ko-step-btns"><button className="btn" onClick={() => set(l.p.id, -1)}>−</button><button className="btn" onClick={() => set(l.p.id, 1)}>+</button></span></div>)}
        {menu.takeaway && <label className="row" style={{ gap: 8 }}><input type="checkbox" checked={takeaway} onChange={(e) => setTakeaway(e.target.checked)} /> {t('kiosk.takeaway')}</label>}
        {menu.service_mode === 'counter' && !menu.table && <input placeholder={t('kiosk.stand')} value={stand} onChange={(e) => setStand(e.target.value)} inputMode="numeric" />}
        {err && <div className="err">{err}</div>}
        <div className="pos-sum"><b>{t('pos.total')}</b><b className="pos-big">{money(total)}</b></div>
        <button className="btn p" disabled={busy} onClick={place}>{t('kiosk.place')}</button></div></div>}
    </div>
  )
}
