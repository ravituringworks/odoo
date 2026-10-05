import { daysLeftKind, formatDate } from './portal'
const eq = (a: unknown, b: unknown, m: string) => { if (JSON.stringify(a) !== JSON.stringify(b)) { console.error('FAIL', m, JSON.stringify(a), JSON.stringify(b)); process.exit(1) } }
eq([daysLeftKind(15), daysLeftKind(4), daysLeftKind(3), daysLeftKind(1), daysLeftKind(0)], ['ok', 'ok', 'warn', 'warn', 'bad'], 'urgency levels')
eq(formatDate('2026-10-05 08:30:00', 'en').includes('2026'), true, 'formats a server timestamp'); eq(formatDate('garbage', 'en'), 'garbage', 'bad input is shown as-is, not thrown')
console.log('portal: ok')
