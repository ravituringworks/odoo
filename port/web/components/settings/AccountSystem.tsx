'use client'
import Link from 'next/link'
import { useEffect, useState } from 'react'
import { call, rpc } from '@/lib/rpc'
import { useT } from '@/lib/i18n'
import { useMenus, type Menu } from '@/lib/hooks'
import { adminRoot, leafHref } from '@/lib/menu'

export function AccountTab({ uid }: { uid: number | null }) {
  const { t } = useT(); const [pw, setPw] = useState(''); const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null)
  const go = async () => {
    if (pw.length < 8) { setMsg({ ok: false, text: t('settings.password_short') }); return }
    try { await call('res.users', 'change_password', [uid, pw]); setPw(''); setMsg({ ok: true, text: t('settings.password_changed') }) } catch (e) { setMsg({ ok: false, text: (e as Error).message }) }
  }
  return (
    <div className="set-sec">
      <h2>{t('settings.change_password')}</h2>
      <div className="row"><label>{t('settings.new_password')}</label><input type="password" autoComplete="new-password" value={pw} onChange={(e) => setPw(e.target.value)} /></div>
      {msg && <div className={msg.ok ? 'card form' : 'err'} style={{ margin: '10px 0' }}>{msg.text}</div>}
      <button className="btn p" disabled={!uid} onClick={go}>{t('settings.save')}</button>
    </div>
  )
}

export function AdminTab() {
  const { t } = useT(); const { data: menus } = useMenus()
  const [s, setS] = useState<{ system: { backend: string; modules: number; models: number; version: string } } | null>(null)
  useEffect(() => { rpc<typeof s>({ method: 'settings_get' }).then(setS).catch(() => undefined) }, [])
  const root = adminRoot(menus)
  // Odoo's own Settings menu (General Settings is this screen; Users & Companies, Translations, Technical… live here)
  const groups = (root?.children ?? []).filter((m) => m.model !== 'res.config.settings')
  const Leaves = ({ items, depth }: { items: Menu[]; depth: number }) => (
    <div style={{ marginInlineStart: depth * 14 }}>
      {items.map((m) => m.model
        ? <div key={m.id}><Link href={leafHref(m)}>{m.name}</Link></div>
        : <details key={m.id} open={depth === 0}><summary style={{ cursor: 'pointer', fontWeight: 600, margin: '6px 0' }}>{m.name}</summary><Leaves items={m.children} depth={depth + 1} /></details>)}
    </div>
  )
  return (
    <div className="set-sec">
      <h2>{t('settings.administration')}</h2>
      {s && <div className="card form" style={{ marginBottom: 16 }}>
        <div className="row"><label>{t('settings.backend')}</label><b>{s.system.backend} · v{s.system.version}</b></div>
        <div className="row"><label>{t('settings.modules_installed')}</label><b>{s.system.modules}</b></div>
        <div className="row"><label>{t('settings.models')}</label><b>{s.system.models}</b></div>
        <Link className="btn p" href="/apps/">{t('settings.manage_apps')}</Link>
      </div>}
      <div className="card form"><Leaves items={groups} depth={0} /></div>
    </div>
  )
}
