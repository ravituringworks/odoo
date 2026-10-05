import assert from 'node:assert/strict'
import { programPoints, rewardDiscount, rewardLines, rewardTotal, type Item, type Program, type Reward } from './loyalty'
import type { Tax } from './pos'

const rw = (o: Partial<Reward>): Reward => ({ id: 1, reward_type: 'discount', required_points: 0, discount: 10, discount_mode: 'percent', discount_applicability: 'order', discount_max_amount: 0, discount_product_ids: [], all_discount_product_ids: [], reward_product_ids: [], reward_product_qty: 1, ...o })
const item = (product: number, qty: number, price: number, taxIds: number[] = []): Item => ({ product, categs: [], qty, price, untaxed: qty * price, total: qty * price * (taxIds.length ? 1.2 : 1), taxIds })
const prog = (o: Partial<Program>): Program => ({ id: 1, name: 'P', program_type: 'loyalty', applies_on: 'future', rules: [], rewards: [], ...o })
const rule = (o: object) => ({ id: 1, minimum_qty: 0, minimum_amount: 0, reward_point_amount: 1, reward_point_mode: 'unit', product_ids: [], valid_product_ids: [], ...o }) as Program['rules'][number]
const vat: Tax = { id: 1, amount: 20, amount_type: 'percent', price_include: false }

// points: per unit / per money / per order, with thresholds, restricted to products
const items = [item(1, 2, 10), item(2, 1, 40)]
assert.equal(programPoints(prog({ rules: [rule({ reward_point_mode: 'unit' })] }), items, []), 3)
assert.equal(programPoints(prog({ rules: [rule({ reward_point_mode: 'money', reward_point_amount: 0.5 })] }), items, []), 30)
assert.equal(programPoints(prog({ rules: [rule({ reward_point_mode: 'order', reward_point_amount: 7, minimum_amount: 100 })] }), items, []), 0)
assert.equal(programPoints(prog({ rules: [rule({ product_ids: [2] })] }), items, []), 1)
assert.equal(programPoints(prog({ trigger: 'with_code', rules: [rule({ code: 'SAVE' })] }), items, []), 0)
assert.equal(programPoints(prog({ trigger: 'with_code', rules: [rule({ code: 'SAVE' })] }), items, ['save']), 3)

// discounts: percent on order, cheapest unit, per-order cap, per-point (gift card)
assert.deepEqual(rewardDiscount(rw({}), items, 0)!.groups, [[[], -6]])
assert.deepEqual(rewardDiscount(rw({ discount_applicability: 'cheapest', discount: 50 }), items, 0)!.groups, [[[], -5]])
assert.deepEqual(rewardDiscount(rw({ discount_max_amount: 4 }), items, 0)!.groups, [[[], -4]])
assert.deepEqual(rewardDiscount(rw({ discount_mode: 'per_order', discount: 1000 }), items, 0)!.groups, [[[], -60]])
const gc = rewardDiscount(rw({ discount_mode: 'per_point', discount: 1 }), items, 25)!; assert.deepEqual(gc.groups, [[[], -25]]); assert.equal(gc.cost, 25)
assert.equal(rewardDiscount(rw({ discount_applicability: 'specific', discount_product_ids: [9] }), items, 0), null)
// discount keeps the tax split of the lines it discounts
const mixed = [item(1, 1, 100, [1]), item(2, 1, 100)]
assert.deepEqual(rewardDiscount(rw({ discount: 10 }), mixed, 0)!.groups, [[[], -10], [[1], -10]])
const taxes = new Map([[1, vat]])
const ls = rewardLines([{ rewardId: 1 }], [prog({ rewards: [rw({})] })], [], mixed, [], taxes); assert.equal(rewardTotal(ls, taxes), -22)
// not enough points → no line; balance is spent across claims on the same card
assert.equal(rewardLines([{ rewardId: 1, cardId: 5 }], [prog({ rewards: [rw({ required_points: 10 })] })], [{ id: 5, code: 'x', points: 4, program_id: 1 }], items, [], taxes).length, 0)
assert.equal(rewardLines([{ rewardId: 1, cardId: 5 }, { rewardId: 1, cardId: 5 }], [prog({ rewards: [rw({ required_points: 3 })] })], [{ id: 5, code: 'x', points: 5, program_id: 1 }], items, [], taxes).length, 1)
console.log('loyalty ok')
