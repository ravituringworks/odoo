// Customer-portal client: its session (`odoo_portal_token`) is separate from an application session, so signing out of one never affects the other.
import { rpc } from './rpc'
export type PortalDb = { db: string; company: string; apps: string[]; created_at: string; days_left: number }
export type Me = { email: string; name: string; company: string; phone: string; country: string; created_at: string; databases: PortalDb[]; max_databases: number; trial_days: number }

const KEY = 'odoo_portal_token'
export const getPortalToken = (): string | null => { try { return localStorage.getItem(KEY) } catch { return null } }
export const setPortalToken = (t: string | null) => { try { t ? localStorage.setItem(KEY, t) : localStorage.removeItem(KEY) } catch { /* ignore */ } }

/** Portal RPC: always the main server (`db: null`) with the portal token. */
export const portal = <T,>(method: string, args: unknown[] = []) => rpc<T>({ method, args, token: getPortalToken(), db: null })

export const daysLeftKind = (n: number): 'ok' | 'warn' | 'bad' => (n <= 0 ? 'bad' : n <= 3 ? 'warn' : 'ok')
export function formatDate(iso: string, lang: string): string {
  const d = new Date(iso.replace(' ', 'T') + 'Z'); if (isNaN(d.getTime())) return iso.slice(0, 10)
  try { return d.toLocaleDateString(lang, { year: 'numeric', month: 'short', day: 'numeric' }) } catch { return iso.slice(0, 10) }
}
