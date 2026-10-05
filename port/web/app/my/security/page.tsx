'use client'
import { useState } from 'react'
import { Portal } from '@/components/Portal'
import { useT } from '@/lib/i18n'
import { portal } from '@/lib/portal'

export default function Security() {
  const { t } = useT(); const [old, setOld] = useState(''); const [nw, setNw] = useState(''); const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null)
  const go = async (e: React.FormEvent) => {
    e.preventDefault(); setMsg(null)
    try { await portal('portal_change_password', [{ old, new: nw }]); setOld(''); setNw(''); setMsg({ ok: true, text: t('portal.pw_changed') }) } catch (x) { setMsg({ ok: false, text: (x as Error).message }) }
  }
  return (
    <Portal title={t('portal.tile_security')} active="security">
      {() => (
        <form className="card tr-form" onSubmit={go} style={{ width: 'min(480px,100%)' }}>
          <h2 style={{ margin: 0 }}>{t('portal.pw_change')}</h2>
          {msg && <div className={msg.ok ? 'card form' : 'err'}>{msg.text}</div>}
          <label>{t('portal.pw_current')}<input type="password" autoComplete="current-password" value={old} onChange={(e) => setOld(e.target.value)} /></label>
          <label>{t('portal.pw_new')}<input type="password" autoComplete="new-password" value={nw} onChange={(e) => setNw(e.target.value)} /><span className="hint">{t('trial.password_hint')}</span></label>
          <button className="btn p" type="submit" disabled={!old || nw.length < 8}>{t('portal.pw_change')}</button>
        </form>
      )}
    </Portal>
  )
}
