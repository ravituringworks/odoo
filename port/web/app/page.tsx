'use client'
import { useT } from '@/lib/i18n'
import { leafHref, visibleApps } from '@/lib/menu'
import Link from 'next/link'
import { useMenus } from '@/lib/hooks'

export default function Home() {
  const { t } = useT()
  const { data, loading, error } = useMenus()
  const first = (m: NonNullable<typeof data>[number]): NonNullable<typeof data>[number] | undefined => (m.model ? m : m.children.map(first).find(Boolean))
  return (
    <>
      <div className="bar"><h1>{t('home.title')}</h1></div>
      {error && <div className="err">{t('home.backend_down')}: {error}. Start it with <code>cargo run -p odoo-server</code>.</div>}
      {loading && <p className="muted">{t('list.loading')}</p>}
      <div className="home">
        <Link className="card tile" href="/apps/"><div className="ic">+</div>{t('home.install')}</Link>
        {visibleApps(data).map((m) => {
          const leaf = first(m)
          return leaf ? (
            <Link key={m.id} className="card tile" href={/^point of sale$/i.test(m.name) ? '/pos/' : leafHref(leaf)}>
              <div className="ic">{m.name[0]}</div>{m.name}
            </Link>
          ) : null
        })}
      </div>
    </>
  )
}
