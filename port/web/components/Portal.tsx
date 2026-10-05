'use client'
import Link from 'next/link'
import { useCallback, useEffect, useState } from 'react'
import { useT } from '@/lib/i18n'
import { type Me, portal, setPortalToken, getPortalToken } from '@/lib/portal'
import { Icon } from './Icon'

/** Portal chrome + sign-in gate. Children render only for a signed-in account. */
export function Portal({ title, active, children }: { title?: string; active?: 'home' | 'databases' | 'security'; children: (me: Me, reload: () => void) => React.ReactNode }) {
  const { t } = useT()
  const [me, setMe] = useState<Me | null | undefined>(undefined)
  const reload = useCallback(() => { if (!getPortalToken()) { setMe(null); return } portal<Me>('portal_me').then(setMe).catch(() => { setPortalToken(null); setMe(null) }) }, [])
  useEffect(reload, [reload])
  const out = async () => { try { await portal('portal_logout') } catch { /* already gone */ } setPortalToken(null); setMe(null) }
  const nav = (href: string, key: 'home' | 'databases' | 'security', label: string) => <Link href={href} className={active === key ? 'pt-on' : ''}>{label}</Link>
  return (
    <div className="tr">
      <header className="tr-head">
        <Link href="/my/" className="tr-logo">{t('app.name')}</Link>
        {me && <nav className="pt-nav">{nav('/my/', 'home', t('portal.title'))}{nav('/my/databases/', 'databases', t('portal.databases'))}{nav('/my/security/', 'security', t('portal.tile_security'))}</nav>}
        <span className="grow" />
        {me ? <details className="pt-menu"><summary><Icon name="user" size="var(--icon-md)" /> {me.name}</summary><div className="card"><div className="muted" style={{ padding: '6px 12px' }}>{me.email}</div><a onClick={out}>{t('portal.signout')}</a></div></details>
          : <Link href="/trial/" className="btn p">{t('trial.try')}</Link>}
      </header>
      {me === undefined && <main className="tr-formwrap"><p className="muted">{t('common.loading')}</p></main>}
      {me === null && <SignIn onDone={reload} />}
      {me && <main className="pt-main">{title && <h1>{title}</h1>}{children(me, reload)}</main>}
    </div>
  )
}

function SignIn({ onDone }: { onDone: () => void }) {
  const { t } = useT()
  const [email, setEmail] = useState(''); const [pw, setPw] = useState(''); const [err, setErr] = useState<string>(); const [busy, setBusy] = useState(false)
  const go = async (e: React.FormEvent) => {
    e.preventDefault(); setBusy(true); setErr(undefined)
    try { const r = await portal<{ token: string }>('portal_login', [email, pw]); setPortalToken(r.token); onDone() } catch (x) { setErr((x as Error).message) } finally { setBusy(false) }
  }
  return (
    <main className="tr-formwrap">
      <form className="card tr-form" onSubmit={go} style={{ width: 'min(420px,100%)' }}>
        <h1>{t('portal.signin_title')}</h1>
        {err && <div className="err">{err}</div>}
        <label>{t('portal.email')}<input type="email" autoComplete="username" value={email} onChange={(e) => setEmail(e.target.value)} /></label>
        <label>{t('portal.password')}<input type="password" autoComplete="current-password" value={pw} onChange={(e) => setPw(e.target.value)} /></label>
        <button className="btn p" type="submit" disabled={busy || !email || !pw}>{t('portal.signin')}</button>
        <p className="hint" style={{ textAlign: 'center' }}>{t('portal.no_account')} <Link href="/trial/">{t('portal.start_trial')}</Link></p>
      </form>
    </main>
  )
}
