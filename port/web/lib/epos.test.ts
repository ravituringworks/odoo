import assert from 'node:assert/strict'
import { EposDoc, eposKitchen, eposReceipt } from './epos'

const d = new EposDoc(20).pair('2 x Coffee', '$6.00').drawer().cut().xml()
assert.ok(d.startsWith('<epos-print xmlns="http://www.epson-pos.com/schemas/2011/03/epos-print">') && d.endsWith('</epos-print>'))
assert.ok(d.includes('<text>2 x Coffee     $6.00&#10;</text>') && d.includes('<pulse drawer="drawer_1" time="pulse_100"/>') && d.includes('<cut type="feed"/>'))
assert.ok(new EposDoc().line('A & B <x> "q"').xml().includes('A &amp; B &lt;x&gt; &quot;q&quot;'))        // markup in product names can never break the document
const r = eposReceipt({ name: 'Order 7', date_order: '2026-10-05 12:00', cashier: 'Ann', header: 'My Shop', footer: 'Thanks!', lines: [{ name: 'Coffee', qty: 2, price: 3, total: 6 }, { name: 'Cake', qty: 1, price: 4, discount: 10, total: 3.6 }], tax: 0.9, total: 9.6, payments: [{ name: 'Cash', amount: 10 }], change: 0.4, qr: 'https://x/y', money: (n) => `$${n.toFixed(2)}` }).xml()
for (const x of ['My Shop', 'Order 7', '2 x Coffee', '$9.60', 'Change', '$0.40', 'Thanks!', '-10%', 'qrcode_model_2']) assert.ok(r.includes(x), x)
const k = eposKitchen('Table 4', [{ name: 'Burger', qty: 2, note: 'no onion' }, { name: 'Cola', qty: 1, cancelled: true }], '12:30').xml()
assert.ok(k.includes('Table 4') && k.includes('2 x Burger') && k.includes('CANCEL 1 x Cola') && k.includes('no onion'))
console.log('epos ok')
