import assert from 'node:assert/strict'
import { Ticket, encodeText, kitchenTicket, parseScale, receipt } from './escpos'

assert.deepEqual(encodeText('Café €5'), [67, 97, 102, 233, 32, 69, 85, 82, 53]); assert.deepEqual(encodeText('日'), [0x3f])
const t = new Ticket(20).pair('2 x Coffee', '$6.00'); const s = Buffer.from(t.bytes().slice(2)).toString('latin1')
assert.equal(s.replace(/\n/g, ''), '2 x Coffee     $6.00'); assert.equal(s.length, 21)
assert.equal(new Ticket(10).pair('A very long name', '$1.00').bytes().length - 3, 10)       // truncated left, amount kept, +init(2)+LF
assert.deepEqual([...new Ticket().bytes().slice(0, 2)], [0x1b, 0x40])
assert.deepEqual([...new Ticket().drawer().bytes().slice(2)], [0x1b, 0x70, 0, 25, 250])
assert.deepEqual([...new Ticket().cut().bytes().slice(-4)], [0x1d, 0x56, 0x42, 0])
const r = receipt({ name: 'Order 00007', date_order: '2026-10-05 12:00', cashier: 'Ann', header: 'My Shop', footer: 'Thanks!', lines: [{ name: 'Coffee', qty: 2, price: 3, total: 6 }, { name: 'Cake', qty: 1, price: 4, discount: 10, total: 3.6 }], tax: 0.9, total: 9.6, payments: [{ name: 'Cash', amount: 10 }], change: 0.4, money: (n) => `$${n.toFixed(2)}` })
const txt = Buffer.from(r.bytes()).toString('latin1'); for (const x of ['My Shop', 'Order 00007', '2 x Coffee', '$9.60', 'Change', '$0.40', 'Thanks!', '-10%']) assert.ok(txt.includes(x), x)
assert.ok(r.hex().startsWith('1b40')); assert.ok(r.hex().endsWith('1d564200'))
const k = Buffer.from(kitchenTicket('Table 4', [{ name: 'Burger', qty: 2, note: 'no onion' }, { name: 'Cola', qty: 1, cancelled: true }], '12:30').bytes()).toString('latin1')
assert.ok(k.includes('Table 4') && k.includes('2 x Burger') && k.includes('CANCEL 1 x Cola') && k.includes('no onion'))
assert.ok(Buffer.from(new Ticket().qr('https://x.y/o').bytes()).toString('latin1').includes('https://x.y/o'))
// scales
assert.deepEqual(parseScale('ST,GS,+  0.250kg'), { kg: 0.25, stable: true }); assert.deepEqual(parseScale('US,GS,+  0.255kg'), { kg: 0.255, stable: false })
assert.deepEqual(parseScale('  1,500 kg'), { kg: 1.5, stable: true }); assert.deepEqual(parseScale('250 g'), { kg: 0.25, stable: true }); assert.equal(parseScale('2.2 lb')?.kg, 0.998)
assert.equal(parseScale(''), null); assert.equal(parseScale('no digits'), null)
console.log('escpos ok')
