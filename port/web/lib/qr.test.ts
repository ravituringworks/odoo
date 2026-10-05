import assert from 'node:assert/strict'
import { qrSvg } from './qr'
const a = qrSvg('https://shop.example/order/?config=2&token=abc&table=7')
assert.ok(a.startsWith('<svg') && a.includes('<path') && a.length > 500)
assert.notEqual(a, qrSvg('https://shop.example/order/?config=2&token=abc&table=8'))   // different data → different code
assert.equal(qrSvg('x'), qrSvg('x'))
console.log('qr ok')
