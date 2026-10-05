'use client'
import { Suspense, useEffect, useMemo, useState } from 'react'
import { call } from '@/lib/rpc'
import { Icon } from '@/components/Icon'
import type { IconName } from '@/lib/icons'

type Prod = { id: number; name: string; list_price: number; description_sale: string | false; public_categ_ids: number[] }
type Cat = { id: number; name: string }
type Line = { tmpl: number; variant: number; name: string; price: number; qty: number }
type Tracked = { name: string; state: string; amount_total: number; invoice_status?: string; pickings: { name: string; state: string }[]; payments: { provider_id: [number, string]; state: string }[] }
type Provider = { id: number; name: string; code: string; payment_method_ids: number[]; pre_msg: string | false; pending_msg: string | false }
const TEST_CARD = '4242424242424242'
const PAY_STATE: Record<string, string> = { draft: 'Not paid', pending: 'Awaiting payment', authorized: 'Authorized', done: 'Paid', cancel: 'Cancelled', error: 'Failed' }
const strip = (h: string | false) => (h ? h.replace(/<[^>]+>/g, '') : '')

const WEBSITE_ID = 1
const CAT_ICON: Record<string, IconName> = { Apparel: 'shirt', 'Home & Living': 'home', Accessories: 'bag', Gifts: 'gift' }
const money = (n: number) => `$${n.toFixed(2)}`
const KEY = 'shop_cart'
const ORDER_STATE: Record<string, string> = { draft: 'Quotation', sent: 'Quotation sent', sale: 'Confirmed', done: 'Locked', cancel: 'Cancelled' }
const SHIP_STATE: Record<string, string> = { draft: 'Draft', waiting: 'Waiting', confirmed: 'Waiting', assigned: 'Ready to ship', done: 'Delivered', cancel: 'Cancelled' }
const orderLabel = (s: string) => ORDER_STATE[s] ?? s
const shipLabel = (s: string) => SHIP_STATE[s] ?? s

function Shop() {
  const [cats, setCats] = useState<Cat[]>([]); const [prods, setProds] = useState<Prod[]>([]); const [variants, setVariants] = useState<Record<number, number>>({}); const [stockQty, setStockQty] = useState<Record<number, number>>({})
  const [cat, setCat] = useState<number | null>(null); const [q, setQ] = useState(''); const [err, setErr] = useState('')
  const [cart, setCart] = useState<Line[]>([]); const [view, setView] = useState<'shop' | 'cart' | 'checkout' | 'done' | 'track'>('shop')
  const [form, setForm] = useState({ name: '', email: '', street: '', city: '' }); const [busy, setBusy] = useState(false)
  const [providers, setProviders] = useState<Provider[]>([]); const [prov, setProv] = useState<number | null>(null); const [card, setCard] = useState({ number: '', exp: '', cvc: '' }); const [draftOrder, setDraftOrder] = useState<number | null>(null); const [payMsg, setPayMsg] = useState('')
  const [placed, setPlaced] = useState<string>(''); const [track, setTrack] = useState(''); const [tracked, setTracked] = useState<Tracked | null>(null)

  useEffect(() => { try { setCart(JSON.parse(localStorage.getItem(KEY) ?? '[]')) } catch { /* ignore */ } }, [])
  useEffect(() => { try { localStorage.setItem(KEY, JSON.stringify(cart)) } catch { /* ignore */ } }, [cart])
  useEffect(() => {
    (async () => {
      try {
        const pv = await call<Provider[]>('payment.provider', 'search_read', [[['state', '!=', 'disabled'], ['is_published', '=', true]]], { fields: ['name', 'code', 'payment_method_ids', 'pre_msg', 'pending_msg'], order: 'id' })
        setProviders(pv); setProv((c) => c ?? pv[0]?.id ?? null)
        const cs = await call<Cat[]>('product.public.category', 'search_read', [[['name', 'like', 'Shop: ']]], { fields: ['name'], order: 'id' })
        setCats(cs.map((c) => ({ id: c.id, name: c.name.replace('Shop: ', '') })))
        const ids = cs.map((c) => c.id)
        const ps = await call<Prod[]>('product.template', 'search_read', [[['public_categ_ids', 'in', ids], ['sale_ok', '=', true]]], { fields: ['name', 'list_price', 'description_sale', 'public_categ_ids'], order: 'name' })
        setProds(ps)
        const vs = await call<{ id: number; product_tmpl_id: [number, string] }[]>('product.product', 'search_read', [[['product_tmpl_id', 'in', ps.map((p) => p.id)]]], { fields: ['product_tmpl_id'] })
        const vm: Record<number, number> = {}; vs.forEach((v) => { vm[v.product_tmpl_id[0]] = v.id }); setVariants(vm)
        const qs = await call<{ product_id: [number, string]; quantity: number }[]>('stock.quant', 'search_read', [[['product_id', 'in', vs.map((v) => v.id)]]], { fields: ['product_id', 'quantity'] })
        const sm: Record<number, number> = {}; qs.forEach((x) => { sm[x.product_id[0]] = (sm[x.product_id[0]] ?? 0) + (x.quantity || 0) }); setStockQty(sm)
      } catch (e) { setErr(String((e as Error).message ?? e)) }
    })()
  }, [placed])

  const catName = (p: Prod) => cats.find((c) => p.public_categ_ids.includes(c.id))?.name ?? ''
  const shown = useMemo(() => prods.filter((p) => (cat === null || p.public_categ_ids.includes(cat)) && p.name.toLowerCase().includes(q.toLowerCase())), [prods, cat, q])
  const count = cart.reduce((a, l) => a + l.qty, 0); const total = cart.reduce((a, l) => a + l.qty * l.price, 0)
  const add = (p: Prod) => { const v = variants[p.id]; if (!v) return; setCart((c) => c.some((l) => l.tmpl === p.id) ? c.map((l) => l.tmpl === p.id ? { ...l, qty: l.qty + 1 } : l) : [...c, { tmpl: p.id, variant: v, name: p.name, price: p.list_price, qty: 1 }]) }
  const setQty = (tmpl: number, qty: number) => setCart((c) => c.map((l) => l.tmpl === tmpl ? { ...l, qty } : l).filter((l) => l.qty > 0))

  const chosen = providers.find((p) => p.id === prov)
  const cardOk = chosen?.code !== 'demo' || (card.number.replace(/\s/g, '').length >= 12 && /^\d{2}\/\d{2}$/.test(card.exp) && /^\d{3,4}$/.test(card.cvc))
  const place = async () => {
    if (!chosen) { setErr('No payment method is available.'); return }
    setBusy(true); setErr(''); setPayMsg('')
    try {
      let so = draftOrder
      if (!so) {
        const ex = await call<{ id: number }[]>('res.partner', 'search_read', [[['email', '=', form.email]]], { fields: ['id'], limit: 1 })
        const partner = ex[0]?.id ?? await call<number>('res.partner', 'create', [{ name: form.name, email: form.email, street: form.street, city: form.city, customer_rank: 1 }])
        so = await call<number>('sale.order', 'create', [{ partner_id: partner, website_id: WEBSITE_ID, order_line: cart.map((l) => [0, 0, { product_id: l.variant, product_uom_qty: l.qty, price_unit: l.price }]) }]); setDraftOrder(so)
      }
      const [o] = await call<{ name: string; amount_total: number; partner_id: [number, string] }[]>('sale.order', 'search_read', [[['id', '=', so]]], { fields: ['name', 'amount_total', 'partner_id'] })
      const approved = chosen.code !== 'demo' || card.number.replace(/\s/g, '') === TEST_CARD
      const state = chosen.code === 'demo' ? (approved ? 'done' : 'error') : 'pending'
      await call('payment.transaction', 'create', [{ amount: o.amount_total, currency_id: 1, partner_id: o.partner_id[0], payment_method_id: chosen.payment_method_ids[0], provider_id: chosen.id, provider_code: chosen.code, reference: `${o.name}-${Date.now().toString(36)}`, state, state_message: approved ? '' : 'Card declined (demo provider only accepts the test card)', sale_order_ids: [[6, 0, [so]]] }])
      if (!approved) { setErr('Your card was declined. In test mode use 4242 4242 4242 4242.'); return }
      await call('sale.order', 'action_confirm', [[so]])
      setPlaced(o.name); setPayMsg(chosen.code === 'demo' ? 'Payment received. Thank you!' : strip(chosen.pending_msg)); setCart([]); setDraftOrder(null); setCard({ number: '', exp: '', cvc: '' }); setView('done')
    } catch (e) { setErr(String((e as Error).message ?? e)) } finally { setBusy(false) }
  }
  const lookup = async (name = track) => {
    setErr(''); setTracked(null)
    try {
      const [o] = await call<(Tracked & { id: number })[]>('sale.order', 'search_read', [[['name', '=', name.trim()]]], { fields: ['name', 'state', 'amount_total', 'invoice_status'], limit: 1 })
      if (!o) { setErr('Order not found'); return }
      const pickings = await call<{ name: string; state: string }[]>('stock.picking', 'search_read', [[['origin', '=', o.name]]], { fields: ['name', 'state'] })
      const payments = await call<Tracked['payments']>('payment.transaction', 'search_read', [[['sale_order_ids', 'in', [o.id]]]], { fields: ['provider_id', 'state'], order: 'id desc' })
      setTracked({ ...o, pickings, payments })
    } catch (e) { setErr(String((e as Error).message ?? e)) }
  }

  return (
    <div className="shop">
      <header className="shop-top">
        <a className="shop-brand" href="/storefront/" onClick={(e) => { e.preventDefault(); setView('shop') }}><Icon name="bag" size="var(--icon-lg)" /> Sample Store</a>
        <input className="shop-search" placeholder="Search products…" value={q} onChange={(e) => { setQ(e.target.value); setView('shop') }} />
        <button className="btn" onClick={() => { setView('track'); setTracked(null) }}>Track order</button>
        <button className="btn p" onClick={() => setView('cart')}>Cart ({count})</button>
        <a className="btn" href="/">Back office</a>
      </header>
      {err && <div className="err">{err}</div>}
      {view === 'shop' && <>
        <div className="shop-chips">
          <button className={`btn${cat === null ? ' p' : ''}`} onClick={() => setCat(null)}>All</button>
          {cats.map((c) => <button key={c.id} className={`btn${cat === c.id ? ' p' : ''}`} onClick={() => setCat(c.id)}>{CAT_ICON[c.name] && <Icon name={CAT_ICON[c.name]} size="var(--icon-md)" />} {c.name}</button>)}
        </div>
        <div className="shop-grid">
          {shown.map((p) => { const left = stockQty[variants[p.id]] ?? 0; return (
            <div className="shop-card" key={p.id}>
              <div className="shop-img"><Icon name={CAT_ICON[catName(p)] ?? 'box'} size={56} /></div>
              <b>{p.name}</b>
              <div className="muted">{p.description_sale || ''}</div>
              <div className="shop-row"><span className="shop-price">{money(p.list_price)}</span><span className="muted">{left > 0 ? (left < 40 ? `Only ${left} left` : 'In stock') : 'Out of stock'}</span></div>
              <button className="btn p" disabled={left <= 0} onClick={() => add(p)}>Add to cart</button>
            </div>) })}
          {!shown.length && <p className="muted">No products found.</p>}
        </div>
      </>}
      {view === 'cart' && <div className="shop-panel">
        <h2>Your cart</h2>
        {!cart.length ? <p className="muted">Your cart is empty.</p> : <>
          {cart.map((l) => <div className="shop-line" key={l.tmpl}><span>{l.name}</span><span><button className="btn" onClick={() => setQty(l.tmpl, l.qty - 1)}>−</button> {l.qty} <button className="btn" onClick={() => setQty(l.tmpl, l.qty + 1)}>+</button></span><b>{money(l.qty * l.price)}</b></div>)}
          <div className="shop-line"><b>Total</b><span /><b>{money(total)}</b></div>
          <button className="btn p" onClick={() => setView('checkout')}>Checkout</button>
        </>}
        <button className="btn" onClick={() => setView('shop')}>Continue shopping</button>
      </div>}
      {view === 'checkout' && <div className="shop-panel">
        <h2>Checkout</h2>
        {(['name', 'email', 'street', 'city'] as const).map((k) => <label key={k} className="shop-field">{k[0].toUpperCase() + k.slice(1)}<input value={form[k]} onChange={(e) => setForm({ ...form, [k]: e.target.value })} /></label>)}
        <h3>Payment</h3>
        {!providers.length && <div className="err">Payment is not configured for this shop.</div>}
        {providers.map((p) => <label key={p.id} className="shop-pay"><input type="radio" name="prov" checked={prov === p.id} onChange={() => setProv(p.id)} /> <b>{p.name}</b>{p.code === 'demo' ? ' — card' : ' — bank transfer'}</label>)}
        {chosen?.code === 'demo' && <div className="shop-card-form">
          {chosen.pre_msg && <p className="muted">{strip(chosen.pre_msg)}</p>}
          <label className="shop-field">Card number<input inputMode="numeric" autoComplete="off" placeholder="4242 4242 4242 4242" value={card.number} onChange={(e) => setCard({ ...card, number: e.target.value })} /></label>
          <div className="shop-two"><label className="shop-field">Expiry<input placeholder="MM/YY" value={card.exp} onChange={(e) => setCard({ ...card, exp: e.target.value })} /></label><label className="shop-field">CVC<input inputMode="numeric" placeholder="123" value={card.cvc} onChange={(e) => setCard({ ...card, cvc: e.target.value })} /></label></div>
        </div>}
        {chosen && chosen.code !== 'demo' && <p className="muted">{strip(chosen.pending_msg)}</p>}
        <p className="muted">{count} item(s) · {money(total)}</p>
        <button className="btn p" disabled={busy || !chosen || !cardOk || !cart.length || !form.name || !/\S+@\S+/.test(form.email)} onClick={place}>{busy ? 'Processing…' : chosen?.code === 'demo' ? `Pay ${money(total)}` : 'Place order'}</button>
        <button className="btn" onClick={() => setView('cart')}>Back</button>
      </div>}
      {view === 'done' && <div className="shop-panel">
        <h2><Icon name="check-circle" /> Thank you!</h2><p>Your order <b>{placed}</b> is confirmed. We&apos;ll ship it shortly.</p>{payMsg && <p className="muted">{payMsg}</p>}
        <button className="btn p" onClick={() => { setTrack(placed); setView('track'); lookup(placed) }}>Track this order</button> <button className="btn" onClick={() => setView('shop')}>Keep shopping</button>
      </div>}
      {view === 'track' && <div className="shop-panel">
        <h2>Track an order</h2>
        <label className="shop-field">Order number<input value={track} placeholder="S00007" onChange={(e) => setTrack(e.target.value)} /></label>
        <button className="btn p" onClick={() => lookup()}>Look up</button>
        {tracked && <div className="shop-track"><p><b>{tracked.name}</b> — {orderLabel(tracked.state)} · {money(tracked.amount_total)} · invoice: {tracked.invoice_status ?? 'n/a'}</p>
          {tracked.payments.map((x, i) => <p key={i}><Icon name="credit-card" size="var(--icon-md)" /> {x.provider_id[1]}: {PAY_STATE[x.state] ?? x.state}</p>)}{!tracked.payments.length && <p className="muted">No payment recorded.</p>}
          {tracked.pickings.map((k) => <p key={k.name}><Icon name="box" size="var(--icon-md)" /> {k.name}: {shipLabel(k.state)}</p>)}{!tracked.pickings.length && <p className="muted">No shipment yet.</p>}</div>}
      </div>}
    </div>
  )
}
export default function Page() { return <Suspense><Shop /></Suspense> }
