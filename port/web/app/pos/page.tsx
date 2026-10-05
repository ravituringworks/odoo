'use client'
import Link from 'next/link'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { call, rpc } from '@/lib/rpc'
import { useT } from '@/lib/i18n'
import { type Card, type Program, type RewardLine, claimable, programPoints, rewardLines, rewardTotal, type Item as LItem } from '@/lib/loyalty'
import { Ticket, kitchenTicket, receipt as escReceipt } from '@/lib/escpos'
import { type HwSettings, connectSerial, loadHw, readWeight, saveHw, serialConnected, serialSupported, writeSerial } from '@/lib/hardware'
import { cartFromDraft, hasUnsent, kitchenDelta, tipAmount, r2, lineTotals, type Cart, type Method, type Pay, type Pricelist, type Product, type Queued, type Tax, canValidate, cartTotals, change, dequeue, due, emptyCart, enqueue, findByCode, paid, parseScale, quickCash, reduce, roundTo, uuid } from '@/lib/pos'

type Data = { session: { id: number; name: string; start_at: string; cash_register_balance_start: number }; config: Record<string, unknown> & { id: number; name: string }; payment_methods: Method[]; categories: { id: number; name: string; parent_id: unknown }[]; products: Product[]; taxes: Tax[]; pricelists: Pricelist[]; cashier: string; currency: { symbol?: string }; invoicing: boolean; loyalty: Program[]; floors: Floor[]; printers: { name: string; target: string; category_ids: number[] }[]; employees: { id: number; name: string; role: 'basic' | 'advanced'; has_pin: boolean }[]; cashier_lock: boolean }
type Table = { id: number; name: string; seats: number; shape: string; position_h: number; position_v: number; width: number; height: number; color?: string; orders: number; total: number }
type Floor = { id: number; name: string; background_color?: string; tables: Table[] }
type Emp = { id: number; name: string; role: 'basic' | 'advanced' }
type Receipt = { name: string; date_order: string; amount_total: number; amount_tax: number; amount_return: number; lines: { full_product_name: string; qty: number; price_unit: number; discount: number; price_subtotal_incl: number }[]; payments: { method?: string; amount: number; is_change?: boolean }[]; id?: number; offline?: boolean; loyalty_issued?: { program: string; type: string; code: string; points: number; earned: number }[] }
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
            <div className="row"><a className="btn p" href={`/pos/?config=${r.id}`}>{sess[r.id] ? t('pos.continue') : t('pos.open_register')}</a><Link className="btn" href={`/form/?model=pos.config&id=${r.id}`}>{t('pos.configure')}</Link><a className="btn" href={`/pos/kitchen/?config=${r.id}`} title={t('pos.kitchen')}>🍳</a><a className="btn" href={`/pos/qr/?config=${r.id}`} title={t('kiosk.qr_title')}>📱</a></div></div>
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
  const [screen, setScreen] = useState<'floor' | 'shop' | 'pay' | 'receipt'>('shop'); const [modal, setModal] = useState<null | 'customer' | 'orders' | 'close' | 'pricelist' | 'rewards' | 'split' | 'transfer' | 'tip' | 'guests' | 'tableorders' | 'hardware' | 'scale'>(null)
  const [pays, setPays] = useState<Pay[]>([]); const [amount, setAmount] = useState(''); const [invoice, setInvoice] = useState(false)
  const [emp, setEmp] = useState<Emp | null>(null)
  const [hw, setHw] = useState<HwSettings>(() => (typeof localStorage === 'undefined' ? loadHw() : loadHw())); const [weighing, setWeighing] = useState<Product | null>(null)
  const updateHw = (p: Partial<HwSettings>) => setHw((o) => { const n = { ...o, ...p }; saveHw(n); return n })
  const [receipt, setReceipt] = useState<Receipt | null>(null); const [online, setOnline] = useState(true); const [queued, setQueued] = useState(0); const [busy, setBusy] = useState(false)
  const [floorIdx, setFloorIdx] = useState(0); const [pending, setPending] = useState<{ table: Table; orders: Parameters<typeof cartFromDraft>[0][] } | null>(null); const [toast, setToast] = useState('')
  const cart = carts[cur] ?? carts[0]
  const dispatch = useCallback((a: Parameters<typeof reduce>[1]) => setCarts((cs) => cs.map((c, i) => (i === cur ? reduce(c, a) : c))), [cur])

  useEffect(() => { act<Data>('pos.config', 'open_ui', [config], {}).then((x) => { setD(x); if (x.floors.length) setScreen('floor') }).catch((e) => setErr(String(e.message ?? e))); setQueued(loadQ().length) }, [config])
  const taxes = useMemo(() => new Map((d?.taxes ?? []).map((x) => [x.id, x])), [d])
  const methods = d?.payment_methods ?? []
  const plist = d?.pricelists.find((p) => p.id === cart.pricelist) ?? null
  const base = useMemo(() => cartTotals(cart, taxes), [cart, taxes])
  const [cards, setCards] = useState<Card[]>([])
  useEffect(() => { if (cart.partner && d?.loyalty.length) act<Card[]>('pos.config', 'loyalty_cards', [config], { partner_id: cart.partner.id }).then((c) => setCards((old) => [...c, ...old.filter((o) => !c.some((x) => x.id === o.id))])).catch(() => {}) }, [cart.partner?.id, d?.loyalty.length, config]) // eslint-disable-line react-hooks/exhaustive-deps
  const litems: LItem[] = useMemo(() => cart.lines.map((l) => { const x = lineTotals(l, taxes); return { product: l.product.id, categs: l.product.categ_chain ?? [], qty: l.qty, price: l.price, untaxed: x.untaxed, total: x.total, taxIds: l.product.tax_ids } }), [cart.lines, taxes])
  const rlines: RewardLine[] = useMemo(() => (d ? rewardLines(cart.claims ?? [], d.loyalty, cards, litems, cart.codes ?? [], taxes) : []), [d, cart.claims, cards, litems, cart.codes, taxes])
  const rsum = rewardTotal(rlines, taxes)
  const totals = useMemo(() => ({ untaxed: r2(base.untaxed + rlines.reduce((s, l) => s + l.price, 0)), tax: r2(base.tax + rsum - rlines.reduce((s, l) => s + l.price, 0)), total: r2(base.total + rsum) }), [base, rlines, rsum])
  const earned = useMemo(() => (d?.loyalty ?? []).filter((p) => ['future', 'both'].includes(p.applies_on)).map((p) => ({ p, pts: programPoints(p, litems, cart.codes ?? []) })).filter((x) => x.pts > 0 && (!x.p.is_nominative && x.p.program_type !== 'loyalty' ? true : !!cart.partner)), [d, litems, cart.codes, cart.partner])
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
  const finish = () => { setCarts((cs) => { const rest = cs.filter((_, i) => i !== cur); return rest.length ? rest : [emptyCart()] }); setCur(0); setSel(null); setBuf(''); setPays([]); setInvoice(false); setReceipt(null); setScreen(restaurant ? 'floor' : 'shop'); if (restaurant) refreshFloors() }

  const orderKw = (c: Cart) => ({ session_id: d!.session.id, uuid: c.uuid, employee_id: emp?.id, table_id: c.table?.id, customer_count: c.guests, takeaway: !!c.takeaway, partner_id: c.partner?.id, note: c.note,
    lines: c.lines.map((l) => ({ uuid: l.key, product_id: l.product.id, qty: l.qty, price_unit: l.price, discount: l.discount, note: l.note })) })
  const restaurant = !!d?.floors.length
  // table orders are persisted as drafts so every terminal (and a refresh) sees them
  const draftTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  useEffect(() => {
    if (!d || !cart.table || screen === 'floor' || screen === 'receipt' || cart.lines.length === 0) return
    if (draftTimer.current) clearTimeout(draftTimer.current)
    draftTimer.current = setTimeout(() => { act('pos.order', 'save_draft', [], orderKw(cart)).catch(() => {}) }, 600)
    return () => { if (draftTimer.current) clearTimeout(draftTimer.current) }
  }, [cart.lines, cart.guests, cart.takeaway, cart.partner?.id, cart.table?.id, screen]) // eslint-disable-line react-hooks/exhaustive-deps
  const refreshFloors = useCallback(() => act<Data>('pos.config', 'open_ui', [config], {}).then((x) => setD((o) => (o ? { ...o, floors: x.floors } : x))).catch(() => {}), [config])
  const openTable = async (tb: Table) => {
    const orders = await act<Parameters<typeof cartFromDraft>[0][]>('pos.config', 'open_orders', [config], { table_id: tb.id }).catch(() => [])
    if (orders.length === 0) { setPending({ table: tb, orders: [] }); setModal('guests') } else if (orders.length === 1) loadTable(tb, orders[0]); else { setPending({ table: tb, orders }); setModal('tableorders') }
  }
  const loadTable = (tb: Table, draft: Parameters<typeof cartFromDraft>[0] | null, guests = 1) => {
    const c = draft ? cartFromDraft(draft, d!.products, { id: tb.id, name: tb.name }) : { ...emptyCart(), table: { id: tb.id, name: tb.name }, guests, sent: {} }
    setCarts((cs) => { const i = cs.findIndex((x) => x.uuid === c.uuid); if (i >= 0) { setCur(i); return cs.map((x, j) => (j === i ? c : x)) } setCur(cs.length); return [...cs, c] })
    setSel(null); setBuf(''); setPays([]); setScreen('shop'); setModal(null); setPending(null)
  }
  const leaveTable = async () => {
    if (cart.table && cart.lines.length === 0) { await act('pos.order', 'discard_draft', [], { uuid: cart.uuid }).catch(() => {}); setCarts((cs) => (cs.length > 1 ? cs.filter((_, i) => i !== cur) : [emptyCart()])); setCur(0) }
    await refreshFloors(); setScreen('floor')
  }
  const sendKitchen = async () => {
    const fresh = cart.lines.filter((l) => l.qty > (cart.sent?.[l.key] ?? 0)).map((l) => ({ name: l.product.name, qty: l.qty - (cart.sent?.[l.key] ?? 0), note: l.note, cats: l.product.category_ids }))
    try { const r = await act<{ ticket_id?: number }>('pos.order', 'send_to_kitchen', [], orderKw(cart)); printKitchen(fresh).catch((e) => setErr(String(e.message ?? e))); dispatch({ t: 'meta', meta: { sent: Object.fromEntries(cart.lines.map((l) => [l.key, l.qty])) } }); setToast(r.ticket_id ? t('pos.sent_kitchen') : t('pos.nothing_new')); setTimeout(() => setToast(''), 2000) } catch (e) { setErr(String((e as Error).message ?? e)) }
  }
  const bill = () => { setReceipt({ name: `${t('pos.bill')} ${cart.table ? `· ${cart.table.name}` : ''}`, date_order: new Date().toISOString().slice(0, 19).replace('T', ' '), amount_total: total, amount_tax: totals.tax, amount_return: 0, lines: cart.lines.map((l) => ({ full_product_name: l.product.name, qty: l.qty, price_unit: l.price, discount: l.discount, price_subtotal_incl: lineTotals(l, taxes).total })), payments: [] }); setScreen('receipt') }
  const split = (keys: string[]) => {
    const moved = cart.lines.filter((l) => keys.includes(l.key)); if (!moved.length || moved.length === cart.lines.length) return
    const n: Cart = { ...emptyCart(), table: cart.table, guests: 1, sent: Object.fromEntries(moved.map((l) => [l.key, cart.sent?.[l.key] ?? 0])), lines: moved }
    setCarts((cs) => [...cs.map((c, i) => (i === cur ? reduce(c, { t: 'move', keys }) : c)), n]); setCur(carts.length); setModal(null)
  }
  const transfer = async (tb: Table) => { try { await act('pos.order', 'transfer_table', [], { uuid: cart.uuid, table_id: tb.id }); dispatch({ t: 'meta', meta: { table: { id: tb.id, name: tb.name } } }); setModal(null) } catch (e) { setErr(String((e as Error).message ?? e)) } }
  const addTip = (pct: number | null, amountIn?: number) => {
    const tp = d?.config.tip_product_id; const id = Array.isArray(tp) ? (tp[0] as number) : (tp as number | undefined); if (!id) return
    const amt = amountIn ?? tipAmount(base.total, pct ?? 0); if (amt <= 0) return
    const line = { key: uuid(), product: { id, name: t('pos.tip'), price: amt, tax_ids: [], category_ids: [] } as Product, qty: 1, price: amt, discount: 0 }
    setCarts((cs) => cs.map((c, i) => (i === cur ? { ...c, lines: [...c.lines.filter((l) => l.product.id !== id), line] } : c))); setModal(null)
  }
  const sendBytes = async (target: string | null, bytes: Uint8Array, hex: string, via: HwSettings['receipt']) => {
    if (via === 'serial') return writeSerial(bytes)
    if (!target) throw new Error(t('pos.no_printer'))
    await act('pos.config', 'print_raw', [config], { target, data: hex })
  }
  const receiptPrinter = d?.printers?.[0]?.target ?? null
  const printReceipt = async (r: Receipt, open = false) => {
    if (!d || hw.receipt === 'browser') { if (!open) window.print(); return }
    const tk = escReceipt({ name: r.name, date_order: r.date_order, cashier: emp?.name ?? d.cashier, header: String(d.config.receipt_header ?? ''), footer: String(d.config.receipt_footer ?? ''), tax: r.amount_tax, total: r.amount_total, change: r.amount_return, money,
      lines: r.lines.map((l) => ({ name: l.full_product_name, qty: l.qty, price: l.price_unit, discount: l.discount, total: l.price_subtotal_incl })), payments: r.payments.filter((p) => !p.is_change).map((p) => ({ name: p.method, amount: p.amount })) })
    if (open) tk.drawer()
    await sendBytes(receiptPrinter, tk.bytes(), tk.hex(), hw.receipt)
  }
  const kickDrawer = async () => { if (!d || hw.receipt === 'browser') return; const tk = new Ticket().drawer(); await sendBytes(receiptPrinter, tk.bytes(), tk.hex(), hw.receipt) }
  const printKitchen = async (lines: { name: string; qty: number; note?: string; cats: number[] }[]) => {
    if (!d || !hw.kitchen) return
    const label = cart.table ? `${t('pos.table')} ${cart.table.name}` : cart.takeaway ? t('pos.takeaway_order') : `#${carts.indexOf(cart) + 1}`
    for (const p of (d.printers ?? []).slice(1)) {
      const mine = lines.filter((l) => !p.category_ids.length || l.cats.some((c) => p.category_ids.includes(c))); if (!mine.length) continue
      const tk = kitchenTicket(label, mine, new Date().toTimeString().slice(0, 5)); await sendBytes(p.target, tk.bytes(), tk.hex(), 'network')
    }
  }
  const validate = async () => {
    if (!d) return; setBusy(true); setErr('')
    const kwargs = { session_id: d.session.id, uuid: cart.uuid, employee_id: emp?.id, table_id: cart.table?.id, customer_count: cart.guests, takeaway: !!cart.takeaway, partner_id: cart.partner?.id, pricelist_id: cart.pricelist ?? undefined, to_invoice: invoice, note: cart.note, rewards: (cart.claims ?? []).map((c) => ({ reward_id: c.rewardId, card_id: c.cardId ?? undefined })), codes: cart.codes,
      lines: cart.lines.map((l) => ({ uuid: l.key, product_id: l.product.id, qty: l.qty, price_unit: l.price, discount: l.discount, note: l.note })), payments: pays.map((p) => ({ payment_method_id: p.method, amount: p.amount })) }
    try { const rc = await act<Receipt>('pos.order', 'create_from_ui', [], kwargs); setReceipt(rc); setScreen('receipt'); const cash = rc.payments.some((p) => !p.is_change && methods.find((m) => m.name === p.method)?.kind === 'cash'); if (hw.receipt !== 'browser') { const fail = (e: unknown) => setErr(String((e as Error).message ?? e)); if (hw.autoPrint) printReceipt(rc, hw.drawer && cash).catch(fail); else if (hw.drawer && cash) kickDrawer().catch(fail) } }
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

  const verify = useCallback(async (kw: Record<string, unknown>) => { const e = await act<Emp>('pos.config', 'verify_employee', [config], kw); setEmp(e); return e }, [config])
  // badge / RFID readers act as keyboards: while locked, typed characters + Enter are a badge scan
  const badge = useRef({ s: '', at: 0 })
  useEffect(() => {
    if (!d?.cashier_lock || emp) return
    const h = (e: KeyboardEvent) => { if ((e.target as HTMLElement)?.tagName === 'INPUT') return; const now = Date.now(); if (now - badge.current.at > 80) badge.current.s = ''; badge.current.at = now
      if (e.key === 'Enter') { const b = badge.current.s; badge.current.s = ''; if (b.length >= 3) verify({ badge: b }).catch((x) => setErr(String(x.message ?? x))) } else if (e.key.length === 1) badge.current.s += e.key }
    window.addEventListener('keydown', h); return () => window.removeEventListener('keydown', h)
  }, [d?.cashier_lock, emp, verify])
  const basic = emp?.role === 'basic'
  if (err && !d) return <div className="pos-reg"><div className="err">{err}</div><Link className="btn" href="/pos/">←</Link></div>
  if (!d) return <div className="pos-reg muted">{t('common.loading')}</div>
  if (d.cashier_lock && !emp) return <CashierLock d={d} verify={verify} err={err} t={t} />
  const topCats = d.categories.filter((c) => !c.parent_id)
  return (
    <div className="pos">
      <header className="pos-top">
        <Link href="/pos/" className="btn">←</Link>{restaurant && screen !== 'floor' && <button className="btn" onClick={leaveTable}>▦ {t('pos.tables')}</button>}<b>{d.config.name}</b><span className="muted">{d.session.name} · {emp?.name ?? d.cashier}</span>{d.cashier_lock && <button className="btn" onClick={() => setEmp(null)}>⇄</button>}
        <div className="pos-tabs">{carts.map((c, i) => <button key={c.uuid} className={`btn ${i === cur ? 'p' : ''}`} onClick={() => { setCur(i); setSel(null); setScreen('shop'); setPays([]) }}>{c.lines.length ? `#${i + 1} · ${money(cartTotals(c, taxes).total)}` : `#${i + 1}`}</button>)}<button className="btn" onClick={newOrder}>+</button></div>
        <span className="grow" />
        <span className={`pill ${online ? 'ok' : 'bad'}`}>{online ? t('pos.online') : t('pos.offline')}{queued > 0 && ` · ${queued}`}</span>
        <button className="btn" onClick={() => setModal('hardware')} title={t('pos.hardware')}>⚙</button><button className="btn" onClick={() => setModal('orders')}>{t('pos.orders')}</button>{!basic && <button className="btn" onClick={() => setModal('close')}>{t('pos.close')}</button>}
      </header>
      {err && <div className="err" onClick={() => setErr('')}>{err}</div>}
      {toast && <div className="pos-toast">{toast}</div>}
      {screen === 'floor' && (
        <div className="pos-floor">
          <div className="pos-cats">{d.floors.map((f, i) => <button key={f.id} className={`btn ${i === floorIdx ? 'p' : ''}`} onClick={() => setFloorIdx(i)}>{f.name}</button>)}<span className="grow" /><button className="btn" onClick={() => { setCarts((cs) => [...cs, emptyCart()]); setCur(carts.length); setScreen('shop') }}>🥡 {t('pos.takeaway_order')}</button></div>
          <div className="pos-plan">
            {(d.floors[floorIdx]?.tables ?? []).map((tb, i) => {
              const placed = tb.position_h > 0 || tb.position_v > 0
              return <button key={tb.id} className={`pos-table ${tb.shape === 'round' ? 'round' : ''} ${tb.orders ? 'busy' : ''}`} style={placed ? { left: tb.position_h, top: tb.position_v, width: tb.width || 90, height: tb.height || 90 } : { position: 'relative', width: 90, height: 90, margin: 8 }} onClick={() => openTable(tb)} data-i={i}><b>{tb.name}</b><small>{tb.orders ? money(tb.total) : `${tb.seats} 👤`}</small></button>
            })}
          </div>
        </div>)}
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
                  {restaurant && cart.table && l.qty > (cart.sent?.[l.key] ?? 0) && <small className="pos-unsent">● {t('pos.not_sent')}</small>}
                </div>))}
            </div>
            {rlines.map((l, i) => <div key={i} className="pos-line reward"><b>🎁 {l.label}</b><span>{money(l.price)}</span><button className="btn" onClick={() => dispatch({ t: 'unclaim', index: (cart.claims ?? []).findIndex((c) => c.rewardId === l.rewardId && (c.cardId ?? null) === (l.cardId ?? null)) })}>×</button></div>)}
            {earned.length > 0 && <div className="pos-earn muted">{earned.map((x) => `+${x.pts} ${x.p.name}`).join(' · ')}</div>}
            <div className="pos-sum"><span>{t('pos.taxes')}</span><span>{money(totals.tax)}</span><b>{t('pos.total')}</b><b className="pos-big">{money(total)}</b></div>
            <div className="pos-actions">
              <button className="btn" onClick={() => setModal('customer')}>👤 {cart.partner?.name ?? t('pos.customer')}</button>
              {restaurant && cart.table && <button className="btn p" onClick={sendKitchen} disabled={!hasUnsent(cart)}>🍳 {t('pos.order_kitchen')}{hasUnsent(cart) ? ` (${kitchenDelta(cart).added.reduce((a, x) => a + x.qty, 0) + kitchenDelta(cart).removed.length})` : ''}</button>}
              {restaurant && cart.lines.length > 0 && <button className="btn" onClick={bill}>🧾 {t('pos.bill')}</button>}
              {restaurant && cart.lines.length > 1 && <button className="btn" onClick={() => setModal('split')}>✂ {t('pos.split')}</button>}
              {restaurant && cart.table && <button className="btn" onClick={() => setModal('transfer')}>⇄ {t('pos.transfer')}</button>}
              {restaurant && !!d.config.tip_product_id && cart.lines.length > 0 && <button className="btn" onClick={() => setModal('tip')}>💬 {t('pos.tip')}</button>}
              {restaurant && <button className={`btn ${cart.takeaway ? 'p' : ''}`} onClick={() => dispatch({ t: 'meta', meta: { takeaway: !cart.takeaway } })}>🥡</button>}
              {d.loyalty.length > 0 && <button className="btn" onClick={() => setModal('rewards')}>🎁 {t('pos.rewards')}{(cart.claims?.length ?? 0) > 0 ? ` (${cart.claims!.length})` : ''}</button>}
              {d.config.use_pricelist ? <button className="btn" onClick={() => setModal('pricelist')}>🏷 {plist?.name ?? t('pos.pricelist')}</button> : null}
              {selLine && <button className="btn" onClick={() => { const n = prompt(t('pos.note'), selLine.note ?? ''); if (n !== null) dispatch({ t: 'note', key: selLine.key, note: n }) }}>📝</button>}
            </div>
            <div className="pos-pad">
              <div className="pos-modes">{(['qty', 'discount', 'price'] as const).filter((m) => m === 'qty' || (!basic && (m === 'discount' ? !!d.config.manual_discount : !d.config.restrict_price_control))).map((m) => <button key={m} className={`btn ${mode === m ? 'p' : ''}`} onClick={() => { setMode(m); setBuf('') }}>{t(`pos.${m}`)}</button>)}</div>
              {['1', '2', '3', '4', '5', '6', '7', '8', '9', '±', '0', '.', 'C', '⌫'].map((k) => <button key={k} className="btn" onClick={() => key(k)}>{k}</button>)}
              <button className="btn p pos-pay" disabled={!cart.lines.length} onClick={() => { setPays([]); setAmount(''); setScreen('pay') }}>{t('pos.payment')} ›</button>
            </div>
          </section>
          <section className="pos-catalog">
            <div className="pos-search"><input placeholder={t('pos.search')} value={q} onChange={(e) => setQ(e.target.value)} />
              <div className="pos-cats"><button className={`btn ${cat === null ? 'p' : ''}`} onClick={() => setCat(null)}>{t('pos.all')}</button>{topCats.map((c) => <button key={c.id} className={`btn ${cat === c.id ? 'p' : ''}`} onClick={() => setCat(c.id)}>{c.name}</button>)}</div></div>
            <div className="pos-grid">{shown.map((p) => <button key={p.id} className="card pos-prod" onClick={() => { if (p.to_weight) { setWeighing(p); setModal('scale') } else { dispatch({ t: 'add', product: p, pl: plist }); setSel(null) } }}><b>{p.name}</b><span className="muted">{money(p.price)}</span></button>)}{shown.length === 0 && <p className="muted">{t('pos.no_products')}</p>}</div>
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
      {screen === 'receipt' && receipt && <ReceiptView r={receipt} d={d} money={money} done={finish} print={() => printReceipt(receipt).catch((e) => setErr(String(e.message ?? e)))} t={t} />}
      {modal === 'guests' && pending && <GuestsModal tb={pending.table} go={(n) => loadTable(pending.table, null, n)} close={() => { setModal(null); setPending(null) }} t={t} />}
      {modal === 'tableorders' && pending && <div className="pos-modal" onClick={() => { setModal(null); setPending(null) }}><div className="card" onClick={(e) => e.stopPropagation()}><h3>{t('pos.table')} {pending.table.name}</h3>{pending.orders.map((o, i) => <button key={o.uuid} className="btn" onClick={() => loadTable(pending.table, o)}>#{i + 1} · {o.lines.length} · {money(Number((o as unknown as { amount_total: number }).amount_total ?? 0))}</button>)}<button className="btn p" onClick={() => setModal('guests')}>+ {t('pos.new_order')}</button></div></div>}
      {modal === 'split' && <div className="pos-modal" onClick={() => setModal(null)}><div className="card" onClick={(e) => e.stopPropagation()}><SplitPick cart={cart} money={money} taxes={taxes} done={split} t={t} /></div></div>}
      {modal === 'transfer' && <div className="pos-modal" onClick={() => setModal(null)}><div className="card" onClick={(e) => e.stopPropagation()}><h3>{t('pos.transfer')}</h3><div className="pos-list">{d.floors.flatMap((f) => f.tables.map((tb) => ({ f, tb }))).filter((x) => x.tb.id !== cart.table?.id).map(({ f, tb }) => <button key={tb.id} className="btn" onClick={() => transfer(tb)}>{f.name} · {tb.name}{tb.orders ? ' ●' : ''}</button>)}</div></div></div>}
      {modal === 'tip' && <div className="pos-modal" onClick={() => setModal(null)}><div className="card" onClick={(e) => e.stopPropagation()}><h3>{t('pos.tip')}</h3><div className="row" style={{ gap: 8, flexWrap: 'wrap' }}>{[10, 15, 20].map((p) => <button key={p} className="btn" onClick={() => addTip(p)}>{p}% · {money(tipAmount(base.total, p))}</button>)}<button className="btn" onClick={() => { const v = Number(prompt(t('pos.tip')) || 0); if (v > 0) addTip(null, v) }}>…</button></div></div></div>}
      {modal === 'hardware' && <HardwareModal hw={hw} update={updateHw} printers={d.printers ?? []} close={() => setModal(null)} test={() => { const tk = new Ticket().align('center').bold(true).line('Odoo RS').bold(false).line(t('pos.test_print')).cut(); return sendBytes(receiptPrinter, tk.bytes(), tk.hex(), hw.receipt) }} t={t} />}
      {modal === 'scale' && weighing && <ScaleModal p={weighing} money={money} close={() => { setModal(null); setWeighing(null) }} add={(kg) => { setCarts((cs) => cs.map((c, i) => { if (i !== cur) return c; const a = reduce(c, { t: 'add', product: weighing, pl: plist }); return { ...a, lines: a.lines.map((l, j) => (j === a.lines.length - 1 ? { ...l, qty: kg } : l)) } })); setModal(null); setWeighing(null) }} t={t} />}
      {modal === 'rewards' && <Rewards d={d} config={config} cart={cart} cards={cards} setCards={setCards} items={litems} claim={(c) => { dispatch({ t: 'claim', claim: c }); setModal(null) }} code={(c) => dispatch({ t: 'code', code: c })} t={t} close={() => setModal(null)} />}
      {modal === 'customer' && <Customers close={() => setModal(null)} pick={(p) => { dispatch({ t: 'partner', partner: p }); setModal(null) }} t={t} />}
      {modal === 'pricelist' && <div className="pos-modal" onClick={() => setModal(null)}><div className="card" onClick={(e) => e.stopPropagation()}><h3>{t('pos.pricelist')}</h3>{[{ id: 0, name: t('pos.default'), items: [] } as Pricelist, ...d.pricelists].map((p) => <button key={p.id} className="btn" onClick={() => { dispatch({ t: 'pricelist', id: p.id || null, list: p.id ? p : null }); setModal(null) }}>{p.name}</button>)}</div></div>}
      {modal === 'orders' && <Orders d={d} emp={emp} money={money} close={() => setModal(null)} t={t} onRefund={(r) => { setReceipt(r); setModal(null); setScreen('receipt') }} />}
      {modal === 'close' && <Close d={d} emp={emp} money={money} close={() => setModal(null)} t={t} />}
    </div>
  )
}

function ReceiptView({ r, d, money, done, print, t }: { r: Receipt; d: Data; money: (n: number) => string; done: () => void; print: () => void; t: TFn }) {
  return (
    <div className="pos-receipt-wrap">
      <div className="pos-receipt card" id="pos-receipt">
        {!!d.config.receipt_header && <p style={{ whiteSpace: 'pre-line', textAlign: 'center' }}>{String(d.config.receipt_header)}</p>}
        <h3 style={{ textAlign: 'center' }}>{r.name}{r.offline ? ` (${t('pos.offline')})` : ''}</h3><p className="muted" style={{ textAlign: 'center' }}>{r.date_order} · {d.cashier}</p>
        {r.lines.map((l, i) => <div key={i} className="pos-rl"><span>{l.qty} × {l.full_product_name}{l.discount ? ` (-${l.discount}%)` : ''}</span><span>{money(l.price_subtotal_incl)}</span></div>)}
        <hr /><div className="pos-rl"><span>{t('pos.taxes')}</span><span>{money(r.amount_tax)}</span></div><div className="pos-rl"><b>{t('pos.total')}</b><b>{money(r.amount_total)}</b></div>
        {r.payments.filter((p) => !p.is_change).map((p, i) => <div key={i} className="pos-rl"><span>{p.method}</span><span>{money(p.amount)}</span></div>)}
        {(r.loyalty_issued ?? []).map((c, i) => <p key={i} style={{ textAlign: 'center' }}>🎁 {c.program}: <b>{c.points}</b>{c.type !== 'loyalty' && <><br /><code>{c.code}</code></>}</p>)}
        {r.amount_return > 0 && <div className="pos-rl"><span>{t('pos.change')}</span><span>{money(r.amount_return)}</span></div>}
        {!!d.config.receipt_footer && <p style={{ whiteSpace: 'pre-line', textAlign: 'center' }}>{String(d.config.receipt_footer)}</p>}
      </div>
      <div className="pos-receipt-actions"><button className="btn" onClick={() => print()}>🖨 {t('pos.print')}</button><button className="btn p" onClick={done}>{t('pos.new_order')} ›</button></div>
    </div>
  )
}

function Customers({ pick, close, t }: { pick: (p: { id: number; name: string } | null) => void; close: () => void; t: TFn }) {
  const [q, setQ] = useState(''); const [rows, setRows] = useState<{ id: number; name: string; email?: string }[]>([])
  useEffect(() => { const h = setTimeout(() => call<typeof rows>('res.partner', 'search_read', [q ? [['name', 'ilike', q]] : []], { fields: ['name', 'email'], limit: 30, order: 'name' }).then(setRows).catch(() => setRows([])), 150); return () => clearTimeout(h) }, [q])
  return <div className="pos-modal" onClick={close}><div className="card" onClick={(e) => e.stopPropagation()}><h3>{t('pos.customer')}</h3><input autoFocus placeholder={t('pos.search_customers')} value={q} onChange={(e) => setQ(e.target.value)} />
    <div className="pos-list"><button className="btn" onClick={() => pick(null)}>— {t('pos.no_customer')}</button>{rows.map((r) => <button key={r.id} className="btn" onClick={() => pick({ id: r.id, name: r.name })}>{r.name} <span className="muted">{r.email || ''}</span></button>)}</div></div></div>
}

function Orders({ d, emp, money, close, onRefund, t }: { d: Data; emp: Emp | null; money: (n: number) => string; close: () => void; onRefund: (r: Receipt) => void; t: TFn }) {
  const [rows, setRows] = useState<{ id: number; name: string; amount_total: number; state: string; date_order: string }[]>([]); const [err, setErr] = useState('')
  useEffect(() => { call<typeof rows>('pos.order', 'search_read', [[['session_id', '=', d.session.id]]], { fields: ['name', 'amount_total', 'state', 'date_order'], order: 'id desc', limit: 50 }).then(setRows).catch((e) => setErr(String(e.message ?? e))) }, [d.session.id])
  const refund = async (id: number) => { try { onRefund(await act<Receipt>('pos.order', 'refund', [id], { session_id: d.session.id, employee_id: emp?.id })) } catch (e) { setErr(String((e as Error).message ?? e)) } }
  return <div className="pos-modal" onClick={close}><div className="card" onClick={(e) => e.stopPropagation()}><h3>{t('pos.orders')}</h3>{err && <div className="err">{err}</div>}
    <div className="pos-list">{rows.map((r) => <div key={r.id} className="pos-payrow"><span>{r.name}<br /><small className="muted">{r.date_order}</small></span><b>{money(r.amount_total)}</b>{r.amount_total > 0 && emp?.role !== 'basic' && <button className="btn" onClick={() => refund(r.id)}>↩ {t('pos.refund')}</button>}</div>)}{rows.length === 0 && <p className="muted">{t('pos.no_orders')}</p>}</div></div></div>
}

function Close({ d, emp, money, close, t }: { d: Data; emp: Emp | null; money: (n: number) => string; close: () => void; t: TFn }) {
  const [s, setS] = useState<Summary | null>(null); const [counted, setCounted] = useState(''); const [res, setRes] = useState<Summary | null>(null); const [err, setErr] = useState('')
  useEffect(() => { act<Summary>('pos.session', 'summary', [d.session.id], {}).then(setS).catch((e) => setErr(String(e.message ?? e))) }, [d.session.id])
  const go = async (force = false) => { try { setRes(await act<Summary>('pos.session', 'close_session', [d.session.id], { counted_cash: counted === '' ? undefined : Number(counted), force, employee_id: emp?.id })) } catch (e) { setErr(String((e as Error).message ?? e)) } }
  return <div className="pos-modal" onClick={close}><div className="card" onClick={(e) => e.stopPropagation()}><h3>{t('pos.close_register')} · {d.session.name}</h3>{err && <div className="err">{err}</div>}
    {s && <><div className="pos-rl"><span>{t('pos.orders')}</span><b>{s.orders}</b></div><div className="pos-rl"><span>{t('pos.total')}</span><b>{money(s.total)}</b></div>{s.refunds !== 0 && <div className="pos-rl"><span>{t('pos.refund')}</span><b>{money(s.refunds)}</b></div>}
      {s.payments.map((p) => <div key={p.name} className="pos-rl"><span>{p.name}</span><span>{money(p.amount)}</span></div>)}<hr /><div className="pos-rl"><span>{t('pos.expected_cash')}</span><b>{money(s.expected_cash)}</b></div></>}
    {!res ? <><label>{t('pos.counted_cash')}<input inputMode="decimal" value={counted} onChange={(e) => setCounted(e.target.value)} /></label><div className="row" style={{ gap: 8, marginTop: 10 }}><button className="btn" onClick={close}>{t('pos.back')}</button><button className="btn p" onClick={() => go()}>{t('pos.close_register')}</button></div></>
      : <><div className="pos-rl"><span>{t('pos.difference')}</span><b>{res.difference == null ? '—' : money(res.difference)}</b></div><button className="btn p" onClick={() => { location.href = '/pos/' }}>{t('pos.done')}</button></>}</div></div>
}

function Rewards({ d, config, cart, cards, setCards, items, claim, code, close, t }: { d: Data; config: number; cart: Cart; cards: Card[]; setCards: (f: (c: Card[]) => Card[]) => void; items: LItem[]; claim: (c: { rewardId: number; cardId?: number | null }) => void; code: (c: string) => void; close: () => void; t: TFn }) {
  const [c, setC] = useState(''); const [msg, setMsg] = useState('')
  const lookup = async () => {
    const v = c.trim(); if (!v) return; setMsg('')
    const found = await act<Card[]>('pos.config', 'loyalty_cards', [config], { code: v }).catch(() => [] as Card[])
    if (found.length) { setCards((o) => [...found, ...o.filter((x) => !found.some((f) => f.id === x.id))]); setC('') }
    else if (d.loyalty.some((p) => p.rules.some((r) => (r.code ?? '').toLowerCase() === v.toLowerCase()))) { code(v); setC('') }
    else setMsg(t('pos.code_unknown'))
  }
  const rows: { key: string; label: string; sub: string; ok: boolean; claim: { rewardId: number; cardId?: number | null } }[] = []
  for (const p of d.loyalty) {
    const mine = cards.filter((k) => k.program_id === p.id && (!k.partner_id || k.partner_id === cart.partner?.id))
    const sources: { card?: Card; balance: number }[] = mine.length ? mine.map((k) => ({ card: k, balance: k.points })) : (['promotion', 'buy_x_get_y', 'promo_code'].includes(p.program_type) || p.applies_on === 'current' ? [{ balance: programPoints(p, items, cart.codes ?? []) }] : [])
    for (const s of sources) for (const r of p.rewards) rows.push({ key: `${p.id}-${r.id}-${s.card?.id ?? 'o'}`, label: r.description || p.name, sub: s.card ? `${p.name} · ${s.card.points} pts${s.card.code && p.program_type !== 'loyalty' ? ` · ${s.card.code.slice(-6)}` : ''}` : p.name, ok: claimable(r, s.balance) && !(cart.claims ?? []).some((x) => x.rewardId === r.id && (x.cardId ?? null) === (s.card?.id ?? null)), claim: { rewardId: r.id, cardId: s.card?.id } })
  }
  return <div className="pos-modal" onClick={close}><div className="card" onClick={(e) => e.stopPropagation()}><h3>🎁 {t('pos.rewards')}</h3>
    <div className="row" style={{ gap: 6 }}><input placeholder={t('pos.enter_code')} value={c} onChange={(e) => setC(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && lookup()} /><button className="btn" onClick={lookup}>{t('pos.apply')}</button></div>
    {msg && <div className="err">{msg}</div>}
    {!cart.partner && d.loyalty.some((p) => p.is_nominative || p.program_type === 'loyalty') && <p className="muted">{t('pos.pick_customer_points')}</p>}
    <div className="pos-list">{rows.map((r) => <button key={r.key} className="btn pos-reward" disabled={!r.ok} onClick={() => claim(r.claim)}><b>{r.label}</b><small className="muted">{r.sub}</small></button>)}{rows.length === 0 && <p className="muted">{t('pos.no_rewards')}</p>}</div></div></div>
}

function CashierLock({ d, verify, err, t }: { d: Data; verify: (kw: Record<string, unknown>) => Promise<unknown>; err: string; t: TFn }) {
  const [pick, setPick] = useState<Data['employees'][number] | null>(null); const [pin, setPin] = useState(''); const [msg, setMsg] = useState(err)
  const go = async (e: Data['employees'][number], p?: string) => { try { await verify({ employee_id: e.id, pin: p }) } catch (x) { setMsg(String((x as Error).message ?? x)); setPin('') } }
  const press = (k: string) => { const n = k === '⌫' ? pin.slice(0, -1) : (pin + k).slice(0, 8); setPin(n) }
  return (
    <div className="pos-lock card"><h2>{t('pos.who')}</h2><p className="muted">{t('pos.scan_badge')}</p>{msg && <div className="err">{msg}</div>}
      {!pick ? <div className="pos-grid">{d.employees.map((e) => <button key={e.id} className="card pos-prod" onClick={() => (e.has_pin ? (setPick(e), setMsg('')) : go(e))}><b>{e.name}</b><span className="muted">{e.role === 'advanced' ? t('pos.manager') : t('pos.cashier')}</span></button>)}</div>
        : <div className="pos-pinpad"><h3>{pick.name}</h3><div className="pos-pin">{'•'.repeat(pin.length) || '—'}</div>
          <div className="pos-pad">{['1', '2', '3', '4', '5', '6', '7', '8', '9', '⌫', '0', '✓'].map((k) => <button key={k} className="btn" onClick={() => (k === '✓' ? go(pick, pin) : press(k))}>{k}</button>)}</div>
          <button className="btn" onClick={() => { setPick(null); setPin('') }}>‹ {t('pos.back')}</button></div>}
    </div>
  )
}

function GuestsModal({ tb, go, close, t }: { tb: Table; go: (n: number) => void; close: () => void; t: TFn }) {
  const [n, setN] = useState(Math.min(2, tb.seats || 2))
  return <div className="pos-modal" onClick={close}><div className="card" onClick={(e) => e.stopPropagation()}><h3>{t('pos.table')} {tb.name} · {t('pos.guests')}</h3>
    <div className="row" style={{ gap: 12, justifyContent: 'center' }}><button className="btn" onClick={() => setN(Math.max(1, n - 1))}>−</button><b className="pos-big">{n}</b><button className="btn" onClick={() => setN(n + 1)}>+</button></div>
    <button className="btn p" onClick={() => go(n)}>{t('pos.open_table')}</button></div></div>
}

function SplitPick({ cart, money, taxes, done, t }: { cart: Cart; money: (n: number) => string; taxes: Map<number, Tax>; done: (keys: string[]) => void; t: TFn }) {
  const [keys, setKeys] = useState<string[]>([])
  return <><h3>{t('pos.split')}</h3><p className="muted">{t('pos.split_hint')}</p>
    <div className="pos-list">{cart.lines.map((l) => <label key={l.key} className="pos-payrow"><span><input type="checkbox" checked={keys.includes(l.key)} onChange={(e) => setKeys((k) => (e.target.checked ? [...k, l.key] : k.filter((x) => x !== l.key)))} /> {l.qty} × {l.product.name}</span><b>{money(lineTotals(l, taxes).total)}</b></label>)}</div>
    <button className="btn p" disabled={keys.length === 0 || keys.length === cart.lines.length} onClick={() => done(keys)}>{t('pos.split')}</button></>
}

function HardwareModal({ hw, update, printers, close, test, t }: { hw: HwSettings; update: (p: Partial<HwSettings>) => void; printers: Data['printers']; close: () => void; test: () => Promise<void>; t: TFn }) {
  const [msg, setMsg] = useState(''); const [, bump] = useState(0)
  const connect = async (kind: 'receipt' | 'scale', baud: number) => { try { await connectSerial(kind, baud); setMsg(t('pos.connected')); bump((x) => x + 1) } catch (e) { setMsg(String((e as Error).message ?? e)) } }
  return <div className="pos-modal" onClick={close}><div className="card" onClick={(e) => e.stopPropagation()}><h3>⚙ {t('pos.hardware')}</h3>{msg && <div className="muted">{msg}</div>}
    <label>{t('pos.receipt_printer')}<select value={hw.receipt} onChange={(e) => update({ receipt: e.target.value as HwSettings['receipt'] })}>
      <option value="browser">{t('pos.hw_browser')}</option><option value="network" disabled={!printers.length}>{t('pos.hw_network')}{printers[0] ? ` (${printers[0].target})` : ` — ${t('pos.hw_not_configured')}`}</option><option value="serial" disabled={!serialSupported()}>{t('pos.hw_serial')}</option></select></label>
    {hw.receipt === 'serial' && <button className="btn" onClick={() => connect('receipt', 9600)}>{serialConnected('receipt') ? '✓ ' : ''}{t('pos.connect_printer')}</button>}
    <label className="row" style={{ gap: 6 }}><input type="checkbox" checked={hw.autoPrint} onChange={(e) => update({ autoPrint: e.target.checked })} /> {t('pos.auto_print')}</label>
    <label className="row" style={{ gap: 6 }}><input type="checkbox" checked={hw.drawer} onChange={(e) => update({ drawer: e.target.checked })} /> {t('pos.open_drawer')}</label>
    <label className="row" style={{ gap: 6 }}><input type="checkbox" checked={hw.kitchen} onChange={(e) => update({ kitchen: e.target.checked })} /> {t('pos.print_kitchen')} ({Math.max(0, printers.length - 1)})</label>
    <label className="row" style={{ gap: 6 }}><input type="checkbox" checked={hw.scale} onChange={(e) => update({ scale: e.target.checked })} /> {t('pos.use_scale')}</label>
    {hw.scale && <button className="btn" onClick={() => connect('scale', 9600)} disabled={!serialSupported()}>{serialConnected('scale') ? '✓ ' : ''}{t('pos.connect_scale')}</button>}
    <div className="row" style={{ gap: 8 }}><button className="btn" onClick={() => test().then(() => setMsg('✓')).catch((e) => setMsg(String(e.message ?? e)))}>{t('pos.test_print')}</button><button className="btn p" onClick={close}>{t('pos.done')}</button></div></div></div>
}

function ScaleModal({ p, money, close, add, t }: { p: Product; money: (n: number) => string; close: () => void; add: (kg: number) => void; t: TFn }) {
  const [kg, setKg] = useState(''); const [msg, setMsg] = useState(''); const [busy, setBusy] = useState(false)
  const read = async () => { setBusy(true); setMsg(''); try { setKg(String(await readWeight())) } catch (e) { setMsg(String((e as Error).message ?? e)) } finally { setBusy(false) } }
  useEffect(() => { if (serialConnected('scale')) read() }, []) // eslint-disable-line react-hooks/exhaustive-deps
  const n = Number(kg)
  return <div className="pos-modal" onClick={close}><div className="card" onClick={(e) => e.stopPropagation()}><h3>⚖ {p.name} · {money(p.price)}/kg</h3>{msg && <div className="err">{msg}</div>}
    <input autoFocus inputMode="decimal" placeholder="kg" value={kg} onChange={(e) => setKg(e.target.value)} />
    <div className="pos-sum"><b>{t('pos.total')}</b><b className="pos-big">{money((Number.isFinite(n) ? n : 0) * p.price)}</b></div>
    <div className="row" style={{ gap: 8 }}>{serialConnected('scale') && <button className="btn" disabled={busy} onClick={read}>⚖ {t('pos.weigh')}</button>}<button className="btn p" disabled={!(n > 0)} onClick={() => add(n)}>{t('pos.add')}</button></div></div></div>
}
