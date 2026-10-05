// Epson ePOS-Print XML (for TM printers with the ePOS service on port 80). Pure builders.
import type { ReceiptData } from './escpos'

const esc = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;').replace(/\n/g, '&#10;')
export type Align = 'left' | 'center' | 'right'
export class EposDoc {
  private parts: string[] = []
  constructor(readonly cols = 42) {}
  align(a: Align) { this.parts.push(`<text align="${a}"/>`); return this }
  bold(on: boolean) { this.parts.push(`<text smooth="true" em="${on}"/>`.replace('smooth="true" ', '')); return this }
  size(w: number, h: number) { this.parts.push(`<text width="${w}" height="${h}"/>`); return this }
  text(s: string) { this.parts.push(`<text>${esc(s)}</text>`); return this }
  line(s = '') { this.parts.push(`<text>${esc(s)}&#10;</text>`); return this }
  pair(l: string, r: string) { const room = Math.max(1, this.cols - r.length - 1); const left = l.length > room ? l.slice(0, room) : l; return this.line(left + ' '.repeat(this.cols - left.length - r.length) + r) }
  rule() { return this.line('-'.repeat(this.cols)) }
  feed(n = 1) { this.parts.push(`<feed line="${n}"/>`); return this }
  qr(data: string) { this.parts.push(`<symbol type="qrcode_model_2" level="level_m" width="6" height="0" size="0">${esc(data)}</symbol>`); return this }
  cut() { this.parts.push('<cut type="feed"/>'); return this }
  drawer() { this.parts.push('<pulse drawer="drawer_1" time="pulse_100"/>'); return this }
  xml() { return `<epos-print xmlns="http://www.epson-pos.com/schemas/2011/03/epos-print">${this.parts.join('')}</epos-print>` }
}

export function eposReceipt(r: ReceiptData, cols = 42): EposDoc {
  const d = new EposDoc(cols).align('center')
  if (r.header) r.header.split('\n').forEach((h) => d.line(h))
  d.bold(true).size(2, 2).line(r.name).size(1, 1).bold(false).line(`${r.date_order}${r.cashier ? ` · ${r.cashier}` : ''}`).align('left').rule()
  for (const l of r.lines) { d.pair(`${l.qty} x ${l.name}`, r.money(l.total)); if (l.discount) d.line(`   -${l.discount}%`) }
  d.rule().pair('Taxes', r.money(r.tax)).bold(true).pair('TOTAL', r.money(r.total)).bold(false)
  for (const p of r.payments) d.pair(p.name ?? 'Payment', r.money(p.amount))
  if (r.change > 0) d.pair('Change', r.money(r.change))
  if (r.qr) d.feed(1).align('center').qr(r.qr).align('left')
  if (r.footer) { d.feed(1).align('center'); r.footer.split('\n').forEach((f) => d.line(f)) }
  return d.cut()
}
export function eposKitchen(label: string, lines: { name: string; qty: number; note?: string; cancelled?: boolean }[], when: string, cols = 42): EposDoc {
  const d = new EposDoc(cols).align('center').bold(true).size(2, 2).line(label).size(1, 1).bold(false).line(when).align('left').rule()
  for (const l of lines) { d.bold(true).line(`${l.cancelled ? 'CANCEL ' : ''}${l.qty} x ${l.name}`).bold(false); if (l.note) d.line(`   ${l.note}`) }
  return d.cut()
}
