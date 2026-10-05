'use client'
import { useEffect, useState } from 'react'
import { rpc } from '@/lib/rpc'
import { useT } from '@/lib/i18n'
import { Icon } from '@/components/Icon'

export type DisplayState = { status: 'idle' | 'cart' | 'paying' | 'done'; name?: string; lines: { name: string; qty: number; price: number; total: number }[]; tax: number; total: number; change?: number; currency: string; partner?: string; message?: string }
const EMPTY: DisplayState = { status: 'idle', lines: [], tax: 0, total: 0, currency: '$' }

/** Customer-facing display. Same browser: live over BroadcastChannel. Another device: pass `&token=…` and it polls the register. */
export default function Display() {
  const { t } = useT()
  const [s, setS] = useState<DisplayState>(EMPTY)
  useEffect(() => {
    const q = new URLSearchParams(location.search); const config = Number(q.get('config')); const token = q.get('token')
    let ch: BroadcastChannel | null = null
    try { ch = new BroadcastChannel(`pos-display-${config}`); ch.onmessage = (e) => setS(e.data as DisplayState) } catch { /* no BroadcastChannel */ }
    let iv: ReturnType<typeof setInterval> | null = null
    if (token) iv = setInterval(() => rpc<DisplayState | null>({ method: 'kiosk_display', args: [config, { access_token: token }], token: null }).then((r) => r && setS(r)).catch(() => {}), 1000)
    return () => { ch?.close(); if (iv) clearInterval(iv) }
  }, [])
  const m = (n: number) => `${s.currency}${n.toFixed(2)}`
  if (s.status === 'idle') return <Idle s={s} t={t} />
  if (s.status === 'done') return <div className="cd cd-idle"><h1>{t('pos.cd_thanks')}</h1>{!!s.change && s.change > 0 && <p>{t('pos.change')}: <b>{m(s.change)}</b></p>}</div>
  return (
    <div className="cd"><h2>{s.name}</h2>{s.partner && <p className="muted">{s.partner}</p>}
      <div className="cd-lines">{s.lines.map((l, i) => <div key={i} className="cd-line"><span>{l.qty} × {l.name}</span><b>{m(l.total)}</b></div>)}</div>
      <div className="cd-total"><span>{t('pos.taxes')} {m(s.tax)}</span><span>{t('pos.total')}</span><b>{m(s.total)}</b></div>{s.status === 'paying' && <p className="muted">{t('pos.cd_paying')}</p>}</div>
  )
}

const KEY = 'pos_display_images'
const load = (): string[] => { try { return JSON.parse(localStorage.getItem(KEY) ?? '[]') as string[] } catch { return [] } }
/** Idle screen: slideshow of ad images kept on *this* display device (localStorage), welcome text over them. */
function Idle({ s, t }: { s: DisplayState; t: ReturnType<typeof useT>['t'] }) {
  const [imgs, setImgs] = useState<string[]>([]); const [i, setI] = useState(0); const [open, setOpen] = useState(false)
  useEffect(() => setImgs(load()), [])
  useEffect(() => { if (imgs.length < 2) return; const h = setInterval(() => setI((x) => (x + 1) % imgs.length), 8000); return () => clearInterval(h) }, [imgs.length])
  const add = async (files: FileList | null) => {
    if (!files) return
    const read = (f: File) => new Promise<string>((res) => { const r = new FileReader(); r.onload = () => res(String(r.result)); r.readAsDataURL(f) })
    const next = [...imgs, ...(await Promise.all([...files].filter((f) => f.type.startsWith('image/') && f.size < 1_500_000).map(read)))].slice(0, 8)
    setImgs(next); try { localStorage.setItem(KEY, JSON.stringify(next)) } catch { /* storage full */ }
  }
  const clear = () => { setImgs([]); try { localStorage.removeItem(KEY) } catch { /* ignore */ } }
  return (
    <div className="cd cd-idle" style={imgs[i] ? { backgroundImage: `linear-gradient(#0008,#0008), url(${imgs[i]})`, backgroundSize: 'cover', backgroundPosition: 'center', color: '#fff' } : undefined}>
      <h1>{s.name ?? t('app.name')}</h1><p>{s.message ?? t('pos.cd_welcome')}</p>
      <button className="cd-gear" onClick={() => setOpen((o) => !o)}><Icon name="gear" size="var(--icon-md)" /></button>
      {open && <div className="card cd-panel"><b>{t('pos.slideshow')} ({imgs.length}/8)</b><label className="btn">{t('pos.add_images')}<input type="file" accept="image/*" multiple hidden onChange={(e) => add(e.target.files)} /></label><button className="btn" onClick={clear}>{t('pos.clear')}</button></div>}
    </div>
  )
}
