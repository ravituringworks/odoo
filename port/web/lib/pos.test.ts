import assert from 'node:assert/strict'
import { cartTotals, change, computeTax, dequeue, due, emptyCart, enqueue, findByCode, parseScale, priceFor, quickCash, reduce, canValidate, roundTo, type Method, type Product, type Tax } from './pos'

const vat: Tax = { id: 1, amount: 15, amount_type: 'percent', price_include: false }
const inc: Tax = { id: 2, amount: 20, amount_type: 'percent', price_include: true }
assert.deepEqual(computeTax(2, 100, 10, [vat]), { untaxed: 180, tax: 27, total: 207 })
assert.deepEqual(computeTax(1, 120, 0, [inc]), { untaxed: 100, tax: 20, total: 120 })
assert.deepEqual(computeTax(3, 10, 0, [{ id: 3, amount: 5, amount_type: 'fixed', price_include: false }]), { untaxed: 30, tax: 15, total: 45 })

const desk: Product = { id: 1, name: 'Desk', price: 100, tax_ids: [1], category_ids: [7], barcode: '111', code: 'D1' }
const lamp: Product = { id: 2, name: 'Lamp', price: 40, tax_ids: [], category_ids: [] }
const taxes = new Map([[1, vat]])
let c = emptyCart()
c = reduce(c, { t: 'add', product: desk }); c = reduce(c, { t: 'add', product: desk }); c = reduce(c, { t: 'add', product: lamp })
assert.equal(c.lines.length, 2); assert.equal(c.lines[0].qty, 2)                    // same product merges
assert.deepEqual(cartTotals(c, taxes), { untaxed: 240, tax: 30, total: 270 })
c = reduce(c, { t: 'order_discount', pct: 10 }); assert.equal(cartTotals(c, taxes).total, 243)
c = reduce(c, { t: 'remove', key: c.lines[1].key }); assert.equal(c.lines.length, 1)
assert.equal(reduce(c, { t: 'discount', key: c.lines[0].key, pct: 500 }).lines[0].discount, 100)   // clamped

// pricelists: most specific rule wins, quantity breaks apply
const pl = { id: 1, name: 'B2B', items: [
  { applied_on: '3_global', compute_price: 'percentage', fixed_price: 0, percent_price: 10, min_quantity: 0, product_tmpl_id: false, product_id: false, categ_id: false },
  { applied_on: '2_product_category', compute_price: 'percentage', fixed_price: 0, percent_price: 20, min_quantity: 0, product_tmpl_id: false, product_id: false, categ_id: [7, 'Desks'] },
  { applied_on: '0_product_variant', compute_price: 'fixed', fixed_price: 70, percent_price: 0, min_quantity: 5, product_tmpl_id: false, product_id: [1, 'Desk'], categ_id: false },
] }
assert.equal(priceFor(desk, 1, pl), 80); assert.equal(priceFor(desk, 5, pl), 70); assert.equal(priceFor(lamp, 1, pl), 36); assert.equal(priceFor(lamp, 1, null), 40)
const c2 = reduce(reduce(emptyCart(), { t: 'add', product: desk }), { t: 'pricelist', id: 1, list: pl }); assert.equal(c2.lines[0].price, 80)

// payments & change
const ms: Method[] = [{ id: 1, name: 'Cash', kind: 'cash' }, { id: 2, name: 'Card', kind: 'bank' }, { id: 3, name: 'Account', kind: 'account' }]
assert.equal(due(50, [{ method: 1, amount: 20 }]), 30)
assert.equal(change(50, [{ method: 1, amount: 60 }], ms), 10)
assert.equal(change(50, [{ method: 2, amount: 60 }], ms), 0)                   // no change on cards
const cart = reduce(emptyCart(), { t: 'add', product: lamp })
assert.ok(canValidate(cart, 40, [{ method: 1, amount: 50 }], ms, false)); assert.ok(!canValidate(cart, 40, [{ method: 1, amount: 30 }], ms, false))
assert.ok(!canValidate(cart, 40, [{ method: 2, amount: 50 }], ms, false)); assert.ok(!canValidate(cart, 40, [{ method: 3, amount: 40 }], ms, false))
assert.ok(canValidate({ ...cart, partner: { id: 9, name: 'A' } }, 40, [{ method: 3, amount: 40 }], ms, true)); assert.ok(!canValidate(cart, 40, [{ method: 1, amount: 40 }], ms, true))
assert.deepEqual(quickCash(12.5), [12.5, 13, 15, 20]); assert.equal(roundTo(10.07, 0.05), 10.05)

// scanning & offline queue
assert.equal(findByCode([desk, lamp], '111')?.id, 1); assert.equal(findByCode([desk, lamp], 'D1')?.id, 1); assert.equal(findByCode([desk], 'x'), undefined)
assert.deepEqual(parseScale('2100123012345'), { code: '00123', kg: 1.234 }); assert.equal(parseScale('1234'), null)
let q = enqueue([], { uuid: 'a', kwargs: {} }); q = enqueue(q, { uuid: 'a', kwargs: {} }); q = enqueue(q, { uuid: 'b', kwargs: {} })
assert.equal(q.length, 2); assert.equal(dequeue(q, 'a').length, 1)
// restaurant: kitchen deltas and draft round-trip
import { cartFromDraft, hasUnsent, kitchenDelta, tipAmount } from './pos'
const tc = { ...reduce(reduce(emptyCart(), { t: 'add', product: desk }), { t: 'add', product: lamp }), sent: {} as Record<string, number> }
assert.ok(hasUnsent(tc)); assert.equal(kitchenDelta(tc).added.length, 2)
const sentAll = { ...tc, sent: Object.fromEntries(tc.lines.map((l) => [l.key, l.qty])) }
assert.ok(!hasUnsent(sentAll))
const more = reduce(sentAll, { t: 'qty', key: sentAll.lines[0].key, qty: 3 }); assert.deepEqual(kitchenDelta(more).added, [{ name: 'Desk', qty: 2 }])
const gone = reduce(sentAll, { t: 'remove', key: sentAll.lines[1].key }); assert.equal(kitchenDelta(gone).removed.length, 1); assert.ok(hasUnsent(gone))
const back = cartFromDraft({ uuid: 'u', table_id: [4, 'T4'], customer_count: 2, takeaway: false, partner_id: [9, 'Ann'], last_order_preparation_change: JSON.stringify({ a: { qty: 1 } }), lines: [{ uuid: 'a', product_id: [1, 'Desk'], qty: 1, price_unit: 100, discount: 0 }, { uuid: 'x', product_id: 99, qty: 1, price_unit: 1, discount: 0 }] }, [desk, lamp], { id: 4, name: '4' })
assert.equal(back.lines.length, 1); assert.equal(back.partner?.name, 'Ann'); assert.ok(!hasUnsent(back)); assert.equal(back.guests, 2)
assert.equal(tipAmount(33.33, 15), 5)
// combos & tracking
import { comboComplete, lotsMissing } from './pos'
const menu: Product = { id: 50, name: 'Menu', price: 10, tax_ids: [], category_ids: [], type: 'combo', combo: [{ id: 1, name: 'Side', items: [] }, { id: 2, name: 'Drink', items: [] }] }
const picks = [{ id: 11, product_id: 5, name: 'Salad', extra: 1.5, tax_ids: [] }, { id: 21, product_id: 6, name: 'Cola', extra: 0, tax_ids: [] }]
let mc = reduce(emptyCart(), { t: 'addCombo', product: menu, picks })
assert.equal(mc.lines.length, 3); assert.equal(cartTotals(mc, new Map()).total, 11.5); assert.ok(comboComplete(mc.lines[0], mc))
mc = reduce(mc, { t: 'qty', key: mc.lines[0].key, qty: 2 }); assert.deepEqual(mc.lines.map((l) => l.qty), [2, 2, 2]); assert.equal(cartTotals(mc, new Map()).total, 23)
assert.equal(reduce(mc, { t: 'qty', key: mc.lines[1].key, qty: 9 }).lines[1].qty, 2)                       // children cannot be edited alone
assert.equal(reduce(mc, { t: 'remove', key: mc.lines[1].key }).lines.length, 3)
assert.equal(reduce(mc, { t: 'remove', key: mc.lines[0].key }).lines.length, 0)                           // removing the combo removes its items
const ph: Product = { id: 7, name: 'Phone', price: 100, tax_ids: [], category_ids: [], tracking: 'serial' }
let tl = reduce(emptyCart(), { t: 'add', product: ph }); assert.ok(lotsMissing(tl.lines[0]))
tl = reduce(tl, { t: 'lots', key: tl.lines[0].key, lots: ['S1'] }); assert.ok(!lotsMissing(tl.lines[0]))
tl = reduce(tl, { t: 'qty', key: tl.lines[0].key, qty: 2 }); assert.ok(lotsMissing(tl.lines[0]))
assert.ok(lotsMissing({ key: 'k', product: { ...ph, tracking: 'lot' }, qty: 3, price: 1, discount: 0 })); assert.ok(!lotsMissing({ key: 'k', product: ph, qty: -1, price: 1, discount: 0 }))   // returns need no serials
// foreign-currency pricelist: list prices convert, fixed rules are already in that currency
const eur = { id: 9, name: 'EUR', rate: 0.5, currency: { name: 'EUR', symbol: '€' }, items: [{ applied_on: '0_product_variant', compute_price: 'fixed', fixed_price: 33, percent_price: 0, fixed: 0, min_quantity: 0, product_tmpl_id: false, product_id: [2, 'Lamp'], categ_id: false }] } as unknown as Parameters<typeof priceFor>[2]
assert.equal(priceFor(desk, 1, eur), 50); assert.equal(priceFor(lamp, 1, eur), 33)
console.log('pos ok')
