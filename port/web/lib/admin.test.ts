import { filterDbs, fmtSize, toggleApp, type DbRow } from './admin'
const eq = (a: unknown, b: unknown, m: string) => { if (JSON.stringify(a) !== JSON.stringify(b)) { console.error('FAIL', m, a, b); process.exit(1) } }
const r = (db: string, kind: DbRow['kind'], status: DbRow['status'], owner = ''): DbRow => ({ db, kind, status, owner, company: '', apps: [], created_at: '', days_left: null, size: null, backups: 0, last_backup: null })
const rows = [r('main', 'main', 'active'), r('trial-a', 'trial', 'active', 'ada@x.com'), r('trial-b', 'trial', 'archived', 'bob@x.com'), r('acme', 'standard', 'active')]
eq(filterDbs(rows, 'all', '').length, 4, 'all'); eq(filterDbs(rows, 'trial', '').map((x) => x.db), ['trial-a', 'trial-b'], 'trial')
eq(filterDbs(rows, 'archived', '').map((x) => x.db), ['trial-b'], 'archived'); eq(filterDbs(rows, 'standard', '').map((x) => x.db), ['acme'], 'standard')
eq(filterDbs(rows, 'all', 'ADA').map((x) => x.db), ['trial-a'], 'search by owner')
eq(fmtSize(null), '—', 'null'); eq(fmtSize(512), '512 B', 'bytes'); eq(fmtSize(1536), '1.5 KB', 'kb'); eq(fmtSize(5 * 1048576), '5.0 MB', 'mb')
eq(toggleApp(['Sales'], 'CRM'), ['CRM', 'Sales'], 'add sorted'); eq(toggleApp(['CRM', 'Sales'], 'CRM'), ['Sales'], 'remove')
console.log('admin ok')
