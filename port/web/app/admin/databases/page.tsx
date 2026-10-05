'use client'
import { Fragment, useCallback, useEffect, useMemo, useState } from 'react'
import { rpc } from '@/lib/rpc'
import { type Account, type Backup, type DbRow, type Filter, archiveDb, backupDb, createDb, deleteBackup, deleteDb, filterDbs, fmtSize, listAccounts, listBackups, listDbs, openDb, restoreDb, setDisallowed, toggleApp } from '@/lib/admin'
import { Icon } from '@/components/Icon'

type Pending = { action: 'delete' | 'restore'; db: string; file?: string }
const FILTERS: Filter[] = ['all', 'main', 'trial', 'standard', 'archived']

export default function DatabasesPage() {
  const [rows, setRows] = useState<DbRow[]>([]); const [accounts, setAccounts] = useState<Account[]>([]); const [catalog, setCatalog] = useState<string[]>([])
  const [filter, setFilter] = useState<Filter>('all'); const [q, setQ] = useState(''); const [msg, setMsg] = useState<{ ok: boolean; text: string }>()
  const [busy, setBusy] = useState<string>(); const [backups, setBackups] = useState<Record<string, Backup[]>>({}); const [pending, setPending] = useState<Pending>(); const [typed, setTyped] = useState(''); const [keep, setKeep] = useState(false)
  const [nf, setNf] = useState({ name: '', email: '', company: '', apps: [] as string[] })
  const load = useCallback(async () => {
    try { const [d, a] = await Promise.all([listDbs(), listAccounts()]); setRows(d); setAccounts(a) } catch (e) { setMsg({ ok: false, text: (e as Error).message }) }
  }, [])
  useEffect(() => { load(); rpc<{ name: string; available: boolean }[]>({ method: 'trial_catalog', db: null }).then((c) => setCatalog(c.filter((x) => x.available).map((x) => x.name))).catch(() => undefined) }, [load])
  const shown = useMemo(() => filterDbs(rows, filter, q), [rows, filter, q])

  const act = async (key: string, f: () => Promise<unknown>, ok: string) => {
    setBusy(key); setMsg(undefined)
    try { await f(); setMsg({ ok: true, text: ok }); setPending(undefined); setTyped(''); await load() } catch (e) { setMsg({ ok: false, text: (e as Error).message }) } finally { setBusy(undefined) }
  }
  const showBackups = async (db: string) => { if (backups[db]) { setBackups(({ [db]: _, ...rest }) => rest); return } setBackups({ ...backups, [db]: await listBackups(db) }) }
  const refreshBackups = async (db: string) => { if (backups[db]) setBackups({ ...backups, [db]: await listBackups(db) }) }

  return (
    <>
      <div className="bar"><h1>Databases</h1><span className="grow" /><button className="btn" onClick={load}><Icon name="refresh" size="var(--icon-md)" /> Refresh</button></div>
      <div className="set-body" style={{ maxWidth: 1180 }}>
        {msg && <div className={msg.ok ? 'hint' : 'err'} role="status">{msg.text}</div>}
        <div style={{ display: 'flex', gap: 8, alignItems: 'center', margin: '10px 0', flexWrap: 'wrap' }}>
          {FILTERS.map((f) => <button key={f} className={`btn ${filter === f ? 'p' : ''}`} onClick={() => setFilter(f)}>{f[0].toUpperCase() + f.slice(1)} ({rows.filter((r) => filterDbs([r], f, '').length).length})</button>)}
          <input type="search" placeholder="Search database, owner, company…" value={q} onChange={(e) => setQ(e.target.value)} style={{ marginInlineStart: 'auto', minWidth: 260 }} />
        </div>
        <table>
          <thead><tr><th>Database</th><th>Type</th><th>Owner</th><th>Apps</th><th>Size</th><th>Expires</th><th>Last backup</th><th /></tr></thead>
          <tbody>
            {shown.map((r) => (
              <Fragment key={r.db}>
                <tr style={r.status === 'archived' ? { opacity: 0.6 } : undefined}>
                  <td><b>{r.db}</b>{r.status === 'archived' && <span className="tr-ent">archived</span>}<div className="muted">{r.company}</div></td>
                  <td>{r.kind === 'main' ? 'Main' : r.kind === 'trial' ? 'Trial' : 'Standard'}</td><td>{r.owner || '—'}</td><td>{r.apps.join(', ') || '—'}</td><td>{fmtSize(r.size)}</td>
                  <td>{r.days_left == null ? '—' : `${r.days_left} d`}</td><td>{r.last_backup ?? 'never'}</td>
                  <td style={{ whiteSpace: 'nowrap' }}>
                    <button className="btn" disabled={!!busy} onClick={() => act(`b${r.db}`, async () => { await backupDb(r.db); await refreshBackups(r.db) }, `Backup of ${r.db} created.`)}>Backup</button>{' '}
                    <button className="btn" onClick={() => showBackups(r.db)}>Backups ({r.backups})</button>{' '}
                    {r.kind !== 'main' && <>
                      {r.status === 'active' && <button className="btn" onClick={() => act(`o${r.db}`, () => openDb(r.db), '')}>Open</button>}{' '}
                      <button className="btn" disabled={!!busy} onClick={() => act(`a${r.db}`, () => archiveDb(r.db, r.status === 'active'), r.status === 'active' ? `${r.db} archived.` : `${r.db} restored from archive.`)}>{r.status === 'active' ? 'Archive' : 'Unarchive'}</button>{' '}
                      <button className="btn" onClick={() => { setPending({ action: 'delete', db: r.db }); setTyped('') }}>Delete</button>
                    </>}
                  </td>
                </tr>
                {pending?.db === r.db && (
                  <tr key={`${r.db}-c`}><td colSpan={8}>
                    <div className="card" style={{ padding: 12 }}>
                      <b>{pending.action === 'delete' ? `Delete ${r.db} permanently?` : `Restore ${r.db} from ${pending.file}?`}</b>{' '}
                      <span className="muted">{pending.action === 'delete' ? 'All its data is removed.' : 'Current data is replaced (a safety backup is made first) and everyone is signed out.'} Type the database name to confirm.</span>
                      <div style={{ display: 'flex', gap: 8, marginTop: 8, alignItems: 'center' }}>
                        <input value={typed} onChange={(e) => setTyped(e.target.value)} placeholder={r.db} aria-label="Confirm database name" />
                        {pending.action === 'delete' && <label><input type="checkbox" checked={keep} onChange={(e) => setKeep(e.target.checked)} /> keep its backups</label>}
                        <button className="btn p" disabled={typed !== r.db || !!busy} onClick={() => act(`x${r.db}`, () => pending.action === 'delete' ? deleteDb(r.db, keep) : restoreDb(r.db, pending.file!), pending.action === 'delete' ? `${r.db} deleted.` : `${r.db} restored.`)}>{pending.action === 'delete' ? 'Delete' : 'Restore'}</button>
                        <button className="btn" onClick={() => setPending(undefined)}>Cancel</button>
                      </div>
                    </div>
                  </td></tr>
                )}
                {backups[r.db] && (
                  <tr key={`${r.db}-b`}><td colSpan={8}>
                    {backups[r.db].length === 0 ? <span className="muted">No backups yet.</span> : (
                      <table><tbody>{backups[r.db].map((b) => (
                        <tr key={b.file}><td>{b.created_at}</td><td>{fmtSize(b.size)}</td><td className="muted">{b.file}</td>
                          <td style={{ textAlign: 'end' }}>
                            {r.kind !== 'main' && <button className="btn" onClick={() => { setPending({ action: 'restore', db: r.db, file: b.file }); setTyped('') }}>Restore</button>}{' '}
                            <button className="btn" onClick={() => act(`d${b.file}`, async () => { await deleteBackup(r.db, b.file); await refreshBackups(r.db) }, 'Backup deleted.')}>Delete</button>
                          </td></tr>))}</tbody></table>)}
                  </td></tr>
                )}
              </Fragment>
            ))}
            {shown.length === 0 && <tr><td colSpan={8} className="muted">No databases match.</td></tr>}
          </tbody>
        </table>

        <h2 className="tr-cat" style={{ fontSize: 22 }}>New database</h2>
        <p className="hint">Administrator-created databases are permanent (they never expire) and are not counted against the trial limit. Leave the owner empty for an unowned database; open it from the table above.</p>
        <form className="card" style={{ padding: 14, display: 'grid', gap: 10 }} onSubmit={(e) => { e.preventDefault(); act('create', () => createDb(nf), `Database created.`).then(() => setNf({ name: '', email: '', company: '', apps: [] })) }}>
          <div className="tr-two">
            <label>Name<input value={nf.name} onChange={(e) => setNf({ ...nf, name: e.target.value.toLowerCase() })} placeholder="acme_prod (optional)" /></label>
            <label>Owner account (email)<input value={nf.email} onChange={(e) => setNf({ ...nf, email: e.target.value })} list="acct" placeholder="optional" /><datalist id="acct">{accounts.map((a) => <option key={a.email} value={a.email} />)}</datalist></label>
          </div>
          <label>Company<input value={nf.company} onChange={(e) => setNf({ ...nf, company: e.target.value })} /></label>
          <fieldset style={{ border: 0, padding: 0 }}><legend className="muted">Apps</legend>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>{catalog.map((a) => <label key={a}><input type="checkbox" checked={nf.apps.includes(a)} onChange={() => setNf({ ...nf, apps: toggleApp(nf.apps, a) })} /> {a}</label>)}</div></fieldset>
          <div><button className="btn p" disabled={!nf.apps.length || !!busy}>Create database</button></div>
        </form>

        <h2 className="tr-cat" style={{ fontSize: 22 }}>Accounts and allowed apps</h2>
        <p className="hint">Ticked apps are allowed. Unticking blocks the app for every database of that account — it cannot be provisioned or installed. Apps already installed are not removed.</p>
        <table>
          <thead><tr><th>Account</th><th>Databases</th><th>Allowed apps</th></tr></thead>
          <tbody>{accounts.map((a) => (
            <tr key={a.email}><td><b>{a.name}</b><div className="muted">{a.email}</div></td><td>{a.databases}</td>
              <td><div style={{ display: 'flex', flexWrap: 'wrap', gap: '4px 12px' }}>{catalog.map((app) => (
                <label key={app}><input type="checkbox" checked={!a.disallowed.includes(app)} disabled={!!busy}
                  onChange={() => act(`s${a.email}${app}`, () => setDisallowed(a.email, toggleApp(a.disallowed, app)), `${app} ${a.disallowed.includes(app) ? 'allowed' : 'blocked'} for ${a.email}.`)} /> {app}</label>))}</div></td></tr>))}
            {accounts.length === 0 && <tr><td colSpan={3} className="muted">No customer accounts yet.</td></tr>}</tbody>
        </table>
      </div>
    </>
  )
}
