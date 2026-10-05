// Shared menu helpers. Odoo's own "Settings" and "Apps" roots are merged into the unified /settings and /apps screens,
// and every res.config.settings page (General Settings + the per-app "Settings" entries) opens the unified Settings screen.
import type { Menu } from './hooks'

export const ADMIN_ROOT = 'base.menu_administration'
const HIDDEN_ROOTS = new Set([ADMIN_ROOT, 'base.menu_management'])
export const visibleApps = (menus: Menu[] | undefined): Menu[] => (menus ?? []).filter((m) => !HIDDEN_ROOTS.has(m.id))
export const adminRoot = (menus: Menu[] | undefined): Menu | undefined => menus?.find((m) => m.id === ADMIN_ROOT)

export const leafHref = (m: Menu): string => {
  if (m.model === 'res.config.settings') return '/settings/'
  if (m.model === 'ir.module.module') return '/apps/'
  const q = new URLSearchParams({ model: m.model!, title: m.name })
  if (m.domain?.length) q.set('domain', JSON.stringify(m.domain))
  return `/${m.view_mode?.split(',')[0] === 'kanban' ? 'kanban' : 'list'}/?${q}`
}
