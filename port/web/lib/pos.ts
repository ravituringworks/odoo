// Point of Sale: pure cart / pricing / payment logic (no I/O). The server recomputes everything on `create_from_ui`.
export type Tax = { id: number; name?: string; amount: number; amount_type: string; price_include: boolean }
export type PricelistItem = { applied_on: string; compute_price: string; fixed_price: number; percent_price: number; min_quantity: number; product_tmpl_id: unknown; product_id: unknown; categ_id: unknown }
export type Pricelist = { id: number; name: string; items: PricelistItem[] }
export type Product = { id: number; name: string; price: number; code?: string | null; barcode?: string | null; tax_ids: number[]; category_ids: number[]; categ_chain?: number[]; to_weight?: boolean; type?: string; tracking?: 'none' | 'lot' | 'serial'; combo?: ComboGroup[] }
export type ComboItem = { id: number; product_id: number; name: string; extra: number; tax_ids: number[] }
export type ComboGroup = { id: number; name: string; items: ComboItem[] }
export type Line = { key: string; product: Product; qty: number; price: number; discount: number; note?: string; lots?: string[]; combo?: { parent: string; item: number } }
export type Method = { id: number; name: string; kind: 'cash' | 'bank' | 'account'; use_payment_terminal?: string | false | null }
export type Pay = { method: number; amount: number; tx?: string; card?: string }
export type Claim = { rewardId: number; cardId?: number | null }
export type Cart = { lines: Line[]; partner?: { id: number; name: string } | null; pricelist?: number | null; note?: string; uuid: string; claims?: Claim[]; codes?: string[]; table?: { id: number; name: string } | null; guests?: number; takeaway?: boolean; sent?: Record<string, number> }

export const r2 = (x: number) => Math.round(x * 100 + (x < 0 ? -1e-9 : 1e-9)) / 100
export const uuid = () => (globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(16).slice(2)}`)
export const emptyCart = (): Cart => ({ lines: [], uuid: uuid() })

/** Same rules as the server's `tax::compute`: percent / fixed / division, with price-included taxes. */
export function computeTax(qty: number, price: number, discount: number, taxes: Tax[]): { untaxed: number; tax: number; total: number } {
  const gross = qty * price * (1 - discount / 100)
  let fixedInc = 0, pctInc = 0
  for (const t of taxes) if (t.price_include) { if (t.amount_type === 'fixed') fixedInc += t.amount * qty; else if (t.amount_type === 'percent') pctInc += t.amount }
  const base = fixedInc !== 0 || pctInc !== 0 ? (gross - fixedInc) / (1 + pctInc / 100) : gross
  const tax = taxes.reduce((s, t) => s + (t.amount_type === 'fixed' ? t.amount * qty : t.amount_type === 'division' ? base / (1 - t.amount / 100) - base : (base * t.amount) / 100), 0)
  return { untaxed: r2(base), tax: r2(tax), total: r2(base + tax) }
}

const refId = (v: unknown): number | null => (Array.isArray(v) ? (v[0] as number) : typeof v === 'number' ? v : null)
const rank: Record<string, number> = { '0_product_variant': 0, '1_product': 1, '2_product_category': 2, '3_global': 3 }

/** Best pricelist price for `qty` units: most specific matching rule wins; falls back to the list price. */
export function priceFor(p: Product, qty: number, pl?: Pricelist | null): number {
  if (!pl) return p.price
  const hit = pl.items
    .filter((i) => qty >= (i.min_quantity || 0) && (
      i.applied_on === '3_global' ||
      (i.applied_on === '0_product_variant' && refId(i.product_id) === p.id) ||
      (i.applied_on === '1_product' && refId(i.product_tmpl_id) !== null && refId(i.product_tmpl_id) === (p as { tmpl?: number }).tmpl) ||
      (i.applied_on === '2_product_category' && p.category_ids.includes(refId(i.categ_id) ?? -1))))
    .sort((a, b) => (rank[a.applied_on] ?? 9) - (rank[b.applied_on] ?? 9) || (b.min_quantity || 0) - (a.min_quantity || 0))[0]
  if (!hit) return p.price
  if (hit.compute_price === 'fixed') return hit.fixed_price
  if (hit.compute_price === 'percentage') return r2(p.price * (1 - hit.percent_price / 100))
  return p.price
}

export const lineTotals = (l: Line, taxes: Map<number, Tax>) => computeTax(l.qty, l.price, l.discount, l.product.tax_ids.map((i) => taxes.get(i)).filter((t): t is Tax => !!t))
export function cartTotals(c: Cart, taxes: Map<number, Tax>) {
  const t = c.lines.reduce((s, l) => { const x = lineTotals(l, taxes); return { untaxed: s.untaxed + x.untaxed, tax: s.tax + x.tax, total: s.total + x.total } }, { untaxed: 0, tax: 0, total: 0 })
  return { untaxed: r2(t.untaxed), tax: r2(t.tax), total: r2(t.total) }
}

/** Round to the nearest `step` (cash rounding, e.g. 0.05). */
export const roundTo = (x: number, step: number) => (step > 0 ? r2(Math.round(x / step) * step) : r2(x))

// ---- cart reducer (immutable) ----
export type Act =
  | { t: 'add'; product: Product; pl?: Pricelist | null }
  | { t: 'qty'; key: string; qty: number; pl?: Pricelist | null }
  | { t: 'price'; key: string; price: number }
  | { t: 'discount'; key: string; pct: number }
  | { t: 'order_discount'; pct: number }
  | { t: 'note'; key: string; note: string }
  | { t: 'remove'; key: string }
  | { t: 'partner'; partner: Cart['partner'] }
  | { t: 'addCombo'; product: Product; picks: ComboItem[]; pl?: Pricelist | null }
  | { t: 'lots'; key: string; lots: string[] }
  | { t: 'meta'; meta: Partial<Cart> }
  | { t: 'move'; keys: string[] }
  | { t: 'claim'; claim: Claim }
  | { t: 'unclaim'; index: number }
  | { t: 'code'; code: string }
  | { t: 'pricelist'; id: number | null; list?: Pricelist | null; products?: Product[] }
export function reduce(c: Cart, a: Act): Cart {
  const upd = (key: string, f: (l: Line) => Line): Cart => ({ ...c, lines: c.lines.map((l) => (l.key === key ? f(l) : l)) })
  switch (a.t) {
    case 'add': {
      const same = c.lines.find((l) => l.product.id === a.product.id && l.discount === 0 && !l.note && !a.product.to_weight)
      if (same) return reduce(c, { t: 'qty', key: same.key, qty: same.qty + 1, pl: a.pl })
      return { ...c, lines: [...c.lines, { key: uuid(), product: a.product, qty: 1, price: priceFor(a.product, 1, a.pl), discount: 0 }] }
    }
    case 'addCombo': {
      const parent: Line = { key: uuid(), product: a.product, qty: 1, price: priceFor(a.product, 1, a.pl), discount: 0 }
      const kids: Line[] = a.picks.map((p) => ({ key: uuid(), product: { id: p.product_id, name: p.name, price: p.extra, tax_ids: p.tax_ids, category_ids: [] } as Product, qty: 1, price: p.extra, discount: 0, combo: { parent: parent.key, item: p.id } }))
      return { ...c, lines: [...c.lines, parent, ...kids] }
    }
    case 'lots': return upd(a.key, (l) => ({ ...l, lots: a.lots }))
    case 'qty': {
      if (c.lines.find((l) => l.key === a.key)?.combo) return c                      // combo items follow their combo
      return { ...c, lines: c.lines.map((l) => (l.key === a.key ? { ...l, qty: a.qty, price: l.price === priceFor(l.product, l.qty, a.pl) ? priceFor(l.product, a.qty, a.pl) : l.price } : l.combo?.parent === a.key ? { ...l, qty: a.qty } : l)) }
    }
    case 'price': return upd(a.key, (l) => ({ ...l, price: Math.max(0, a.price) }))
    case 'discount': return upd(a.key, (l) => ({ ...l, discount: Math.min(100, Math.max(0, a.pct)) }))
    case 'order_discount': return { ...c, lines: c.lines.map((l) => ({ ...l, discount: Math.min(100, Math.max(0, a.pct)) })) }
    case 'note': return upd(a.key, (l) => ({ ...l, note: a.note }))
    case 'remove': return { ...c, lines: c.lines.filter((l) => l.key !== a.key && l.combo?.parent !== a.key && !(l.key === a.key)) .filter((l) => !(c.lines.find((x) => x.key === a.key)?.combo)) }
    case 'partner': return { ...c, partner: a.partner, claims: (c.claims ?? []).filter((x) => x.cardId == null) }   // a card belongs to the customer: drop card claims on change
    case 'meta': return { ...c, ...a.meta }
    case 'move': return { ...c, lines: c.lines.filter((l) => !a.keys.includes(l.key)) }
    case 'claim': return { ...c, claims: [...(c.claims ?? []), a.claim] }
    case 'unclaim': return { ...c, claims: (c.claims ?? []).filter((_, i) => i !== a.index) }
    case 'code': return (c.codes ?? []).includes(a.code) ? c : { ...c, codes: [...(c.codes ?? []), a.code] }
    case 'pricelist': return { ...c, pricelist: a.id, lines: c.lines.map((l) => ({ ...l, price: priceFor(l.product, l.qty, a.list) })) }
  }
}

// ---- payments ----
export const paid = (ps: Pay[]) => r2(ps.reduce((s, p) => s + p.amount, 0))
export const due = (total: number, ps: Pay[]) => r2(total - paid(ps))
/** Change owed back: only meaningful when over-tendered; capped by what was paid in cash. */
export function change(total: number, ps: Pay[], methods: Method[]): number {
  const over = r2(paid(ps) - total)
  const cash = ps.filter((p) => methods.find((m) => m.id === p.method)?.kind === 'cash').reduce((s, p) => s + p.amount, 0)
  return over > 0 && over <= r2(cash) ? over : 0
}
export const canValidate = (c: Cart, total: number, ps: Pay[], methods: Method[], invoice: boolean) =>
  c.lines.length > 0 && due(total, ps) <= 0.0001 && (due(total, ps) >= 0 || change(total, ps, methods) > 0) &&
  (!invoice || !!c.partner) && !ps.some((p) => methods.find((m) => m.id === p.method)?.kind === 'account' && !c.partner)

/** Suggested quick-cash buttons for an amount due. */
export const quickCash = (amt: number): number[] => { const a = Math.ceil(amt); return [...new Set([amt, a, Math.ceil(a / 5) * 5, Math.ceil(a / 10) * 10, Math.ceil(a / 50) * 50].filter((x) => x >= amt))].slice(0, 4) }

// ---- barcode / lookup ----
export const findByCode = (ps: Product[], code: string) => ps.find((p) => p.barcode === code) ?? ps.find((p) => p.code === code)
/** Scale-label barcodes (EAN-13 starting 21): digits 3-7 product code, 8-12 weight in grams. */
export function parseScale(code: string): { code: string; kg: number } | null { return /^21\d{11}$/.test(code) ? { code: code.slice(2, 7), kg: Number(code.slice(7, 12)) / 1000 } : null }

// ---- offline queue (orders are idempotent by uuid on the server) ----
export type Queued = { uuid: string; kwargs: Record<string, unknown> }
export const enqueue = (q: Queued[], o: Queued) => (q.some((x) => x.uuid === o.uuid) ? q : [...q, o])
export const dequeue = (q: Queued[], id: string) => q.filter((x) => x.uuid !== id)

// ---- restaurant helpers ----
/** What the kitchen has not seen yet: new/increased quantities, and removed lines. */
export function kitchenDelta(c: Cart): { added: { name: string; qty: number }[]; removed: { key: string; qty: number }[] } {
  const sent = c.sent ?? {}
  const added = c.lines.map((l) => ({ name: l.product.name, qty: l.qty - (sent[l.key] ?? 0) })).filter((x) => x.qty > 0)
  const removed = Object.entries(sent).filter(([k, q]) => q > 0 && !c.lines.some((l) => l.key === k)).map(([key, qty]) => ({ key, qty }))
  return { added, removed }
}
export const hasUnsent = (c: Cart) => { const d = kitchenDelta(c); return d.added.length > 0 || d.removed.length > 0 || c.lines.some((l) => l.qty < (c.sent?.[l.key] ?? 0)) }
/** Rebuild a cart from a server draft (lines are matched to the terminal's product list). */
export function cartFromDraft(d: { uuid: string; table_id?: unknown; customer_count?: number; takeaway?: boolean; partner_id?: unknown; last_order_preparation_change?: string; lines: { uuid?: string; product_id: unknown; qty: number; price_unit: number; discount: number; customer_note?: string }[] }, products: Product[], table: { id: number; name: string } | null): Cart {
  const byId = new Map(products.map((p) => [p.id, p]))
  let sent: Record<string, number> = {}
  try { const j = JSON.parse(d.last_order_preparation_change || '{}') as Record<string, { qty: number }>; sent = Object.fromEntries(Object.entries(j).map(([k, v]) => [k, v.qty])) } catch { /* no history */ }
  const tb = table ?? (Array.isArray(d.table_id) ? { id: d.table_id[0] as number, name: String(d.table_id[1]).replace(/^.*?(\d+)$/, '$1') } : null)
  return { uuid: d.uuid, table: tb, guests: d.customer_count ?? 1, takeaway: !!d.takeaway, sent,
    partner: Array.isArray(d.partner_id) ? { id: d.partner_id[0] as number, name: d.partner_id[1] as string } : null,
    lines: d.lines.flatMap((l) => { const id = Array.isArray(l.product_id) ? (l.product_id[0] as number) : (l.product_id as number); const p = byId.get(id); return p ? [{ key: l.uuid || uuid(), product: p, qty: l.qty, price: l.price_unit, discount: l.discount, note: l.customer_note || undefined }] : [] }) }
}
/** Tip as a percentage of the untaxed-or-total amount, rounded to cents. */
export const tipAmount = (total: number, pct: number) => r2((total * pct) / 100)

// ---- tracking ----
/** Tracked goods need serial/lot numbers before payment: serial = one per unit, lot = at least one. */
export const lotsMissing = (l: Line): boolean => (l.product.tracking === 'serial' ? (l.lots?.length ?? 0) !== Math.round(Math.abs(l.qty)) && l.qty > 0 : l.product.tracking === 'lot' ? (l.lots?.length ?? 0) < 1 && l.qty > 0 : false)
export const comboComplete = (parent: Line, c: Cart): boolean => { const need = parent.product.combo?.length ?? 0; return c.lines.filter((l) => l.combo?.parent === parent.key).length === need }
