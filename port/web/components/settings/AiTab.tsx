'use client'
import { useEffect, useState } from 'react'
import { rpc } from '@/lib/rpc'
import { useT } from '@/lib/i18n'
import { Icon } from '@/components/Icon'

type Provider = { id: string; label: string; base_url: string; needs_key: boolean }
type View = { ai: { enabled: boolean; provider: string; model: string; base_url: string; has_key: boolean; temperature: number; system_prompt: string; max_rows: number }; providers: Provider[] }

export function AiTab({ admin }: { admin: boolean }) {
  const { t } = useT()
  const [v, setV] = useState<View | null>(null); const [form, setForm] = useState<Record<string, string | boolean | number>>({}); const [key, setKey] = useState('')
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null); const [busy, setBusy] = useState(false); const [models, setModels] = useState<string[]>([])
  useEffect(() => { rpc<View>({ method: 'settings_get' }).then((r) => { setV(r); setForm({ 'ai.enabled': r.ai.enabled, 'ai.provider': r.ai.provider, 'ai.model': r.ai.model, 'ai.base_url': r.ai.base_url, 'ai.temperature': r.ai.temperature, 'ai.system_prompt': r.ai.system_prompt, 'ai.max_rows': r.ai.max_rows }) }).catch((e) => setMsg({ ok: false, text: String(e.message) })) }, [])
  const set = (k: string, val: string | boolean | number) => setForm((f) => ({ ...f, [k]: val }))
  const prov = v?.providers.find((p) => p.id === form['ai.provider'])
  const save = async () => {
    setBusy(true); setMsg(null)
    try { const r = await rpc<View>({ method: 'settings_set', args: [{ ...form, ...(key ? { 'ai.api_key': key } : {}) }] }); setV(r); setKey(''); setMsg({ ok: true, text: t('settings.saved') }) }
    catch (e) { setMsg({ ok: false, text: (e as Error).message }) } finally { setBusy(false) }
  }
  const test = async () => { setBusy(true); setMsg({ ok: true, text: t('settings.testing') }); try { await save(); const r = await rpc<{ reply: string }>({ method: 'ai_test' }); setMsg({ ok: true, text: `${t('settings.test_ok')}: ${r.reply}` }) } catch (e) { setMsg({ ok: false, text: (e as Error).message }) } finally { setBusy(false) } }
  const detect = async () => { try { setModels(await rpc<string[]>({ method: 'ai_models', args: [{ provider: form['ai.provider'], base_url: form['ai.base_url'], api_key: key }] })) } catch (e) { setMsg({ ok: false, text: (e as Error).message }) } }
  if (!v) return <p className="muted">{t('common.loading')}</p>
  const ro = !admin
  return (
    <div className="set-sec">
      <h2>{t('settings.ai')}</h2><p className="hint">{t('settings.ai_hint')}</p>
      {ro && <div className="err">{t('settings.admin_only')}</div>}
      <div className="row"><label>{t('settings.enable_ai')}</label><input type="checkbox" disabled={ro} checked={!!form['ai.enabled']} onChange={(e) => set('ai.enabled', e.target.checked)} style={{ justifySelf: 'start' }} /></div>
      <div className="row"><label>{t('settings.provider')}</label><select disabled={ro} value={String(form['ai.provider'])} onChange={(e) => { set('ai.provider', e.target.value); setModels([]) }}>{v.providers.map((p) => <option key={p.id} value={p.id}>{p.label}</option>)}</select></div>
      <div className="row"><label>{t('settings.base_url')}</label><input disabled={ro} value={String(form['ai.base_url'])} placeholder={prov?.base_url || 'https://…/v1'} onChange={(e) => set('ai.base_url', e.target.value)} /></div>
      <div className="row"><label>{t('settings.api_key')}</label><div><input disabled={ro} type="password" autoComplete="off" value={key} placeholder={v.ai.has_key ? '••••••••' : prov?.needs_key ? '' : '—'} onChange={(e) => setKey(e.target.value)} style={{ width: '100%' }} /><div className="hint" style={{ margin: '4px 0 0' }}>{v.ai.has_key ? <><Icon name="check" size="var(--icon-md)" /> {t('settings.key_set')}</> : t('settings.key_unset')} · {t('settings.key_write_only')}</div></div></div>
      <div className="row"><label>{t('settings.model')}</label><div style={{ display: 'flex', gap: 6 }}><input disabled={ro} list="ai-models" value={String(form['ai.model'])} onChange={(e) => set('ai.model', e.target.value)} style={{ flex: 1 }} /><datalist id="ai-models">{models.map((m) => <option key={m} value={m} />)}</datalist>{!ro && <button className="btn" onClick={detect}>⟳</button>}</div></div>
      <div className="row"><label>{t('settings.temperature')} ({String(form['ai.temperature'])})</label><input disabled={ro} type="range" min={0} max={1} step={0.1} value={Number(form['ai.temperature'])} onChange={(e) => set('ai.temperature', parseFloat(e.target.value))} /></div>
      <div className="row"><label>{t('settings.max_rows')}</label><input disabled={ro} type="number" min={1} max={200} value={Number(form['ai.max_rows'])} onChange={(e) => set('ai.max_rows', parseInt(e.target.value || '25', 10))} style={{ width: 100 }} /></div>
      <div className="row"><label>{t('settings.system_prompt')}</label><textarea disabled={ro} rows={4} value={String(form['ai.system_prompt'])} onChange={(e) => set('ai.system_prompt', e.target.value)} /></div>
      {msg && <div className={msg.ok ? 'card form' : 'err'} style={{ margin: '10px 0' }}>{msg.text}</div>}
      {!ro && <div style={{ display: 'flex', gap: 8 }}><button className="btn p" disabled={busy} onClick={save}>{t('settings.save')}</button><button className="btn" disabled={busy} onClick={test}>{t('settings.test')}</button></div>}
    </div>
  )
}
