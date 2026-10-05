// ESC/POS receipt & kitchen-ticket encoding (Epson-compatible thermal printers), plus scale-reading parsers. Pure functions.
const ESC = 0x1b, GS = 0x1d, LF = 0x0a

/** Latin-1-ish encode with a transliteration pass so accents survive on printers without UTF-8 code pages. */
export function encodeText(s: string): number[] {
  const map: Record<string, string> = { '€': 'EUR', '’': "'", '‘': "'", '“': '"', '”': '"', '–': '-', '—': '-', '…': '...', '•': '*' }
  return [...s].flatMap((ch) => { const r = map[ch] ?? ch; return [...r].map((c) => { const n = c.codePointAt(0)!; return n < 256 ? n : 0x3f }) })
}
export type Align = 'left' | 'center' | 'right'
export class Ticket {
  private b: number[] = [ESC, 0x40]                                   // initialise
  constructor(readonly cols = 42) {}
  align(a: Align) { this.b.push(ESC, 0x61, a === 'left' ? 0 : a === 'center' ? 1 : 2); return this }
  bold(on: boolean) { this.b.push(ESC, 0x45, on ? 1 : 0); return this }
  big(on: boolean) { this.b.push(GS, 0x21, on ? 0x11 : 0x00); return this }   // double width + height
  text(s: string) { this.b.push(...encodeText(s)); return this }
  line(s = '') { return this.text(s).feed(1) }
  feed(n = 1) { for (let i = 0; i < n; i++) this.b.push(LF); return this }
  rule(ch = '-') { return this.line(ch.repeat(this.cols)) }
  /** "left ....... right" on one line; truncates the left side so the amount is never lost. */
  pair(l: string, r: string) { const room = Math.max(1, this.cols - r.length - 1); const left = l.length > room ? l.slice(0, room) : l; return this.line(left + ' '.repeat(this.cols - left.length - r.length) + r) }
  qr(data: string, size = 6) {
    const d = encodeText(data); const n = d.length + 3
    this.b.push(GS, 0x28, 0x6b, 4, 0, 0x31, 0x41, 0x32, 0)                           // model 2
    this.b.push(GS, 0x28, 0x6b, 3, 0, 0x31, 0x43, size)                                // module size
    this.b.push(GS, 0x28, 0x6b, 3, 0, 0x31, 0x45, 0x31)                                // error correction M
    this.b.push(GS, 0x28, 0x6b, n & 0xff, n >> 8, 0x31, 0x50, 0x30, ...d)              // store
    this.b.push(GS, 0x28, 0x6b, 3, 0, 0x31, 0x51, 0x30); return this                   // print
  }
  cut() { this.b.push(...Array(4).fill(LF), GS, 0x56, 0x42, 0); return this }
  drawer() { this.b.push(ESC, 0x70, 0, 25, 250); return this }                         // pulse pin 2
  bytes() { return new Uint8Array(this.b) }
  hex() { return [...this.bytes()].map((x) => x.toString(16).padStart(2, '0')).join('') }
}

export type ReceiptData = { name: string; date_order: string; cashier?: string; header?: string; footer?: string; lines: { name: string; qty: number; price: number; discount?: number; total: number }[]; tax: number; total: number; payments: { name?: string; amount: number }[]; change: number; qr?: string; money: (n: number) => string }
export function receipt(r: ReceiptData, cols = 42): Ticket {
  const t = new Ticket(cols).align('center')
  if (r.header) r.header.split('\n').forEach((h) => t.line(h))
  t.bold(true).big(true).line(r.name).big(false).bold(false).line(`${r.date_order}${r.cashier ? ` · ${r.cashier}` : ''}`).align('left').rule()
  for (const l of r.lines) { t.pair(`${l.qty} x ${l.name}`, r.money(l.total)); if (l.discount) t.line(`   -${l.discount}%`) }
  t.rule().pair('Taxes', r.money(r.tax)).bold(true).pair('TOTAL', r.money(r.total)).bold(false)
  for (const p of r.payments) t.pair(p.name ?? 'Payment', r.money(p.amount))
  if (r.change > 0) t.pair('Change', r.money(r.change))
  if (r.qr) t.feed(1).align('center').qr(r.qr).align('left')
  if (r.footer) { t.feed(1).align('center'); r.footer.split('\n').forEach((f) => t.line(f)) }
  return t.cut()
}
/** Kitchen/bar ticket: big table label, quantities, removals marked. */
export function kitchenTicket(label: string, lines: { name: string; qty: number; note?: string; cancelled?: boolean }[], when: string, cols = 42): Ticket {
  const t = new Ticket(cols).align('center').bold(true).big(true).line(label).big(false).bold(false).line(when).align('left').rule()
  for (const l of lines) { t.bold(true).line(`${l.cancelled ? 'CANCEL ' : ''}${l.qty} x ${l.name}`).bold(false); if (l.note) t.line(`   ${l.note}`) }
  return t.cut()
}

// ---- scales: parse one reading line from the serial stream; returns kilograms ----
/** Handles the common "ST,GS,+  0.250kg", "  0.250 kg", "W:0.250" and Toledo-style "\x020 0.250 0" frames. */
export function parseScale(line: string): { kg: number; stable: boolean } | null {
  const clean = line.replace(/[\x00-\x08\x0b-\x1f]/g, ' ').trim(); if (!clean) return null
  const m = clean.match(/(-?\d+(?:[.,]\d+)?)\s*(kg|g|lb)?/i); if (!m) return null
  let v = parseFloat(m[1].replace(',', '.')); const u = (m[2] ?? 'kg').toLowerCase()
  if (u === 'g') v /= 1000; if (u === 'lb') v *= 0.45359237
  const stable = !/\b(US|UN|MOTION)\b/i.test(clean)                       // "US" = unstable on many scales
  return Number.isFinite(v) && v >= 0 ? { kg: Math.round(v * 1000) / 1000, stable } : null
}
