'use client'
import { useT } from '@/lib/i18n'
import { AiPanel, AI_TOGGLE } from './AiPanel'
import Link from 'next/link'
import { useState } from 'react'
import { useEffect } from 'react'
import { useMenus, type Menu } from '@/lib/hooks'
import { usePathname } from 'next/navigation'
import { getDb, getToken, rpc, setDb, setToken } from '@/lib/rpc'
import { leafHref, visibleApps } from '@/lib/menu'
import { backToAdmin, hasAdminStash } from '@/lib/admin'

function Tree({ items }: { items: Menu[] }) {
  return (
    <div className="sub">
      {items.map((m) =>
        m.model ? (
          <Link key={m.id} href={leafHref(m)}>{m.name}</Link>
        ) : (
          <div key={m.id}><div className="grp">{m.name}</div><Tree items={m.children} /></div>
        ),
      )}
    </div>
  )
}

function Login({ onDone }: { onDone: () => void }) {
  const { t } = useT()
  const [login, setLogin] = useState(''); const [pw, setPw] = useState(''); const [err, setErr] = useState<string>()
  const go = async (e: React.FormEvent) => {
    e.preventDefault()
    try { const r = await rpc<{ token: string }>({ method: 'login', args: [login, pw] }); setToken(r.token); onDone() } catch (x) { setErr((x as Error).message) }
  }
  return (
    <form onSubmit={go} className="card form" style={{ maxWidth: 340, margin: '12vh auto' }}>
      <h1 style={{ marginBottom: 14 }}>{t('auth.login')}</h1>
      {err && <div className="err">{err}</div>}
      <div style={{ display: 'grid', gap: 10 }}>
        <input placeholder={t('auth.login_field')} autoComplete="username" value={login} onChange={(e) => setLogin(e.target.value)} />
        <input placeholder={t('auth.password')} type="password" autoComplete="current-password" value={pw} onChange={(e) => setPw(e.target.value)} />
        <button className="btn p" type="submit">{t('auth.login')}</button>
        {getDb() && <div className="hint">{t('auth.trial_db')}: <code>{getDb()}</code> · <a onClick={() => { setDb(null); setToken(null); location.reload() }}>{t('auth.use_main')}</a></div>}
        <div className="hint" style={{ textAlign: 'center' }}><Link href="/trial/">{t('auth.trial_cta')}</Link></div>
      </div>
    </form>
  )
}

export function Shell({ children }: { children: React.ReactNode }) {
  const path = usePathname()
  // public pages (free-trial signup) render without the app chrome or the login gate
  if (path?.startsWith('/trial') || path?.startsWith('/my') || path?.startsWith('/order') || path?.startsWith('/industries') || path?.startsWith('/app/') || path === '/app') return <>{children}</>
  // auth gate: Tauri/dev-mode servers answer `whoami` without a session; otherwise show the login form
  const [authed, setAuthed] = useState<boolean | null>(null)
  const check = () => rpc<{ uid: number } | null>({ method: 'whoami' }).then((r) => setAuthed(!!r), () => setAuthed(false))
  useEffect(() => { check(); const h = () => setAuthed(false); window.addEventListener('odoo-auth', h); return () => window.removeEventListener('odoo-auth', h) }, [])
  if (authed === null) return null
  if (!authed) return <Login onDone={() => { setAuthed(true); location.reload() }} />
  // the POS terminal is a full-screen app: authenticated, but without the back-office chrome
  if (path?.startsWith('/pos')) return <>{children}</>
  return <Inner>{children}</Inner>
}

type CompanyState = { current: number | null; allowed: number[]; companies: { id: number; name: string }[] }

/** Odoo's company switcher: the first ticked company is active; every ticked one is visible. Hidden for single-company users. */
function CompanySwitcher() {
  const { t } = useT()
  const [st, setSt] = useState<CompanyState | null>(null)
  useEffect(() => { rpc<CompanyState>({ method: 'companies' }).then(setSt, () => setSt(null)) }, [])
  if (!st || st.companies.length < 2) return null
  const apply = (ids: number[]) => { if (ids.length) rpc({ method: 'switch_company', args: [ids] }).then(() => location.reload(), () => {}) }
  const toggle = (id: number) => apply(st.allowed.includes(id) ? st.allowed.filter((x) => x !== id) : [...st.allowed, id])
  return (
    <div className="sub" aria-label={t('nav.companies')}>
      {st.companies.map((c) => (
        <div key={c.id} style={{ display: 'flex', gap: 6, alignItems: 'center', fontSize: 13 }}>
          <input type="checkbox" checked={st.allowed.includes(c.id)} onChange={() => toggle(c.id)} />
          <a onClick={() => apply([c.id, ...st.allowed.filter((x) => x !== c.id)])} style={{ fontWeight: c.id === st.current ? 700 : 400 }}>{c.name}</a>
        </div>
      ))}
    </div>
  )
}

function Inner({ children }: { children: React.ReactNode }) {
  const { t } = useT()
  const { data: menus, error } = useMenus()
  const [open, setOpen] = useState<string | null>(null)
  return (
    <div className="shell">
      <nav className="side">
        <div className="brand"><Link href="/">Odoo&nbsp;RS</Link> {getToken() && <a className="muted" style={{ float: 'right', fontSize: 12 }} onClick={() => { rpc({ method: 'logout' }).finally(() => { setToken(null); location.href = '/' }) }}>{t('auth.logout')}</a>}</div>
        <CompanySwitcher />
        <Link className="app" href="/apps/">{t('nav.apps_install')}</Link>
        <a className="app" onClick={() => window.dispatchEvent(new Event(AI_TOGGLE))}>✦ {t('nav.ai')}</a>
        <Link className="app" href="/settings/">{t('nav.settings')}</Link>
        {getDb() && <Link className="app" href="/my/">{t('portal.link_dbs')}</Link>}
        {hasAdminStash() && <a className="app" onClick={backToAdmin}>← Back to admin</a>}
        {!getDb() && <Link className="app" href="/admin/databases/">Databases</Link>}
        {error && <div className="err">{error}</div>}
        {visibleApps(menus).map((m) => (
          <div key={m.id}>
            <a className={`app ${open === m.id ? 'on' : ''}`} onClick={() => setOpen(open === m.id ? null : m.id)}>{m.name}</a>
            {open === m.id && (m.model ? <div className="sub"><Link href={leafHref(m)}>{m.name}</Link></div> : <Tree items={m.children} />)}
          </div>
        ))}
      </nav>
      <main>{children}</main>
      <AiPanel />
    </div>
  )
}
