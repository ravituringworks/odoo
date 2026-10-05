'use client'
import Link from 'next/link'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { call, rpc } from '@/lib/rpc'
import { useT } from '@/lib/i18n'
import { type Cart, type Method, type Pay, type Pricelist, type Product, type Queued, type Tax, canValidate, cartTotals, change, dequeue, due, emptyCart, enqueue, findByCode, paid, parseScale, quickCash, reduce, roundTo, uuid } from '@/lib/pos'

type Data = { session: { id: number; name: string; start_at: string; cash_register_balance_start: number }; config: Record<string, unknown> & { id: number; name: string }; payment_methods: Method[]; categories: { id: number; name: string; parent_id: unknown }[]; products: Product[]; taxes: Tax[]; pricelists: Pricelist[]; cashier: string; currency: { symbol?: string }; invoicing: boolean }
type Receipt = { name: string; date_order: string; amount_total: number; amount_tax: number; amount_return: number; lines: { full_product_name: string; qty: number; price_unit: number; discount: number; price_subtotal_incl: number }[]; payments: { method?: string; amount: number; is_change?: boolean }[]; id?: number; offline?: boolean }
type Summary = { name: string; orders: number; total: number; refunds: number; opening_cash: number; expected_cash: number; payments: { name: string; amount: number; count: number }[]; state: string; difference?: number }

type TFn = ReturnType<typeof useT>['t']
const QKEY = 'pos_queue'
const loadQ = (): Queued[] => { try { return JSON.parse(localStorage.getItem(QKEY) ?? '[]') } catch { return [] } }
const saveQ = (q: Queued[]) => { try { localStorage.setItem(QKEY, JSON.stringify(q)) } catch { /* private mode */ } }
const act = <T,>(model: string, method: string, ids: number[], kw: Record<string, unknown>) => rpc<T>({ method, model, args: [ids, kw] })
const idOf = (v: unknown) => (Array.isArray(v) ? (v[0] as number) : (v as number))

export default function Pos() {
  const [cfg, setCfg] = useState<number | null | undefined>(undefined)
  useEffect(() => { const c = new URLSearchParams(location.search).get('config'); setCfg(c ? Number(c) : null) }, [])
  if (cfg === undefined) return null
  return cfg === null ? <Registers /> : <Terminal config={cfg} />
}

function Registers() {
  const { t } = useT()
  const [rows, setRows] = useState<{ id: number; name: string }[] | null>(null); const [err, setErr] = useState('')
  const [sess, setSess] = useState<Record<number, string>>({})
  useEffect(() => {
    call<{ id: number; name: string }[]>('pos.config', 'search_read', [[]], { fields: ['name'], order: 'name' }).then(setRows).catch((e) => setErr(String(e.message ?? e)))
    call<{ config_id: unknown; name: string }[]>('pos.session', 'search_read', [[['state', '=', 'opened']]], { fields: ['config_id', 'name'] }).then((s) => setSess(Object.fromEntries(s.map((x) => [idOf(x.config_id), x.name])))).catch(() => {})
  }, [])
  const add = async () => { const id = await call<number>('pos.config', 'create', [{ name: `Shop ${(rows?.length ?? 0) + 1}` }]); location.href = `/pos/?config=${id}` }
  return (
    <div className="pos-reg">
      <div className="bar"><Link href="/">←</Link><h1>{t('pos.title')}</h1><span className="grow" /><button className="btn" onClick={add}>{t('pos.new_register')}</button></div>
      {err && <div className="err">{err}</div>}
      <div className="home">
        {rows?.map((r) => (
          <div key={r.id} className="card tile pos-regcard"><b>{r.name}</b><span className="muted">{sess[r.id] ? `${t('pos.session_open')} · ${sess[r.id]}` : t('pos.closed')}</span>
            <div className="row"><a className="btn p" href={`/pos/?config=${r.id}`}>{sess[r.id] ? t('pos.continue') : t('pos.open_register')}</a><Link className="btn" href={`/form/?model=pos.config&id=${r.id}`}>{t('pos.configure')}</Link></div></div>
        ))}
        {rows?.length === 0 && <p className="muted">{t('pos.no_registers')}</p>}
      </div>
      <div className="row" style={{ marginTop: 16, gap: 8, flexWrap: 'wrap' }}>
        {[['pos.session', t('pos.sessions')], ['pos.order', t('pos.orders')], ['pos.payment.method', t('pos.payment_methods')], ['pos.category', t('pos.categories')], ['product.template', t('pos.products')], ['product.pricelist', t('pos.pricelists')]].map(([m, l]) => <Link key={m} className="btn" href={`/list/?model=${m}&title=${encodeURIComponent(l)}`}>{l}</Link>)}
      </div>
    </div>
  )
}

function Terminal({ config }: { config: number }) {
  const { t } = useT()
  const [d, setD] = useState<Data | null>(null); const [err, setErr] = useState('')
  const [carts, setCarts] = useState<Cart[]>([emptyCart()]); const [cur, setCur] = useState(0)
  const [cat, setCat] = useState<number | null>(null); const [q, setQ] = useState('')
  const [sel, setSel] = useState<string | null>(null); const [mode, setMode] = useState<'qty' | 'discount' | 'price'>('qty'); const [buf, setBuf] = useState('')
  const [screen, setScreen] = useState<'shop' | 'pay' | 'receipt'>('shop'); const [modal, setModal] = useState<null | 'customer' | 'orders' | 'close' | 'pricelist'>(null)
  const [pays, setPays] = useState<Pay[]>([]); const [amount, setAmount] = useState(''); const [invoice, setInvoice] = useState(false)
  const [receipt, setReceipt] = useState<Receipt | null>(null); const [online, setOnline] = useState(true); const [queued, setQueued] = useState(0); const [busy, setBusy] = useState(false)
  const cart = carts[cur] ?? carts[0]
  const dispatch = useCallback((a: Parameters<typeof reduce>[1]) => setCarts((cs) => cs.map((c, i) => (i === cur ? reduce(c, a) : c))), [cur])

  useEffect(() => { act<Data>('pos.config', 'open_ui', [config], {}).then(setD).catch((e) => setErr(String(e.message ?? e))); setQueued(loadQ().length) }, [config])
  const taxes = useMemo(() => new Map((d?.taxes ?? []).map((x) => [x.id, x])), [d])
  const methods = d?.payment_methods ?? []
  const plist = d?.pricelists.find((p) => p.id === cart.pricelist) ?? null
  const totals = useMemo(() => cartTotals(cart, taxes), [cart, taxes])
  const step = d && d.config.cash_rounding ? 0.05 : 0
  const total = roundTo(totals.total, step)
  const sym = d?.currency.symbol ?? '$'; const money = (n: number) => `${n < 0 ? '-' : ''}${sym}${Math.abs(n).toFixed(2)}`

  // flush offline orders whenever we are (or come back) online
  const flush = useCallback(async () => {
    let qq = loadQ()
    for (const o of qq) { try { await act('pos.order', 'create_from_ui', [], o.kwargs); qq = dequeue(qq, o.uuid); saveQ(qq); setQueued(qq.length); setOnline(true) } catch (e) { if (/failed to fetch|network|load failed/i.test(String((e as Error).message))) { setOnline(false); return } qq = dequeue(qq, o.uuid); saveQ(qq); setQueued(qq.length) } }
  }, [])
  useEffect(() => { const on = () => { setOnline(true); flush() }; const off = () => setOnline(false); window.addEventListener('online', on); window.addEventListener('offline', off); const iv = setInterval(flush, 15000); return () => { window.removeEventListener('online', on); window.removeEventListener('offline', off); clearInterval(iv) } }, [flush])

  // barcode scanners type fast and end with Enter
  const scan = useRef({ s: '', at: 0 })
  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement)?.tagName === 'INPUT' || screen !== 'shop' || modal || !d) return
      const now = Date.now(); if (now - scan.current.at > 80) scan.current.s = ''; scan.current.at = now
      if (e.key === 'Enter') { const code = scan.current.s; scan.current.s = ''; if (code.length < 3) return
        const w = parseScale(code); const p = w ? d.products.find((x) => x.code === w.code || x.barcode === w.code) : findByCode(d.products, code)
        if (p) { dispatch({ t: 'add', product: p, pl: plist }); if (w) setCarts((cs) => cs.map((c, i) => i === cur ? { ...c, lines: c.lines.map((l, j) => j === c.lines.length - 1 ? { ...l, qty: w.kg } : l) } : c)) } }
      else if (e.key.length === 1) scan.current.s += e.key
    }
    window.addEventListener('keydown', h); return () => window.removeEventListener('keydown', h)
  }, [d, screen, modal, dispatch, plist, cur])

  const catKids = (id: number): number[] => [id, ...(d?.categories ?? []).filter((c) => idOf(c.parent_id) === id).flatMap((c) => catKids(c.id))]
  const shown = useMemo(() => (d?.products ?? []).filter((p) => (cat === null || p.category_ids.some((c) => catKids(cat).includes(c))) && (!q || `${p.name} ${p.code ?? ''} ${p.barcode ?? ''}`.toLowerCase().includes(q.toLowerCase()))), [d, cat, q]) // eslint-disable-line react-hooks/exhaustive-deps
  const selLine = cart.lines.find((l) => l.key === sel)

  const key = (k: string) => {
    if (k === 'C') { setBuf(''); if (sel && mode === 'qty') dispatch({ t: 'remove', key: sel }); return }
    if (k === '±') { if (selLine) dispatch({ t: 'qty', key: selLine.key, qty: -selLine.qty, pl: plist }); return }
    const nb = k === '⌫' ? buf.slice(0, -1) : buf + k; setBuf(nb)
    if (!sel) return; const n = Number(nb || 0); if (Number.isNaN(n)) return
    dispatch(mode === 'qty' ? { t: 'qty', key: sel, qty: n, pl: plist } : mode === 'discount' ? { t: 'discount', key: sel, pct: n } : { t: 'price', key: sel, price: n })
  }
  const newOrder = () => { setCarts((cs) => [...cs, emptyCart()]); setCur(carts.length); setSel(null); setBuf(''); setPays([]); setInvoice(false); setScreen('shop') }
  const finish = () => { setCarts((cs) => { const rest = cs.filter((_, i) => i !== cur); return rest.length ? rest : [emptyCart()] }); setCur(0); setSel(null); setBuf(''); setPays([]); setInvoice(false); setReceipt(null); setScreen('shop') }

  const validate = async () => {
    if (!d) return; setBusy(true); setErr('')
    const kwargs = { session_id: d.session.id, uuid: cart.uuid, partner_id: cart.partner?.id, pricelist_id: cart.pricelist ?? undefined, to_invoice: invoice, note: cart.note,
      lines: cart.lines.map((l) => ({ product_id: l.product.id, qty: l.qty, price_unit: l.price, discount: l.discount, note: l.note })), payments: pays.map((p) => ({ payment_method_id: p.method, amount: p.amount })) }
    try { setReceipt(await act<Receipt>('pos.order', 'create_from_ui', [], kwargs)); setScreen('receipt') }
    catch (e) {
      const m = String((e as Error).message ?? e)
      if (/failed to fetch|network|load failed/i.test(m)) {   // keep selling offline: queue it, the server dedupes by uuid when it syncs
        const qq = enqueue(loadQ(), { uuid: cart.uuid, kwargs }); saveQ(qq); setQueued(qq.length); setOnline(false)
        setReceipt({ name: `Offline ${cart.uuid.slice(0, 6)}`, date_order: new Date().toISOString().slice(0, 19).replace('T', ' '), amount_total: total, amount_tax: totals.tax, amount_return: change(total, pays, methods), offline: true,
          lines: cart.lines.map((l) => ({ full_product_name: l.product.name, qty: l.qty, price_unit: l.price, discount: l.discount, price_subtotal_incl: cartTotals({ ...cart, lines: [l] }, taxes).total })), payments: pays.map((p) => ({ method: methods.find((x) => x.id === p.method)?.name, amount: p.amount })) })
        setScreen('receipt')
      } else setErr(m)
    } finally { setBusy(false) }
  }

  if (err && !d) return <div className="pos-reg"><div className="err">{err}</div><Link className="btn" href="/pos/">←</Link></div>
  if (!d) return <div className="pos-reg muted">{t('common.loading')}</div>
  const topCats = d.categories.filter((c) => !c.parent_id)
  return (
    <div className="pos">
      <header className="pos-top">
        <Link href="/pos/" className="btn">←</Link><b>{d.config.name}</b><span className="muted">{d.session.name} · {d.cashier}</span>
        <div className="pos-tabs">{carts.map((c, i) => <button key={c.uuid} className={`btn ${i === cur ? 'p' : ''}`} onClick={() => { setCur(i); setSel(null); setScreen('shop'); setPays([]) }}>{c.lines.length ? `#${i + 1} · ${money(cartTotals(c, taxes).total)}` : `#${i + 1}`}</button>)}<button className="btn" onClick={newOrder}>+</button></div>
        <span className="grow" />
        <span className={`pill ${online ? 'ok' : 'bad'}`}>{online ? t('pos.online') : t('pos.offline')}{queued > 0 && ` · ${queued}`}</span>
        <button className="btn" onClick={() => setModal('orders')}>{t('pos.orders')}</button><button className="btn" onClick={() => setModal('close')}>{t('pos.close')}</button>
      </header>
      {err && <div className="err" onClick={() => setErr('')}>{err}</div>}
      {screen === 'shop' && (
        <div className="pos-body">
          <section className="pos-ticket">
            <div className="pos-lines">
              {cart.lines.length === 0 && <p className="muted pos-empty">{t('pos.empty')}</p>}
              {cart.lines.map((l) => (
                <div key={l.key} className={`pos-line ${l.key === sel ? 'on' : ''}`} onClick={() => { setSel(l.key); setBuf('') }}>
                  <b>{l.product.name}</b><span>{money(cartTotals({ ...cart, lines: [l] }, taxes).total)}</span>
                  <small className="muted">{l.qty} × {money(l.price)}{l.discount ? ` · -${l.discount}%` : ''}</small>
                  {l.note && <small>📝 {l.note}</small>}
                </div>))}
            </div>
            <div className="pos-sum"><span>{t('pos.taxes')}</span><span>{money(totals.tax)}</span><b>{t('pos.total')}</b><b className="pos-big">{money(total)}</b></div>
            <div className="pos-actions">
              <button className="btn" onClick={() => setModal('customer')}>👤 {cart.partner?.name ?? t('pos.customer')}</button>
              {d.config.use_pricelist ? <button className="btn" onClick={() => setModal('pricelist')}>🏷 {plist?.name ?? t('pos.pricelist')}</button> : null}
              {selLine && <button className="btn" onClick={() => { const n = prompt(t('pos.note'), selLine.note ?? ''); if (n !== null) dispatch({ t: 'note', key: selLine.key, note: n }) }}>📝</button>}
            </div>
            <div className="pos-pad">
              <div className="pos-modes">{(['qty', 'discount', 'price'] as const).filter((m) => m === 'qty' || (m === 'discount' ? !!d.config.manual_discount : !d.config.restrict_price_control)).map((m) => <button key={m} className={`btn ${mode === m ? 'p' : ''}`} onClick={() => { setMode(m); setBuf('') }}>{t(`pos.${m}`)}</button>)}</div>
              {['1', '2', '3', '4', '5', '6', '7', '8', '9', '±', '0', '.', 'C', '⌫'].map((k) => <button key={k} className="btn" onClick={() => key(k)}>{k}</button>)}
              <button className="btn p pos-pay" disabled={!cart.lines.length} onClick={() => { setPays([]); setAmount(''); setScreen('pay') }}>{t('pos.payment')} ›</button>
            </div>
          </section>
          <section className="pos-catalog">
            <div className="pos-search"><input placeholder={t('pos.search')} value={q} onChange={(e) => setQ(e.target.value)} />
              <div className="pos-cats"><button className={`btn ${cat === null ? 'p' : ''}`} onClick={() => setCat(null)}>{t('pos.all')}</button>{topCats.map((c) => <button key={c.id} className={`btn ${cat === c.id ? 'p' : ''}`} onClick={() => setCat(c.id)}>{c.name}</button>)}</div></div>
            <div className="pos-grid">{shown.map((p) => <button key={p.id} className="card pos-prod" onClick={() => { dispatch({ t: 'add', product: p, pl: plist }); setSel(null) }}><b>{p.name}</b><span className="muted">{money(p.price)}</span></button>)}{shown.length === 0 && <p className="muted">{t('pos.no_products')}</p>}</div>
          </section>
        </div>)}
      {screen === 'pay' && (
        <div className="pos-pay-screen card">
          <h2>{t('pos.payment')} · {money(total)}</h2>
          <div className="pos-pay-cols">
            <div>{methods.map((m) => <button key={m.id} className="btn pos-method" onClick={() => { const a = amount ? Number(amount) : Math.max(due(total, pays), 0); if (a > 0) setPays((p) => [...p, { method: m.id, amount: a }]); setAmount('') }}>{m.kind === 'cash' ? '💵' : m.kind === 'account' ? '🧾' : '💳'} {m.name}</button>)}
              <div className="row" style={{ gap: 6, flexWrap: 'wrap', marginTop: 8 }}>{quickCash(Math.max(due(total, pays), 0)).map((n) => <button key={n} className="btn" onClick={() => setAmount(String(n))}>{money(n)}</button>)}</div>
              <input className="pos-amt" inputMode="decimal" placeholder={String(Math.max(due(total, pays), 0).toFixed(2))} value={amount} onChange={(e) => setAmount(e.target.value)} />
              <label className="row" style={{ gap: 6, marginTop: 10 }}><input type="checkbox" checked={invoice} disabled={!d.invoicing} onChange={(e) => setInvoice(e.target.checked)} /> {t('pos.invoice')}{invoice && !cart.partner && <button className="btn" onClick={() => setModal('customer')}>{t('pos.pick_customer')}</button>}</label></div>
            <div>{pays.map((p, i) => <div key={i} className="pos-payrow"><span>{methods.find((m) => m.id === p.method)?.name}</span><b>{money(p.amount)}</b><button className="btn" onClick={() => setPays((x) => x.filter((_, j) => j !== i))}>×</button></div>)}
              <div className="pos-sum"><b>{due(total, pays) > 0 ? t('pos.remaining') : t('pos.change')}</b><b className="pos-big">{money(due(total, pays) > 0 ? due(total, pays) : change(total, pays, methods))}</b></div>
              <div className="row" style={{ gap: 8 }}><button className="btn" onClick={() => setScreen('shop')}>‹ {t('pos.back')}</button><button className="btn p grow" disabled={busy || !canValidate(cart, total, pays, methods, invoice)} onClick={validate}>{t('pos.validate')}</button></div></div>
          </div>
        </div>)}
      {screen === 'receipt' && receipt && <ReceiptView r={receipt} d={d} money={money} done={finish} t={t} />}
      {modal === 'customer' && <Customers close={() => setModal(null)} pick={(p) => { dispatch({ t: 'partner', partner: p }); setModal(null) }} t={t} />}
      {modal === 'pricelist' && <div className="pos-modal" onClick={() => setModal(null)}><div className="card" onClick={(e) => e.stopPropagation()}><h3>{t('pos.pricelist')}</h3>{[{ id: 0, name: t('pos.default'), items: [] } as Pricelist, ...d.pricelists].map((p) => <button key={p.id} className="btn" onClick={() => { dispatch({ t: 'pricelist', id: p.id || null, list: p.id ? p : null }); setModal(null) }}>{p.name}</button>)}</div></div>}
      {modal === 'orders' && <Orders d={d} money={money} close={() => setModal(null)} t={t} onRefund={(r) => { setReceipt(r); setModal(null); setScreen('receipt') }} />}
      {modal === 'close' && <Close d={d} money={money} close={() => setModal(null)} t={t} />}
    </div>
  )
}

function ReceiptView({ r, d, money, done, t }: { r: Receipt; d: Data; money: (n: number) => string; done: () => void; t: TFn }) {
  return (
    <div className="pos-receipt-wrap">
      <div className="pos-receipt card" id="pos-receipt">
        {!!d.config.receipt_header && <p style={{ whiteSpace: 'pre-line', textAlign: 'center' }}>{String(d.config.receipt_header)}</p>}
        <h3 style={{ textAlign: 'center' }}>{r.name}{r.offline ? ` (${t('pos.offline')})` : ''}</h3><p className="muted" style={{ textAlign: 'center' }}>{r.date_order} · {d.cashier}</p>
        {r.lines.map((l, i) => <div key={i} className="pos-rl"><span>{l.qty} × {l.full_product_name}{l.discount ? ` (-${l.discount}%)` : ''}</span><span>{money(l.price_subtotal_incl)}</span></div>)}
        <hr /><div className="pos-rl"><span>{t('pos.taxes')}</span><span>{money(r.amount_tax)}</span></div><div className="pos-rl"><b>{t('pos.total')}</b><b>{money(r.amount_total)}</b></div>
        {r.payments.filter((p) => !p.is_change).map((p, i) => <div key={i} className="pos-rl"><span>{p.method}</span><span>{money(p.amount)}</span></div>)}
        {r.amount_return > 0 && <div className="pos-rl"><span>{t('pos.change')}</span><span>{money(r.amount_return)}</span></div>}
        {!!d.config.receipt_footer && <p style={{ whiteSpace: 'pre-line', textAlign: 'center' }}>{String(d.config.receipt_footer)}</p>}
      </div>
      <div className="pos-receipt-actions"><button className="btn" onClick={() => window.print()}>🖨 {t('pos.print')}</button><button className="btn p" onClick={done}>{t('pos.new_order')} ›</button></div>
    </div>
  )
}

function Customers({ pick, close, t }: { pick: (p: { id: number; name: string } | null) => void; close: () => void; t: TFn }) {
  const [q, setQ] = useState(''); const [rows, setRows] = useState<{ id: number; name: string; email?: string }[]>([])
  useEffect(() => { const h = setTimeout(() => call<typeof rows>('res.partner', 'search_read', [q ? [['name', 'ilike', q]] : []], { fields: ['name', 'email'], limit: 30, order: 'name' }).then(setRows).catch(() => setRows([])), 150); return () => clearTimeout(h) }, [q])
  return <div className="pos-modal" onClick={close}><div className="card" onClick={(e) => e.stopPropagation()}><h3>{t('pos.customer')}</h3><input autoFocus placeholder={t('pos.search')} value={q} onChange={(e) => setQ(e.target.value)} />
    <div className="pos-list"><button className="btn" onClick={() => pick(null)}>— {t('pos.no_customer')}</button>{rows.map((r) => <button key={r.id} className="btn" onClick={() => pick({ id: r.id, name: r.name })}>{r.name} <span className="muted">{r.email || ''}</span></button>)}</div></div></div>
}

function Orders({ d, money, close, onRefund, t }: { d: Data; money: (n: number) => string; close: () => void; onRefund: (r: Receipt) => void; t: TFn }) {
  const [rows, setRows] = useState<{ id: number; name: string; amount_total: number; state: string; date_order: string }[]>([]); const [err, setErr] = useState('')
  useEffect(() => { call<typeof rows>('pos.order', 'search_read', [[['session_id', '=', d.session.id]]], { fields: ['name', 'amount_total', 'state', 'date_order'], order: 'id desc', limit: 50 }).then(setRows).catch((e) => setErr(String(e.message ?? e))) }, [d.session.id])
  const refund = async (id: number) => { try { onRefund(await act<Receipt>('pos.order', 'refund', [id], { session_id: d.session.id })) } catch (e) { setErr(String((e as Error).message ?? e)) } }
  return <div className="pos-modal" onClick={close}><div className="card" onClick={(e) => e.stopPropagation()}><h3>{t('pos.orders')}</h3>{err && <div className="err">{err}</div>}
    <div className="pos-list">{rows.map((r) => <div key={r.id} className="pos-payrow"><span>{r.name}<br /><small className="muted">{r.date_order}</small></span><b>{money(r.amount_total)}</b>{r.amount_total > 0 && <button className="btn" onClick={() => refund(r.id)}>↩ {t('pos.refund')}</button>}</div>)}{rows.length === 0 && <p className="muted">{t('pos.no_orders')}</p>}</div></div></div>
}

function Close({ d, money, close, t }: { d: Data; money: (n: number) => string; close: () => void; t: TFn }) {
  const [s, setS] = useState<Summary | null>(null); const [counted, setCounted] = useState(''); const [res, setRes] = useState<Summary | null>(null); const [err, setErr] = useState('')
  useEffect(() => { act<Summary>('pos.session', 'summary', [d.session.id], {}).then(setS).catch((e) => setErr(String(e.message ?? e))) }, [d.session.id])
  const go = async (force = false) => { try { setRes(await act<Summary>('pos.session', 'close_session', [d.session.id], { counted_cash: counted === '' ? undefined : Number(counted), force })) } catch (e) { setErr(String((e as Error).message ?? e)) } }
  return <div className="pos-modal" onClick={close}><div className="card" onClick={(e) => e.stopPropagation()}><h3>{t('pos.close_register')} · {d.session.name}</h3>{err && <div className="err">{err}</div>}
    {s && <><div className="pos-rl"><span>{t('pos.orders')}</span><b>{s.orders}</b></div><div className="pos-rl"><span>{t('pos.total')}</span><b>{money(s.total)}</b></div>{s.refunds !== 0 && <div className="pos-rl"><span>{t('pos.refund')}</span><b>{money(s.refunds)}</b></div>}
      {s.payments.map((p) => <div key={p.name} className="pos-rl"><span>{p.name}</span><span>{money(p.amount)}</span></div>)}<hr /><div className="pos-rl"><span>{t('pos.expected_cash')}</span><b>{money(s.expected_cash)}</b></div></>}
    {!res ? <><label>{t('pos.counted_cash')}<input inputMode="decimal" value={counted} onChange={(e) => setCounted(e.target.value)} /></label><div className="row" style={{ gap: 8, marginTop: 10 }}><button className="btn" onClick={close}>{t('pos.back')}</button><button className="btn p" onClick={() => go()}>{t('pos.close_register')}</button></div></>
      : <><div className="pos-rl"><span>{t('pos.difference')}</span><b>{res.difference == null ? '—' : money(res.difference)}</b></div><button className="btn p" onClick={() => { location.href = '/pos/' }}>{t('pos.done')}</button></>}</div></div>
}
