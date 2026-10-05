'use client'
import Link from 'next/link'
import { Suspense } from 'react'
import { useSearchParams } from 'next/navigation'
import { Portal } from '@/components/Portal'
import { OpenButton } from '@/components/OpenButton'
import { useT } from '@/lib/i18n'
import { countryOptions } from '@/lib/countries'
import { sitesFor } from '@/lib/portal'
import { Icon } from '@/components/Icon'

function Home() {
  const { t, lang } = useT(); const q = useSearchParams(); const fresh = q.get('new')
  return (
    <Portal title={t('portal.title')} active="home">
      {(me) => {
        const country = countryOptions(lang).find((c) => c.code === me.country)?.name ?? me.country
        const created = fresh ? me.databases.find((d) => d.db === fresh) : undefined
        return (
          <>
            {created && (
              <div className="card pt-ready"><div><b><Icon name="check-circle" size="var(--icon-lg)" /> {t('portal.db_new_banner')}</b><div className="muted">{created.apps.join(', ')} · {t('portal.db_days', { n: created.days_left })}</div></div><span className="grow" /><OpenButton db={created.db} primary /></div>
            )}
            <h2 style={{ margin: '4px 0 10px' }}>{t('portal.sites')}</h2>
            {me.databases.length === 0 && <p className="muted">{t('portal.db_none')}</p>}
            {me.databases.map((d) => (
              <section key={d.db} className="card" style={{ marginBottom: 14, padding: 16 }}>
                <b>{d.company || d.db}</b> <code className="muted">{d.db}</code>
                <div className="pt-tiles" style={{ marginTop: 10 }}>
                  {sitesFor(d.apps).map((s) => (
                    <div key={s.key} className="card pt-tile"><span className="pt-ico"><Icon name={s.icon} /></span><div style={{ flex: 1 }}><b>{s.name}</b><div className="muted">{s.desc}</div></div><OpenButton db={d.db} to={s.to} label={t('portal.db_open')} /></div>
                  ))}
                </div>
              </section>
            ))}
            <div className="pt-cols">
              <section className="pt-tiles">
                <Link href="/my/databases/" className="card pt-tile"><span className="pt-ico"><Icon name="database" /></span><div><b>{t('portal.tile_db')} <span className="pill">{me.databases.length}</span></b><div className="muted">{t('portal.tile_db_desc')}</div></div></Link>
                <Link href="/my/security/" className="card pt-tile"><span className="pt-ico"><Icon name="lock" /></span><div><b>{t('portal.tile_security')}</b><div className="muted">{t('portal.tile_security_desc')}</div></div></Link>
                <Link href="/my/details/" className="card pt-tile"><span className="pt-ico"><Icon name="pen" /></span><div><b>{t('portal.tile_details')}</b><div className="muted">{t('portal.tile_details_desc')}</div></div></Link>
                <Link href="/trial/" className="card pt-tile"><span className="pt-ico"><Icon name="plus" /></span><div><b>{t('portal.link_new')}</b><div className="muted">{t('portal.db_limit', { n: me.max_databases })}</div></div></Link>
              </section>
              <aside className="card pt-contact">
                <b style={{ fontSize: 16 }}>{me.name}</b>
                {me.company && <div>{me.company}</div>}{country && <div>{country}</div>}{me.phone && <div>{me.phone}</div>}<div>{me.email}</div>
                <Link href="/my/details/" className="btn" style={{ marginTop: 10, display: 'inline-block' }}>{t('portal.edit')}</Link>
                <h3 style={{ margin: '18px 0 6px' }}>{t('portal.useful')}</h3>
                <Link href="/my/databases/">{t('portal.link_dbs')}</Link><br /><Link href="/">{t('portal.link_app')}</Link>
              </aside>
            </div>
          </>
        )
      }}
    </Portal>
  )
}
export default function Page() { return <Suspense><Home /></Suspense> }
