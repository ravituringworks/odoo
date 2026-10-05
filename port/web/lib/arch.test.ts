import { combine, dateFilters, DATE_PERIODS, periodDomain, pyDomain, searchFilters, type Node } from './arch'
const eq = (a: unknown, b: unknown, m: string) => { if (JSON.stringify(a) !== JSON.stringify(b)) { console.error('FAIL', m, JSON.stringify(a), JSON.stringify(b)); process.exit(1) } }
eq(pyDomain("[('state', '=', 'draft'), ('active', '=', True)]"), [['state', '=', 'draft'], ['active', '=', true]], 'literal')
eq(pyDomain("[('date','>=', context_today())]", { today: '2026-10-05' }), [['date', '>=', '2026-10-05']], 'context_today evaluated')
eq(pyDomain("[('x','=', some_function())]"), null, 'arbitrary python -> hidden')
const n = (tag: string, attrs: Record<string, string> = {}, children: Node[] = []): Node => ({ tag, attrs, text: null, children })
const s = n('search', {}, [n('filter', { name: 'a', string: 'A', domain: "[('x','=',1)]" }), n('filter', { name: 'b', string: 'B', domain: "[('y','=',2)]" }), n('separator'), n('filter', { name: 'c', string: 'C', domain: "[('z','=',3)]" }), n('filter', { name: 'g', context: "{'group_by':'x'}", domain: "[]" })])
const f = searchFilters(s); eq(f.map((x) => x.name), ['a', 'b', 'c'], 'filters')
eq(combine(f), ['&', '|', ['x', '=', 1], ['y', '=', 2], ['z', '=', 3]], 'a|b & c')
eq(combine([f[0]]), [['x', '=', 1]], 'single'); eq(combine([]), [], 'empty')

// uid / context_today evaluation (My Expenses, Late Activities, ...)
eq(pyDomain("[('employee_id.user_id', '=', uid)]", { uid: 7 }), [['employee_id.user_id', '=', 7]], 'uid')
eq(pyDomain("[('employee_id.user_id', '=', uid)]"), null, 'uid unknown -> hidden')
eq(pyDomain("[('my_activity_date_deadline', '<', context_today().strftime('%Y-%m-%d'))]", { today: '2026-10-05' }), [['my_activity_date_deadline', '<', '2026-10-05']], 'context_today')
eq(pyDomain("[('d', '>=', (context_today() - datetime.timedelta(days=7)).strftime('%Y-%m-%d'))]", { today: '2026-10-05' }), [['d', '>=', '2026-09-28']], 'timedelta')
eq(pyDomain("[('d', '>', (datetime.datetime.now() + relativedelta(days=-365)).to_utc())]"), null, 'needs python -> hidden')
eq(pyDomain("[('state', 'in', ('draft', 'reported'))]"), [['state', 'in', ['draft', 'reported']]], 'tuples')
// date filters
const sd = n('search', {}, [n('filter', { name: 'filter_date', string: 'Date', date: 'date_start' })]); eq(dateFilters(sd), [{ name: 'filter_date', label: 'Date', field: 'date_start' }], 'date filter parsed')
eq(periodDomain('d', 'this_month', '2026-10-05'), [['d', '>=', '2026-10-01'], ['d', '<', '2026-11-01']], 'this month'); eq(periodDomain('d', 'last_month', '2026-01-15'), [['d', '>=', '2025-12-01'], ['d', '<', '2026-01-01']], 'last month across year')
eq(periodDomain('d', 'this_quarter', '2026-11-20'), [['d', '>=', '2026-10-01'], ['d', '<', '2027-01-01']], 'quarter'); eq(periodDomain('d', 'this_year', '2026-05-05'), [['d', '>=', '2026-01-01'], ['d', '<', '2027-01-01']], 'year'); eq(DATE_PERIODS.length, 5, 'periods')
console.log('arch: ok')
