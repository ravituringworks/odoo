'use client'
import Link from 'next/link'
import { Suspense } from 'react'
import { useSearchParams } from 'next/navigation'
import { Portal } from '@/components/Portal'
import { OpenButton } from '@/components/OpenButton'
import { useT } from '@/lib/i18n'
import { countryOptions } from '@/lib/countries'

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
              <div className="card pt-ready"><div><b>🎉 {t('portal.db_new_banner')}</b><div className="muted">{created.apps.join(', ')} · {t('portal.db_days', { n: created.days_left })}</div></div><span className="grow" /><OpenButton db={created.db} primary /></div>
            )}
            <div className="pt-cols">
              <section className="pt-tiles">
                <Link href="/my/databases/" className="card pt-tile"><span className="pt-ico">🗄️</span><div><b>{t('portal.tile_db')} <span className="pill">{me.databases.length}</span></b><div className="muted">{t('portal.tile_db_desc')}</div></div></Link>
                <Link href="/my/security/" className="card pt-tile"><span className="pt-ico">🔐</span><div><b>{t('portal.tile_security')}</b><div className="muted">{t('portal.tile_security_desc')}</div></div></Link>
                <Link href="/my/details/" className="card pt-tile"><span className="pt-ico">✏️</span><div><b>{t('portal.tile_details')}</b><div className="muted">{t('portal.tile_details_desc')}</div></div></Link>
                <Link href="/trial/" className="card pt-tile"><span className="pt-ico">➕</span><div><b>{t('portal.link_new')}</b><div className="muted">{t('portal.db_limit', { n: me.max_databases })}</div></div></Link>
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
