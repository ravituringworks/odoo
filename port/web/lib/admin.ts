// Operator console client: every database (main, trials, administrator-created), their lifecycle and per-account app rules.
import { getDb, getToken, rpc, setDb, setToken } from './rpc'

export type DbRow = { db: string; kind: 'main' | 'trial' | 'standard'; status: 'active' | 'archived'; owner: string; company: string; apps: string[]; created_at: string; days_left: number | null; size: number | null; backups: number; last_backup: string | null }
export type Backup = { file: string; size: number; created_at: string }
export type Account = { email: string; name: string; company: string; created_at: string; databases: number; disallowed: string[] }

// admin methods always target the main database, even while this browser is inside a trial
const main = <T>(method: string, args: unknown[] = []) => rpc<T>({ method, args, db: null })
export const listDbs = () => main<DbRow[]>('admin_db_list')
export const backupDb = (db: string) => main<Backup>('admin_db_backup', [db])
export const listBackups = (db: string) => main<Backup[]>('admin_db_backups', [db])
export const deleteBackup = (db: string, file: string) => main('admin_db_backup_delete', [db, file])
export const restoreDb = (db: string, file: string) => main('admin_db_restore', [db, file, db])
export const archiveDb = (db: string, on: boolean) => main('admin_db_archive', [db, on])
export const deleteDb = (db: string, keepBackups: boolean) => main('admin_db_delete', [db, db, keepBackups])
export const createDb = (f: { name: string; email: string; company: string; apps: string[] }) => main('admin_db_create', [f])
export const listAccounts = () => main<Account[]>('admin_account_list')
export const setDisallowed = (email: string, apps: string[]) => main('admin_account_set_apps', [email, apps])

const STASH = 'odoo_admin_token'
/** Work inside a database as its administrator; the console session is stashed so "Back to admin" can return to it. */
export async function openDb(db: string) {
  const r = await main<{ db: string; token: string }>('admin_db_open', [db])
  try { if (!getDb()) sessionStorage.setItem(STASH, getToken() ?? '') } catch { /* ignore */ }
  setDb(r.db); setToken(r.token); location.href = '/'
}
export const hasAdminStash = (): boolean => { try { return !!sessionStorage.getItem(STASH) && !!getDb() } catch { return false } }
export function backToAdmin() { try { const t = sessionStorage.getItem(STASH); sessionStorage.removeItem(STASH); setDb(null); setToken(t || null) } catch { /* ignore */ } location.href = '/admin/databases/' }

export const fmtSize = (n: number | null): string => n == null ? '—' : n < 1024 ? `${n} B` : n < 1048576 ? `${(n / 1024).toFixed(1)} KB` : n < 1073741824 ? `${(n / 1048576).toFixed(1)} MB` : `${(n / 1073741824).toFixed(1)} GB`
export type Filter = 'all' | 'main' | 'trial' | 'standard' | 'archived'
export const filterDbs = (rows: DbRow[], f: Filter, q: string): DbRow[] => {
  const s = q.trim().toLowerCase()
  return rows.filter((r) => (f === 'all' || (f === 'archived' ? r.status === 'archived' : r.kind === f)) && (!s || `${r.db} ${r.owner} ${r.company}`.toLowerCase().includes(s)))
}
/** Toggle one app in an account's disallowed list. */
export const toggleApp = (list: string[], app: string): string[] => (list.includes(app) ? list.filter((a) => a !== app) : [...list, app].sort())
