'use client'
import { useEffect, useMemo, useState } from 'react'
import { useT } from '@/lib/i18n'
import type { ThemeDef } from '@/lib/vibe/themes'

type Mod = typeof import('@/lib/vibe/themes')
const PAGE = 96

export function Appearance() {
  const { t } = useT()
  const [m, setM] = useState<Mod | null>(null)
  const [cur, setCur] = useState<string>(''); const [q, setQ] = useState(''); const [cat, setCat] = useState('all'); const [mode, setMode] = useState<'all' | 'dark' | 'light'>('all'); const [more, setMore] = useState(PAGE)
  useEffect(() => { import('@/lib/vibe/themes').then((mod) => { setM(mod); try { setCur(localStorage.getItem('odoo-rs-theme-id') || '') } catch { /* ignore */ } }) }, [])
  const list = useMemo(() => (m?.THEMES ?? []).filter((x) => (cat === 'all' || x.category === cat) && (mode === 'all' || x.mode === mode) && (!q || `${x.name} ${x.pairId}`.toLowerCase().includes(q.toLowerCase()))), [m, q, cat, mode])
  const apply = (x: ThemeDef) => { m?.applyThemeById(x.id); setCur(x.id) }
  const active = m?.THEMES.find((x) => x.id === cur)
  const flip = () => { const p = active ? m?.getPairedTheme(active.id) : m?.THEMES.find((x) => x.id === (document.documentElement.getAttribute('data-theme') === 'dark' ? m.DEFAULT_LIGHT_THEME_ID : m.DEFAULT_DARK_THEME_ID)); if (p) apply(p) }
  return (
    <div className="set-sec">
      <h2>{t('settings.theme')}</h2><p className="hint">{t('settings.theme_hint')}</p>
      <div className="bar">
        <button className="btn" onClick={flip}>{(active?.mode ?? 'dark') === 'dark' ? `☀ ${t('settings.light')}` : `☾ ${t('settings.dark')}`}</button>
        <input placeholder={t('settings.search_themes')} value={q} onChange={(e) => { setQ(e.target.value); setMore(PAGE) }} style={{ minWidth: 220 }} />
        <span className="muted">{list.length} {t('settings.themes_count')}</span>
      </div>
      <div className="chips">
        {['all', ...(m?.THEME_CATEGORIES.map((c) => c.id) ?? [])].map((c) => <a key={c} className={`pill ${cat === c ? 'sale' : ''}`} style={{ cursor: 'pointer' }} onClick={() => { setCat(c); setMore(PAGE) }}>{c === 'all' ? t('settings.all') : t(`settings.cat.${c}` as never)}</a>)}
        <span style={{ width: 12 }} />
        {(['all', 'dark', 'light'] as const).map((x) => <a key={x} className={`pill ${mode === x ? 'sale' : ''}`} style={{ cursor: 'pointer' }} onClick={() => { setMode(x); setMore(PAGE) }}>{x === 'all' ? t('settings.all') : x === 'dark' ? t('settings.dark') : t('settings.light')}</a>)}
      </div>
      {!m && <p className="muted">{t('common.loading')}</p>}
      <div className="themes">
        {list.slice(0, more).map((x) => (
          <button key={x.id} className={`theme ${cur === x.id ? 'on' : ''}`} onClick={() => apply(x)} title={x.id}>
            <div className="sw" style={{ background: x.preview.bg }}><i style={{ background: x.preview.bg }} /><i style={{ background: x.preview.secondary }} /><i style={{ background: x.preview.accent }} /><i style={{ background: x.preview.fg }} /></div>
            <div style={{ fontWeight: 600, fontSize: 13 }}>{x.name}</div><div className="muted" style={{ fontSize: 11 }}>{x.mode === 'dark' ? t('settings.dark') : t('settings.light')} · {t(`settings.cat.${x.category}` as never)}</div>
          </button>
        ))}
      </div>
      {list.length > more && <p><button className="btn" onClick={() => setMore(more + PAGE)}>+{Math.min(PAGE, list.length - more)}</button></p>}
    </div>
  )
}
