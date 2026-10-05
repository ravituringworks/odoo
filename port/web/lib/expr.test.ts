import { evalExpr } from './expr'
const cases: [string, Record<string, unknown>, boolean][] = [
  ["state != 'draft'", { state: 'sale' }, true], ["state == 'draft'", { state: 'sale' }, false],
  ["state in ('draft','sent')", { state: 'sent' }, true], ["state not in ['sale','done']", { state: 'draft' }, true],
  ["locked or state == 'cancel'", { locked: false, state: 'cancel' }, true], ["not locked and partner_id", { locked: false, partner_id: [3, 'X'] }, true],
  ["amount_total > 100", { amount_total: 250.5 }, true], ["1", {}, true], ["0", {}, false], ["unknown_field", {}, false], ["state ==", {}, false],
]
let bad = 0
for (const [e, r, want] of cases) { const got = evalExpr(e, r); if (got !== want) { bad++; console.error('FAIL', e, got, want) } }
if (bad) process.exit(1); console.log(`expr: ${cases.length} cases ok`)
