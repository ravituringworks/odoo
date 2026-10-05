'use client'
import { useEffect, useMemo, useState } from 'react'
import { Portal } from '@/components/Portal'
import { useT } from '@/lib/i18n'
import { countryOptions } from '@/lib/countries'
import { type Me, portal } from '@/lib/portal'

function Form({ me, reload }: { me: Me; reload: () => void }) {
  const { t, lang } = useT(); const [f, setF] = useState({ name: me.name, company: me.company, phone: me.phone, country: me.country }); const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null)
  const countries = useMemo(() => countryOptions(lang), [lang]); useEffect(() => setF({ name: me.name, company: me.company, phone: me.phone, country: me.country }), [me])
  const go = async (e: React.FormEvent) => { e.preventDefault(); setMsg(null); try { await portal('portal_update', [f]); reload(); setMsg({ ok: true, text: t('settings.saved') }) } catch (x) { setMsg({ ok: false, text: (x as Error).message }) } }
  return (
    <form className="card tr-form" onSubmit={go} style={{ width: 'min(520px,100%)' }}>
      {msg && <div className={msg.ok ? 'card form' : 'err'}>{msg.text}</div>}
      <label>{t('trial.name')}<input value={f.name} onChange={(e) => setF({ ...f, name: e.target.value })} /></label>
      <label>{t('portal.email')}<input value={me.email} disabled /></label>
      <label>{t('trial.company')}<input value={f.company} onChange={(e) => setF({ ...f, company: e.target.value })} /></label>
      <div className="tr-two"><label>{t('trial.phone')}<input type="tel" value={f.phone} onChange={(e) => setF({ ...f, phone: e.target.value })} /></label>
        <label>{t('trial.country')}<select value={f.country} onChange={(e) => setF({ ...f, country: e.target.value })}><option value="" />{countries.map((c) => <option key={c.code} value={c.code}>{c.name}</option>)}</select></label></div>
      <button className="btn p" type="submit">{t('settings.save')}</button>
    </form>
  )
}
export default function Details() { const { t } = useT(); return <Portal title={t('portal.tile_details')} active="home">{(me, reload) => <Form me={me} reload={reload} />}</Portal> }
