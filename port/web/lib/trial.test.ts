import { byCategory, toggle, validate, type Form, type TrialApp } from './trial'
const eq = (a: unknown, b: unknown, m: string) => { if (JSON.stringify(a) !== JSON.stringify(b)) { console.error('FAIL', m, JSON.stringify(a), JSON.stringify(b)); process.exit(1) } }
const ok: Form = { name: 'Ada', email: 'ada@example.com', phone: '+44 20 7946 0000', company: 'AE', country: 'GB', password: 'long-enough', terms: true }
eq(validate(ok, ['CRM']), {}, 'valid form')
eq(Object.keys(validate({ ...ok, email: 'nope' }, ['CRM'])), ['email'], 'email'); eq(Object.keys(validate({ ...ok, password: 'short' }, ['CRM'])), ['password'], 'password')
eq(Object.keys(validate({ ...ok, terms: false }, [])).sort(), ['apps', 'terms'], 'terms + no apps'); eq(Object.keys(validate({ ...ok, phone: '<x>' }, ['CRM'])), ['phone'], 'phone')
eq(toggle(['A'], 'B'), ['A', 'B'], 'add'); eq(toggle(['A', 'B'], 'A'), ['B'], 'remove'); eq(toggle(Array.from({ length: 15 }, (_, i) => `a${i}`), 'z').length, 15, 'cap at 15')
const cat: TrialApp[] = [{ name: 'Website', category: 'Website', icon: '', available: true }, { name: 'CRM', category: 'Sales', icon: '', available: true }, { name: 'eCommerce', category: 'Website', icon: '', available: true }]
eq(byCategory(cat).map(([c, a]) => [c, a.map((x) => x.name)]), [['Website', ['Website', 'eCommerce']], ['Sales', ['CRM']]], 'grouping keeps order')
console.log('trial: ok')
