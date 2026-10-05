'use client'
import { useEffect, useState } from 'react'
import { rpc } from '@/lib/rpc'
import { useT } from '@/lib/i18n'
import type { Key } from '@/lib/i18n/en'
import { Icon } from '@/components/Icon'

type Field = { id: string; label: string; secret: boolean; options: string[]; placeholder: string }
type Provider = { id: string; label: string; kind: 'http' | 'smtp'; docs: string; fields: Field[] }
type Email = { enabled: boolean; provider: string; from_address: string; from_name: string; reply_to: string; domain: string; region: string; smtp_host: string; smtp_port: number; smtp_security: string; smtp_user: string; base_url: string; app_url: string; has_api_key: boolean; has_smtp_password: boolean }
type View = { email: Email | null; email_providers: Provider[] }

// labels reuse translated keys where one exists; unknown provider-declared fields fall back to the server's English label
const LABEL: Record<string, Key> = { api_key: 'settings.api_key', domain: 'email.domain', region: 'email.region', smtp_host: 'email.smtp_host', smtp_port: 'email.smtp_port', smtp_security: 'email.smtp_security', smtp_user: 'email.smtp_user', smtp_password: 'email.smtp_password' }
const SECRET_FLAG: Record<string, keyof Email> = { api_key: 'has_api_key', smtp_password: 'has_smtp_password' }

export function EmailTab({ admin }: { admin: boolean }) {
  const { t } = useT()
  const [v, setV] = useState<View | null>(null); const [form, setForm] = useState<Record<string, string | boolean>>({}); const [secrets, setSecrets] = useState<Record<string, string>>({})
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null); const [busy, setBusy] = useState(false); const [to, setTo] = useState('')
  const load = (r: View) => { setV(r); const e = r.email; if (e) setForm({ enabled: e.enabled, provider: e.provider, from_address: e.from_address, from_name: e.from_name, reply_to: e.reply_to, domain: e.domain, region: e.region, smtp_host: e.smtp_host, smtp_port: String(e.smtp_port), smtp_security: e.smtp_security, smtp_user: e.smtp_user, base_url: e.base_url, app_url: e.app_url }) }
  useEffect(() => { rpc<View>({ method: 'settings_get' }).then(load).catch((e) => setMsg({ ok: false, text: String(e.message) })) }, [])
  if (!v) return <p className="muted">{t('common.loading')}</p>
  if (!v.email) return <div className="err">{t('settings.admin_only')}</div>
  const prov = v.email_providers.find((p) => p.id === form.provider) ?? v.email_providers[0]
  const set = (k: string, val: string | boolean) => setForm((f) => ({ ...f, [k]: val }))
  const payload = () => {
    const out: Record<string, unknown> = {}
    for (const [k, val] of Object.entries(form)) out[`email.${k}`] = typeof val === 'string' ? val : val
    for (const [k, val] of Object.entries(secrets)) if (val) out[`email.${k}`] = val
    return out
  }
  const save = async (): Promise<boolean> => {
    setBusy(true); setMsg(null)
    try { const r = await rpc<View>({ method: 'settings_set', args: [payload()] }); load(r); setSecrets({}); setMsg({ ok: true, text: t('settings.saved') }); return true }
    catch (e) { setMsg({ ok: false, text: (e as Error).message }); return false } finally { setBusy(false) }
  }
  const test = async () => {
    if (!(await save())) return
    setBusy(true); setMsg({ ok: true, text: t('email.sending') })
    try { await rpc({ method: 'email_test', args: [{ to }] }); setMsg({ ok: true, text: `${t('email.sent')}: ${to}` }) } catch (e) { setMsg({ ok: false, text: (e as Error).message }) } finally { setBusy(false) }
  }
  const ro = !admin
  const input = (f: Field) => {
    const label = LABEL[f.id] ? t(LABEL[f.id]) : f.label
    const flag = SECRET_FLAG[f.id]; const has = flag ? Boolean(v.email![flag]) : false
    return (
      <div className="row" key={f.id}><label>{label}</label>
        {f.options.length ? <select disabled={ro} value={String(form[f.id] ?? '')} onChange={(e) => set(f.id, e.target.value)}>{f.options.map((o) => <option key={o} value={o}>{o}</option>)}</select>
          : f.secret ? <div><input disabled={ro} type="password" autoComplete="off" value={secrets[f.id] ?? ''} placeholder={has ? '••••••••' : f.placeholder} onChange={(e) => setSecrets({ ...secrets, [f.id]: e.target.value })} style={{ width: '100%' }} /><div className="hint" style={{ margin: '4px 0 0' }}>{has ? <><Icon name="check" size="var(--icon-md)" /> {t('settings.key_set')}</> : t('settings.key_unset')} · {t('settings.key_write_only')}</div></div>
          : <input disabled={ro} value={String(form[f.id] ?? '')} placeholder={f.placeholder} onChange={(e) => set(f.id, e.target.value)} />}
      </div>
    )
  }
  return (
    <div className="set-sec">
      <h2>{t('settings.email')}</h2><p className="hint">{t('settings.email_hint')}</p>
      {ro && <div className="err">{t('settings.admin_only')}</div>}
      <div className="row"><label>{t('email.enable')}</label><input type="checkbox" disabled={ro} checked={!!form.enabled} onChange={(e) => set('enabled', e.target.checked)} style={{ justifySelf: 'start' }} /></div>
      <div className="row"><label>{t('settings.provider')}</label><select disabled={ro} value={String(form.provider)} onChange={(e) => set('provider', e.target.value)}>{v.email_providers.map((p) => <option key={p.id} value={p.id}>{p.label}</option>)}</select></div>
      {prov && <p className="hint">{prov.docs}</p>}
      {prov?.fields.map(input)}
      <div className="row"><label>{t('email.from_address')}</label><input disabled={ro} type="email" value={String(form.from_address ?? '')} placeholder="noreply@example.com" onChange={(e) => set('from_address', e.target.value)} /></div>
      <div className="row"><label>{t('email.from_name')}</label><input disabled={ro} value={String(form.from_name ?? '')} onChange={(e) => set('from_name', e.target.value)} /></div>
      <div className="row"><label>{t('email.reply_to')}</label><input disabled={ro} type="email" value={String(form.reply_to ?? '')} onChange={(e) => set('reply_to', e.target.value)} /></div>
      <details style={{ margin: '8px 0' }}><summary style={{ cursor: 'pointer' }}>{t('email.advanced')}</summary>
        <div className="row"><label>{t('email.app_url')}</label><div><input disabled={ro} value={String(form.app_url ?? '')} placeholder="https://erp.example.com" onChange={(e) => set('app_url', e.target.value)} style={{ width: '100%' }} /><div className="hint" style={{ margin: '4px 0 0' }}>{t('email.app_url_hint')}</div></div></div>
        {prov?.kind === 'http' && <div className="row"><label>{t('email.base_url')}</label><input disabled={ro} value={String(form.base_url ?? '')} placeholder="https://…" onChange={(e) => set('base_url', e.target.value)} /></div>}
      </details>
      {msg && <div className={msg.ok ? 'card form' : 'err'} style={{ margin: '10px 0' }}>{msg.text}</div>}
      {!ro && <>
        <div style={{ display: 'flex', gap: 8 }}><button className="btn p" disabled={busy} onClick={save}>{t('settings.save')}</button></div>
        <div className="row" style={{ marginTop: 18 }}><label>{t('email.test_to')}</label><div style={{ display: 'flex', gap: 8 }}><input type="email" value={to} placeholder="you@example.com" onChange={(e) => setTo(e.target.value)} style={{ flex: 1 }} /><button className="btn" disabled={busy || !to} onClick={test}>{t('email.send_test')}</button></div></div>
      </>}
    </div>
  )
}
