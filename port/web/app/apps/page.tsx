'use client'
import { useT } from '@/lib/i18n'
import { useMemo, useState } from 'react'
import { rpc } from '@/lib/rpc'
import { useAsync } from '@/lib/hooks'

type App = { name: string; title: string; summary: string | null; category: string | null; application: boolean; depends: string[]; installed: boolean; models: number; missing_deps: string[] }
type Filter = 'apps' | 'all' | 'installed'

export default function AppsPage() {
  const { t } = useT()
  const apps = useAsync(() => rpc<App[]>({ method: 'apps_list' }), [])
  const [q, setQ] = useState(''); const [filter, setFilter] = useState<Filter>('apps')
  const [busy, setBusy] = useState<string | null>(null); const [err, setErr] = useState<string>(); const [done, setDone] = useState<string[]>([])
  const shown = useMemo(() => (apps.data ?? []).filter((a) =>
    (filter === 'all' || (filter === 'installed' ? a.installed : a.application)) &&
    (!q || `${a.title} ${a.name} ${a.summary ?? ''} ${a.category ?? ''}`.toLowerCase().includes(q.toLowerCase()))), [apps.data, q, filter])

  const install = async (a: App) => {
    setBusy(a.name); setErr(undefined)
    try { const r = await rpc<{ installed: string[] }>({ method: 'install_module', args: [a.name] }); setDone(r.installed); apps.reload() }
    catch (e) { setErr((e as Error).message) } finally { setBusy(null) }
  }
  return (
    <>
      <div className="bar">
        <h1>{t('apps.title')}</h1>
        <input placeholder={t('apps.search')} value={q} onChange={(e) => setQ(e.target.value)} style={{ minWidth: 260 }} />
        {(['apps', 'all', 'installed'] as Filter[]).map((f) => <a key={f} className={`pill ${filter === f ? 'sale' : ''}`} style={{ cursor: 'pointer' }} onClick={() => setFilter(f)}>{f === 'apps' ? t('apps.f_apps') : f === 'all' ? t('apps.f_all') : t('apps.f_installed')}</a>)}
        <span className="grow" /><span className="muted">{shown.length} of {apps.data?.length ?? 0}</span>
      </div>
      {err && <div className="err">{err}</div>}
      {done.length > 0 && <div className="card form" style={{ marginBottom: 12 }}>{t('apps.just_installed')}: <b>{done.join(', ')}</b>. <a onClick={() => location.assign('/')}>{t('apps.reload_menus')}</a></div>}
      {apps.loading && <p className="muted">{t('common.loading')}</p>}
      <div className="home" style={{ gridTemplateColumns: 'repeat(auto-fill,minmax(300px,1fr))' }}>
        {shown.slice(0, 120).map((a) => (
          <div key={a.name} className="card form" style={{ display: 'grid', gap: 6, alignContent: 'start' }}>
            <div style={{ display: 'flex', gap: 10, alignItems: 'center' }}><div className="ic" style={{ width: 36, height: 36, fontSize: 16, margin: 0, borderRadius: 8, background: 'var(--brand)', color: '#fff', display: 'grid', placeItems: 'center' }}>{a.title[0]}</div><div><b>{a.title}</b><div className="muted" style={{ fontSize: 12 }}>{a.name}{a.category ? ` · ${a.category}` : ''}</div></div></div>
            <div className="muted" style={{ minHeight: 36 }}>{a.summary || `${a.models} models`}</div>
            <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
              {a.installed ? <span className="pill sale">{t('apps.installed')}</span>
                : a.missing_deps.length ? <span className="pill cancel" title={a.missing_deps.join(', ')}>{t('apps.missing_dep')}</span>
                : <button className="btn p" disabled={!!busy} onClick={() => install(a)}>{busy === a.name ? t('apps.installing') : t('apps.install')}</button>}
              {a.depends.length > 0 && !a.installed && <span className="muted" style={{ fontSize: 12 }}>{t('apps.needs')} {a.depends.slice(0, 3).join(', ')}{a.depends.length > 3 ? '…' : ''}</span>}
            </div>
          </div>
        ))}
      </div>
      {shown.length > 120 && <p className="muted">Showing 120 — refine the search.</p>}
    </>
  )
}
