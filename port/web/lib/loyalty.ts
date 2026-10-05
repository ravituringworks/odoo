// POS loyalty preview: mirrors the server's pos_loyalty rules (the server recomputes and is authoritative).
import { computeTax, r2, type Tax } from './pos'

export type Rule = { id: number; mode?: string; code?: string; minimum_qty: number; minimum_amount: number; minimum_amount_tax_mode?: string; reward_point_amount: number; reward_point_mode: string; product_ids: number[]; valid_product_ids: number[]; any_product?: boolean; product_category_id?: unknown }
export type Reward = { id: number; description?: string; reward_type: 'discount' | 'product'; required_points: number; discount: number; discount_mode: string; discount_applicability: string; discount_max_amount: number; discount_product_ids: number[]; all_discount_product_ids: number[]; discount_product_category_id?: unknown; reward_product_id?: unknown; reward_product_ids: number[]; reward_product_qty: number }
export type Program = { id: number; name: string; program_type: string; trigger?: string; applies_on: string; is_nominative?: boolean; rules: Rule[]; rewards: Reward[] }
export type Card = { id: number; code: string; points: number; program_id: number; partner_id?: number | null }
export type Item = { product: number; categs: number[]; qty: number; untaxed: number; total: number; taxIds: number[]; price: number }
export type Claim = { rewardId: number; cardId?: number | null }
export type RewardLine = { label: string; taxIds: number[]; price: number; cost: number; rewardId: number; cardId?: number | null; addProduct?: { product: number; qty: number } }

const ref = (v: unknown): number | null => (Array.isArray(v) ? (v[0] as number) : typeof v === 'number' ? v : null)
const under = (chain: number[], t: number) => chain.includes(t)

const matches = (r: Rule, i: Item) => {
  const cat = ref(r.product_category_id)
  const restricted = !r.any_product && (r.product_ids.length > 0 || cat !== null)
  return !restricted || r.product_ids.includes(i.product) || r.valid_product_ids.includes(i.product) || (cat !== null && under(i.categs, cat))
}
export function rulePoints(r: Rule, items: Item[]): number {
  const el = items.filter((i) => matches(r, i)); if (!el.length) return 0
  const qty = el.reduce((s, i) => s + i.qty, 0); const amt = el.reduce((s, i) => s + (r.minimum_amount_tax_mode === 'excl' ? i.untaxed : i.total), 0)
  if (qty < r.minimum_qty || amt < r.minimum_amount) return 0
  return r.reward_point_mode === 'money' ? amt * r.reward_point_amount : r.reward_point_mode === 'unit' ? qty * r.reward_point_amount : r.reward_point_amount
}
export function programPoints(p: Program, items: Item[], codes: string[]): number {
  return r2(p.rules.reduce((s, r) => {
    if (r.mode === 'with_code' || p.trigger === 'with_code') { if (!r.code || !codes.some((c) => c.toLowerCase() === r.code!.toLowerCase())) return s }
    return s + rulePoints(r, items)
  }, 0))
}

/** [taxIds, negative untaxed amount] groups + points cost, or null if the reward cannot apply. */
export function rewardDiscount(rw: Reward, items: Item[], balance: number): { groups: [number[], number][]; cost: number } | null {
  const cheapest = rw.discount_applicability === 'cheapest'
  let el = items
  if (rw.discount_applicability === 'specific') { const ps = [...rw.discount_product_ids, ...rw.all_discount_product_ids]; const c = ref(rw.discount_product_category_id); el = items.filter((i) => ps.includes(i.product) || (c !== null && under(i.categs, c))) }
  else if (cheapest) { const pos = items.filter((i) => i.qty > 0).sort((a, b) => a.untaxed / a.qty - b.untaxed / b.qty); el = pos.slice(0, 1) }
  const baseOf = (i: Item) => (cheapest ? i.untaxed / Math.max(i.qty, 1) : i.untaxed)
  const base = el.reduce((s, i) => s + baseOf(i), 0); if (base <= 0) return null
  let amount: number, cost: number
  if (rw.discount_mode === 'per_point') { amount = Math.min(rw.discount * balance, base); cost = rw.discount > 0 ? amount / rw.discount : 0 }
  else if (rw.discount_mode === 'per_order') { amount = Math.min(rw.discount, base); cost = rw.required_points }
  else { amount = (base * rw.discount) / 100; cost = rw.required_points }
  if (rw.discount_max_amount > 0) amount = Math.min(amount, rw.discount_max_amount)
  amount = r2(amount)
  const g = new Map<string, [number[], number]>()
  for (const i of el) { const k = [...i.taxIds].sort((a, b) => a - b); const key = k.join(','); const cur = g.get(key) ?? [k, 0]; cur[1] += baseOf(i); g.set(key, cur) }
  const lex = (a: number[], b: number[]): number => { for (let n = 0; n < Math.min(a.length, b.length); n++) if (a[n] !== b[n]) return a[n] - b[n]; return a.length - b.length }
  return { groups: [...g.values()].sort((x, y) => lex(x[0], y[0])).map(([k, v]) => [k, -r2((amount * v) / base)] as [number[], number]), cost }
}

export const claimable = (rw: Reward, balance: number) => balance + 1e-9 >= rw.required_points

/** Reward lines for the current claims (used for the ticket preview and the total). */
export function rewardLines(claims: Claim[], programs: Program[], cards: Card[], items: Item[], codes: string[], taxes: Map<number, Tax>): RewardLine[] {
  const spent = new Map<number, number>(); const out: RewardLine[] = []
  for (const c of claims) {
    const prog = programs.find((p) => p.rewards.some((r) => r.id === c.rewardId)); const rw = prog?.rewards.find((r) => r.id === c.rewardId); if (!prog || !rw) continue
    const card = c.cardId ? cards.find((k) => k.id === c.cardId) : undefined
    const balance = card ? card.points - (spent.get(card.id) ?? 0) : programPoints(prog, items, codes)
    if (!claimable(rw, balance)) continue
    const label = rw.description || prog.name
    if (rw.reward_type === 'product') {   // a free product: discount it when already in the basket, else the server adds it and nets it to zero
      const pid = ref(rw.reward_product_id) ?? rw.reward_product_ids[0]; const want = Math.max(rw.reward_product_qty || 1, 1)
      const inCart = items.filter((i) => i.product === pid)
      const have = inCart.reduce((s, i) => s + i.qty, 0)
      if (have >= want && inCart[0]) out.push({ label, taxIds: inCart[0].taxIds, price: -r2((inCart[0].untaxed / inCart[0].qty) * want), cost: rw.required_points, rewardId: rw.id, cardId: c.cardId })
      else out.push({ label: `${label} (${want} free)`, taxIds: [], price: 0, cost: rw.required_points, rewardId: rw.id, cardId: c.cardId })
      if (card) spent.set(card.id, (spent.get(card.id) ?? 0) + rw.required_points)
      continue
    }
    const d = rewardDiscount(rw, items, balance); if (!d) continue
    for (const [taxIds, price] of d.groups) if (price !== 0) out.push({ label, taxIds, price, cost: d.cost, rewardId: rw.id, cardId: c.cardId })
    if (card) spent.set(card.id, (spent.get(card.id) ?? 0) + d.cost)
  }
  return out
}
export const rewardTotal = (ls: RewardLine[], taxes: Map<number, Tax>) => r2(ls.reduce((s, l) => s + computeTax(1, l.price, 0, l.taxIds.map((i) => taxes.get(i)).filter((t): t is Tax => !!t)).total, 0))
