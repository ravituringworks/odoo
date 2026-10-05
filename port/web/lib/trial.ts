// Pure helpers for the trial signup form (validation mirrors the server's rules so errors show before a round trip).
import { appBySlug } from './apps'
import { industryBySlug, trialApps } from './industries'
export type Form = { name: string; email: string; phone: string; company: string; country: string; password: string; terms: boolean }
export type TrialApp = { name: string; category: string; icon: string; available: boolean }
export type Errors = Partial<Record<keyof Form | 'apps', string>>

export const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@.]+$/
export function validate(f: Form, apps: string[]): Errors {
  const e: Errors = {}
  if (!f.name.trim()) e.name = 'name'
  if (f.name.length > 100) e.name = 'name'
  if (!EMAIL.test(f.email.trim()) || f.email.length > 120) e.email = 'email'
  if (f.phone && !/^[0-9 +\-().]{1,30}$/.test(f.phone)) e.phone = 'phone'
  if (f.password.length < 8) e.password = 'password'
  if (!f.terms) e.terms = 'terms'
  if (apps.length === 0) e.apps = 'apps'
  return e
}

export const toggle = (apps: string[], name: string): string[] => (apps.includes(name) ? apps.filter((a) => a !== name) : apps.length >= 15 ? apps : [...apps, name])

/** Group the catalog by category, keeping Odoo's category order. */
export function byCategory(cat: TrialApp[]): [string, TrialApp[]][] {
  const out: [string, TrialApp[]][] = []
  for (const a of cat) { const g = out.find(([c]) => c === a.category); if (g) g[1].push(a); else out.push([a.category, [a]]) }
  return out
}

/** Apps to preselect from a landing page link (`?industry=<slug>` or `?app=<slug>`), limited to what the catalog offers. */
export function preselect(search: string, cat: TrialApp[]): { label: string; apps: string[] } | null {
  const q = new URLSearchParams(search), ok = (n: string) => cat.some((c) => c.name === n && c.available)
  const ind = industryBySlug(q.get('industry') ?? ''), app = appBySlug(q.get('app') ?? '')
  if (ind) return { label: ind.name, apps: trialApps(ind).filter(ok) }
  if (app && ok(app.name)) return { label: app.name, apps: [app.name] }
  return null
}
