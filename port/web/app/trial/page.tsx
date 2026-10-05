'use client'
import Link from 'next/link'
import { useEffect, useMemo, useState } from 'react'
import { rpc } from '@/lib/rpc'
import { type Me, getPortalToken, portal, setPortalToken } from '@/lib/portal'
import { useT } from '@/lib/i18n'
import { countryOptions, guessCountry } from '@/lib/countries'
import { byCategory, preselect, toggle, validate, type Errors, type Form, type TrialApp } from '@/lib/trial'

type Step = 'apps' | 'form' | 'creating'
const EMPTY: Form = { name: '', email: '', phone: '', company: '', country: '', password: '', terms: false }
const STORE = 'odoo-rs-trial-apps'

export default function TrialPage() {
  const { t, lang } = useT()
  const [step, setStep] = useState<Step>('apps')
  const [cat, setCat] = useState<TrialApp[] | null>(null); const [loadErr, setLoadErr] = useState<string>()
  const [apps, setApps] = useState<string[]>([]); const [form, setForm] = useState<Form>(EMPTY); const [errs, setErrs] = useState<Errors>({})
  const [me, setMe] = useState<Me | null>(null)   // signed-in portal account (then the form only asks for company/country)
  const [from, setFrom] = useState<string>()   // landing page the visitor came from (industry or app)
  const [showPw, setShowPw] = useState(false); const [serverErr, setServerErr] = useState<string>(); const [phase, setPhase] = useState(0)

  useEffect(() => {
    rpc<TrialApp[]>({ method: 'trial_catalog', db: null }).then((c) => { setCat(c); const p = preselect(location.search, c); if (p && p.apps.length) { setApps(p.apps); setFrom(p.label) } }).catch((e) => setLoadErr((e as Error).message))
    try { const saved = JSON.parse(sessionStorage.getItem(STORE) || '[]'); if (Array.isArray(saved)) setApps(saved) } catch { /* ignore */ }
    setForm((f) => ({ ...f, country: guessCountry() }))
    if (getPortalToken()) portal<Me>('portal_me').then((m) => { setMe(m); setForm((f) => ({ ...f, country: m.country || f.country, company: m.company })) }).catch(() => setPortalToken(null))
  }, [])
  useEffect(() => { try { sessionStorage.setItem(STORE, JSON.stringify(apps)) } catch { /* ignore */ } }, [apps])
  const countries = useMemo(() => countryOptions(lang), [lang])
  const groups = useMemo(() => byCategory(cat ?? []), [cat])
  const chosen = apps.filter((a) => cat?.find((c) => c.name === a)?.available)

  const set = (k: keyof Form, v: string | boolean) => { setForm((f) => ({ ...f, [k]: v })); if (errs[k]) setErrs((e) => ({ ...e, [k]: undefined })) }
  const submit = async (e: React.FormEvent) => {
    e.preventDefault(); setServerErr(undefined)
    const v = me ? (chosen.length ? {} : { apps: 'apps' as const }) : validate(form, chosen); setErrs(v); if (Object.keys(v).length) return
    setStep('creating'); setPhase(0)
    const timers = [setTimeout(() => setPhase(1), 1500), setTimeout(() => setPhase(2), 4500)]
    try {
      if (me) {   // signed in: another database for the same account
        const r = await portal<{ db: string }>('portal_new_trial', [{ apps: chosen, company: form.company, country: form.country }])
        try { sessionStorage.removeItem(STORE) } catch { /* ignore */ }
        location.href = `/my/databases/?new=${r.db}`
      } else {
        const r = await rpc<{ db: string; portal_token: string }>({ method: 'trial_create', args: [{ ...form, apps: chosen }], db: null, token: null })
        setPortalToken(r.portal_token); try { sessionStorage.removeItem(STORE) } catch { /* ignore */ }
        location.href = `/my/?new=${r.db}`   // land on the customer portal, like odoo.com/my
      }
    } catch (x) { setServerErr((x as Error).message); setStep('form') } finally { timers.forEach(clearTimeout) }
  }
  const fieldErr = (k: keyof Errors) => errs[k] ? <div className="tr-err">{k === 'terms' ? t('trial.err_terms') : k === 'email' ? t('trial.err_email') : k === 'password' ? t('trial.password_hint') : k === 'name' ? t('trial.err_name') : t('trial.err_phone')}</div> : null

  return (
    <div className="tr">
      <header className="tr-head">
        <Link href="/trial/" className="tr-logo">{t('app.name')}</Link><span className="grow" />
        {me ? <Link href="/my/">👤 {me.name}</Link> : <Link href="/my/">{t('trial.signin')}</Link>}
        <Link href="/trial/" className="btn p">{t('trial.try')}</Link>
      </header>

      {step === 'apps' && (
        <>
          <section className="tr-hero">
            <h1>{t('trial.title_a')} <span className="tr-under">{t('trial.title_b')}</span></h1>
            <p>{t('trial.subtitle')}</p>
            {from && <p className="muted">Configured for <b>{from}</b> — adjust the apps below.</p>}
          </section>
          <main className="tr-body">
            {loadErr && <div className="err">{loadErr.includes('not enabled') ? t('trial.disabled') : loadErr}</div>}
            {!cat && !loadErr && <p className="muted">{t('common.loading')}</p>}
            {groups.map(([g, list]) => (
              <section key={g}>
                <h2 className="tr-cat">{g}</h2>
                <div className="tr-grid">
                  {list.map((a) => {
                    const on = apps.includes(a.name)
                    return (
                      <button key={a.name} type="button" className={`tr-card ${on ? 'on' : ''}`} disabled={!a.available} aria-pressed={on} title={a.available ? '' : t('trial.enterprise')} onClick={() => setApps((cur) => toggle(cur, a.name))}>
                        <span className="tr-ic">{a.icon}</span><span className="tr-name">{a.name}</span>
                        {a.available ? <span className="tr-check">{on ? '✓' : ''}</span> : <span className="tr-ent">{t('trial.enterprise')}</span>}
                      </button>
                    )
                  })}
                </div>
              </section>
            ))}
          </main>
          <div className="tr-bar">
            <span>{chosen.length === 0 ? t('trial.pick_one') : chosen.length === 1 ? t('trial.selected_one') : t('trial.selected', { n: chosen.length })}</span><span className="grow" />
            <button className="btn p" disabled={!chosen.length} onClick={() => setStep('form')}>{t('trial.continue')} →</button>
          </div>
        </>
      )}

      {step !== 'apps' && (
        <main className="tr-formwrap">
          <form className="card tr-form" onSubmit={submit} noValidate>
            <h1>{t('trial.form_title')}</h1>
            <p className="muted">{t('trial.form_sub', { apps: chosen.join(', ') })}</p>
            {serverErr && <div className="err">{serverErr}</div>}
            {me && <p className="hint">{t('portal.signed_in_as', { email: me.email })}</p>}
            {!me && <>
            <label>{t('trial.name')}<input autoComplete="name" value={form.name} onChange={(e) => set('name', e.target.value)} disabled={step === 'creating'} />{fieldErr('name')}</label>
            <label>{t('trial.email')}<input type="email" autoComplete="email" value={form.email} onChange={(e) => set('email', e.target.value)} disabled={step === 'creating'} />{fieldErr('email')}</label>
            </>}
            <div className="tr-two">
              {!me && <label>{t('trial.phone')}<input type="tel" autoComplete="tel" value={form.phone} onChange={(e) => set('phone', e.target.value)} disabled={step === 'creating'} />{fieldErr('phone')}</label>}
              <label>{t('trial.country')}<select value={form.country} onChange={(e) => set('country', e.target.value)} disabled={step === 'creating'}><option value="" />{countries.map((c) => <option key={c.code} value={c.code}>{c.name}</option>)}</select></label>
            </div>
            <label>{t('trial.company')}<input autoComplete="organization" value={form.company} onChange={(e) => set('company', e.target.value)} disabled={step === 'creating'} /></label>
            {!me && <>
            <label>{t('trial.password')}
              <span className="tr-pw"><input type={showPw ? 'text' : 'password'} autoComplete="new-password" value={form.password} onChange={(e) => set('password', e.target.value)} disabled={step === 'creating'} /><button type="button" className="btn" onClick={() => setShowPw(!showPw)}>{showPw ? t('trial.hide') : t('trial.show')}</button></span>
              <span className="hint">{t('trial.password_hint')}</span>{fieldErr('password')}
            </label>
            <label className="tr-terms"><input type="checkbox" checked={form.terms} onChange={(e) => set('terms', e.target.checked)} disabled={step === 'creating'} /> {t('trial.terms')}{fieldErr('terms')}</label>
            </>}
            {step === 'creating'
              ? <div className="tr-progress" role="status"><div className="tr-spin" /><b>{t(phase === 0 ? 'trial.creating' : phase === 1 ? 'trial.installing' : 'trial.finishing')}</b></div>
              : <div className="tr-actions"><button type="button" className="btn" onClick={() => setStep('apps')}>← {t('trial.back')}</button><button className="btn p" type="submit">{t('trial.start')}</button></div>}
            {!me && <p className="hint">{t('portal.have_account')} <Link href="/my/">{t('portal.signin')}</Link></p>}
            <p className="hint">{t('trial.expires', { days: 15 })}</p>
          </form>
        </main>
      )}
    </div>
  )
}
