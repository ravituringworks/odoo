'use client'
import { useEffect, useState } from 'react'
import { call, rpc } from '@/lib/rpc'
import { useT } from '@/lib/i18n'
import { Icon } from '@/components/Icon'

type Pm = { id: number; name: string; use_payment_terminal: string }
type View = { pos_terminals: { stripe_has_key: boolean; stripe_base_url: string; adyen_base_url: string } }

/** Card-terminal credentials. Adyen keys and terminal ids live on each POS payment method; Stripe's secret key is account-wide. */
export function TerminalsTab({ admin }: { admin: boolean }) {
  const { t } = useT()
  const [v, setV] = useState<View | null>(null); const [key, setKey] = useState(''); const [stripeUrl, setStripeUrl] = useState(''); const [adyenUrl, setAdyenUrl] = useState(''); const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null)
  const [methods, setMethods] = useState<Pm[]>([]); const [tests, setTests] = useState<Record<number, { ok: boolean; message: string }>>({}); const [pine, setPine] = useState('')
  useEffect(() => { rpc<View>({ method: 'settings_get' }).then((r) => { setV(r); setStripeUrl(r.pos_terminals.stripe_base_url); setAdyenUrl(r.pos_terminals.adyen_base_url); setPine((r.pos_terminals as { pine_labs_base_url?: string }).pine_labs_base_url ?? '') }).catch((e) => setMsg({ ok: false, text: String(e.message ?? e) })) }, [])
  useEffect(() => { call<Pm[]>('pos.payment.method', 'search_read', [[['use_payment_terminal', '!=', false]]], { fields: ['name', 'use_payment_terminal'] }).then(setMethods).catch(() => setMethods([])) }, [])
  const test = async (id: number) => { setTests((x) => ({ ...x, [id]: { ok: true, message: '…' } })); try { const r = await rpc<{ ok: boolean; message: string }>({ method: 'terminal_test', args: [{ method_id: id }] }); setTests((x) => ({ ...x, [id]: { ok: !!r.ok, message: String(r.message) } })) } catch (e) { setTests((x) => ({ ...x, [id]: { ok: false, message: (e as Error).message } })) } }
  const save = async () => {
    setMsg(null)
    try { const r = await rpc<View>({ method: 'settings_set', args: [{ 'pos.stripe_base_url': stripeUrl, 'pos.adyen_base_url': adyenUrl, 'pos.pine_labs_base_url': pine, ...(key ? { 'pos.stripe_secret_key': key } : {}) }] }); setV(r); setKey(''); setMsg({ ok: true, text: t('settings.saved') }) }
    catch (e) { setMsg({ ok: false, text: (e as Error).message }) }
  }
  if (!v) return <p className="muted">{t('common.loading')}</p>
  const ro = !admin
  return (
    <div className="set-sec"><h2>{t('settings.terminals')}</h2><p className="hint">{t('settings.terminals_hint')}</p>
      {ro && <div className="err">{t('settings.admin_only')}</div>}
      <div className="row"><label>Stripe {t('settings.api_key')}</label><input disabled={ro} type="password" autoComplete="off" value={key} placeholder={v.pos_terminals.stripe_has_key ? '••••••••' : 'sk_live_…'} onChange={(e) => setKey(e.target.value)} /></div>
      <div className="row"><label>Stripe {t('settings.base_url')}</label><input disabled={ro} value={stripeUrl} placeholder="https://api.stripe.com" onChange={(e) => setStripeUrl(e.target.value)} /></div>
      <div className="row"><label>Adyen {t('settings.base_url')}</label><input disabled={ro} value={adyenUrl} placeholder="https://terminal-api-test.adyen.com" onChange={(e) => setAdyenUrl(e.target.value)} /></div>
      <div className="row"><label>Pine Labs {t('settings.base_url')}</label><input disabled={ro} value={pine} placeholder="https://… (proxy, outside India/Malaysia)" onChange={(e) => setPine(e.target.value)} /></div>
      {msg && <div className={msg.ok ? 'card form' : 'err'} style={{ margin: '10px 0' }}>{msg.text}</div>}
      {!ro && <button className="btn p" onClick={save}>{t('settings.save')}</button>}
      {methods.length > 0 && <><h3 style={{ marginTop: 18 }}>{t('pos.payment_methods')}</h3>{methods.map((m) => <div key={m.id} className="row"><label>{m.name} <small className="muted">({m.use_payment_terminal})</small></label><div style={{ display: 'flex', gap: 8, alignItems: 'center' }}><button className="btn" onClick={() => test(m.id)}>{t('pos.test_connection')}</button><small>{tests[m.id] && <><Icon name={tests[m.id].ok ? 'check' : 'x'} size="var(--icon-sm)" /> {tests[m.id].message}</>}</small></div></div>)}</>}
    </div>
  )
}
