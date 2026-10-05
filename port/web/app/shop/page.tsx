'use client'
import { useCallback, useEffect, useMemo, useState } from 'react'
import { rpc } from '@/lib/rpc'
import { useT } from '@/lib/i18n'
import { uuid } from '@/lib/pos'
import { Icon } from '@/components/Icon'

type Product = { id: number; name: string; price: number; description?: string; category_ids: number[] }
type Menu = { name: string; mode: string; pickup: boolean; open: boolean; currency: string; categories: { id: number; name: string; parent_id: unknown }[]; products: Product[] }
type Done = { uuid: string; number: string; total: number; path: 'sale' | 'pos' }
type Status = { stage: 'received' | 'preparing' | 'ready' | 'cancelled'; number: string; total: number }
type Q = { config: number; token: string; db?: string }
type Customer = { name: string; email: string; phone: string; street: string; city: string; zip: string }

const call = <T,>(q: Q, method: string, kw: Record<string, unknown>) => rpc<T>({ method, args: [q.config, { access_token: q.token, ...kw }], token: null, db: q.db ?? null })
const KEY = 'shop_customer'

/** Public online shop: catalogue with pictures, cart, customer details, delivery or pickup. Orders become sale orders (or pickup orders on the register). */
export default function Shop() {
  const { t } = useT()
  const [q, setQ] = useState<Q | null>(null); const [menu, setMenu] = useState<Menu | null>(null); const [err, setErr] = useState('')
  const [cat, setCat] = useState<number | null>(null); const [search, setSearch] = useState(''); const [cart, setCart] = useState<Record<number, number>>({})
  const [images, setImages] = useState<Record<string, string>>({}); const [step, setStep] = useState<'shop' | 'checkout' | 'done'>('shop')
  const [me, setMe] = useState<Customer>({ name: '', email: '', phone: '', street: '', city: '', zip: '' }); const [delivery, setDelivery] = useState<'pickup' | 'delivery'>('pickup'); const [note, setNote] = useState('')
  const [busy, setBusy] = useState(false); const [done, setDone] = useState<Done | null>(null); const [status, setStatus] = useState<Status | null>(null); const [orderId, setOrderId] = useState(() => `shop-${uuid().replace(/-/g, '')}`)
  useEffect(() => {
    const p = new URLSearchParams(location.search); const config = Number(p.get('config')); const token = p.get('token') ?? ''
    if (!config || !token) { setErr(t('kiosk.bad_link')); return }
    const nq: Q = { config, token, db: p.get('db') ?? undefined }; setQ(nq)
    call<Menu>(nq, 'kiosk_menu', {}).then(setMenu).catch((e) => setErr(String(e.message ?? e)))
    try { const c = localStorage.getItem(KEY); if (c) setMe(JSON.parse(c) as Customer) } catch { /* first visit */ }
  }, []) // eslint-disable-line react-hooks/exhaustive-deps
  const shown = useMemo(() => (menu?.products ?? []).filter((p) => (cat === null || p.category_ids.includes(cat)) && (!search || p.name.toLowerCase().includes(search.toLowerCase()))), [menu, cat, search])
  // product pictures are fetched a page at a time
  useEffect(() => { if (!q) return; const need = shown.map((p) => p.id).filter((id) => !(String(id) in images)).slice(0, 24); if (!need.length) return
    call<Record<string, string>>(q, 'kiosk_images', { ids: need }).then((r) => setImages((o) => ({ ...o, ...Object.fromEntries(need.map((id) => [String(id), r[String(id)] ?? ''])) }))).catch(() => setImages((o) => ({ ...o, ...Object.fromEntries(need.map((id) => [String(id), ''])) }))) }, [q, shown]) // eslint-disable-line react-hooks/exhaustive-deps
  const money = useCallback((n: number) => `${menu?.currency ?? '$'}${n.toFixed(2)}`, [menu])
  const lines = (menu?.products ?? []).filter((p) => (cart[p.id] ?? 0) > 0).map((p) => ({ p, qty: cart[p.id] })); const total = lines.reduce((s, l) => s + l.p.price * l.qty, 0); const count = lines.reduce((s, l) => s + l.qty, 0)
  const set = (id: number, d: number) => setCart((c) => ({ ...c, [id]: Math.max(0, Math.min(50, (c[id] ?? 0) + d)) }))
  const canOrder = menu?.mode !== 'consultation' && menu?.open
  const field = (k: keyof Customer, label: string, type = 'text') => <label key={k}>{label}<input type={type} value={me[k]} onChange={(e) => setMe((m) => ({ ...m, [k]: e.target.value }))} /></label>

  const place = async () => {
    if (!q) return; setBusy(true); setErr('')
    try {
      const r = await call<Done>(q, 'kiosk_checkout', { uuid: orderId, customer: me, delivery, note, lines: lines.map((l) => ({ product_id: l.p.id, qty: l.qty })) })
      try { localStorage.setItem(KEY, JSON.stringify(me)) } catch { /* private mode */ }
      setDone(r); setCart({}); setStep('done'); setOrderId(`shop-${uuid().replace(/-/g, '')}`)
    } catch (e) { setErr(String((e as Error).message ?? e)) } finally { setBusy(false) }
  }
  useEffect(() => {
    if (!q || !done || status?.stage === 'ready') return
    const poll = () => call<Status>(q, done.path === 'sale' ? 'kiosk_shop_status' : 'kiosk_status', { uuid: done.uuid }).then(setStatus).catch(() => {})
    poll(); const i = setInterval(poll, 8000); return () => clearInterval(i)
  }, [q, done, status?.stage])

  if (err && !menu) return <div className="ko"><div className="err">{err}</div></div>
  if (!menu) return <div className="ko muted">{t('common.loading')}</div>
  if (step === 'done' && done) return (
    <div className="ko ko-done"><h1>{t('shop.thanks')}</h1><div className="ko-num">{done.number}</div><p className="muted">{t('shop.confirmation')} {me.email} · {money(done.total)}</p>
      {status && <p>{status.stage === 'ready' ? t('shop.ready') : status.stage === 'cancelled' ? t('shop.cancelled') : t('shop.in_progress')}</p>}
      <button className="btn p" onClick={() => { setStep('shop'); setDone(null); setStatus(null) }}>{t('shop.continue')}</button></div>)
  if (step === 'checkout') return (
    <div className="ko"><header className="ko-head"><button className="btn" onClick={() => setStep('shop')}><Icon name="arrow-left" size="var(--icon-md)" /></button><b>{t('shop.checkout')}</b></header>
      <div className="shop-form">
        <div className="card" style={{ padding: 14, display: 'grid', gap: 8 }}>{lines.map((l) => <div key={l.p.id} className="pos-rl"><span>{l.qty} × {l.p.name}</span><b>{money(l.p.price * l.qty)}</b></div>)}<div className="pos-rl"><b>{t('pos.total')}</b><b>{money(total)}</b></div></div>
        <div className="card" style={{ padding: 14, display: 'grid', gap: 8 }}>
          {field('name', t('shop.name'))}{field('email', t('shop.email'), 'email')}{field('phone', t('shop.phone'), 'tel')}
          <div className="row" style={{ gap: 8 }}><label className="row" style={{ gap: 6 }}><input type="radio" checked={delivery === 'pickup'} onChange={() => setDelivery('pickup')} /> {t('shop.pickup')}</label><label className="row" style={{ gap: 6 }}><input type="radio" checked={delivery === 'delivery'} onChange={() => setDelivery('delivery')} /> {t('shop.delivery')}</label></div>
          {delivery === 'delivery' && <>{field('street', t('shop.street'))}{field('city', t('shop.city'))}{field('zip', t('shop.zip'))}</>}
          <label>{t('shop.note')}<textarea rows={2} value={note} onChange={(e) => setNote(e.target.value)} /></label></div>
        {err && <div className="err">{err}</div>}
        <button className="btn p ko-place" disabled={busy || !me.name || !me.email} onClick={place}>{t('shop.place')} · {money(total)}</button><p className="muted">{t('shop.pay_later')}</p></div></div>)
  return (
    <div className="ko">
      <header className="ko-head"><b>{menu.name}</b><span className="grow" /><input className="shop-search" placeholder={t('pos.search')} value={search} onChange={(e) => setSearch(e.target.value)} /></header>
      {!menu.open && <div className="err">{t('kiosk.closed')}</div>}
      <div className="pos-cats ko-cats"><button className={`btn ${cat === null ? 'p' : ''}`} onClick={() => setCat(null)}>{t('pos.all')}</button>{menu.categories.filter((c) => !c.parent_id).map((c) => <button key={c.id} className={`btn ${cat === c.id ? 'p' : ''}`} onClick={() => setCat(c.id)}>{c.name}</button>)}</div>
      <div className="ko-grid">{shown.map((p) => (
        <div key={p.id} className="card ko-prod shop-prod">{images[String(p.id)] ? <img src={images[String(p.id)]} alt="" className="shop-img" /> : <div className="shop-img ph">{p.name[0]}</div>}<b>{p.name}</b>{p.description && <small className="muted">{p.description}</small>}<span>{money(p.price)}</span>
          {canOrder && ((cart[p.id] ?? 0) > 0 ? <div className="ko-step-btns"><button className="btn" onClick={() => set(p.id, -1)}>−</button><b>{cart[p.id]}</b><button className="btn" onClick={() => set(p.id, 1)}>+</button></div> : <button className="btn p" onClick={() => set(p.id, 1)}>{t('kiosk.add')}</button>)}</div>))}
        {shown.length === 0 && <p className="muted">{t('pos.no_products')}</p>}</div>
      {canOrder && count > 0 && <button className="btn p ko-bar" onClick={() => setStep('checkout')}>{t('shop.checkout')} · {count} · {money(total)}</button>}
    </div>
  )
}
