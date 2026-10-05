import type { IconName } from './icons'
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

/** Navigable sites a database offers, derived from its apps. `app: null` = always available. */
export type Site = { key: string; icon: IconName; name: string; desc: string; to: string; app: string | null }
export const SITES: Site[] = [
  { key: 'backoffice', icon: 'building', name: 'Back office', desc: 'All your apps and menus', to: '/', app: null },
  { key: 'shop', icon: 'bag', name: 'Online shop', desc: 'Storefront with cart, checkout and order tracking', to: '/storefront/', app: 'eCommerce' },
  { key: 'pos', icon: 'receipt', name: 'Point of Sale', desc: 'Cash register terminal', to: '/pos/', app: 'Point of Sale' },
  { key: 'orders', icon: 'coins', name: 'Sales orders', desc: 'Quotations, online orders and fulfilment', to: '/list/?model=sale.order&title=Sales%20orders', app: 'Sales' },
  { key: 'campaigns', icon: 'megaphone', name: 'Campaigns', desc: 'Social and email campaigns with tracked links', to: '/list/?model=utm.campaign&title=Campaigns', app: 'Email Marketing' },
  { key: 'mailings', icon: 'mail', name: 'Mailings', desc: 'Mass mailings and mailing lists', to: '/list/?model=mailing.mailing&title=Mailings', app: 'Email Marketing' },
  { key: 'stock', icon: 'box', name: 'Deliveries', desc: 'Pickings and stock moves', to: '/list/?model=stock.picking&title=Deliveries', app: 'Inventory' },
]
export const sitesFor = (apps: string[]): Site[] => SITES.filter((s) => s.app === null || apps.includes(s.app))
