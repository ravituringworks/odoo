'use client'
import Link from 'next/link'
import { useEffect, useState } from 'react'
import { rpc } from '@/lib/rpc'
import { qrSvg } from '@/lib/qr'
import { useT } from '@/lib/i18n'
import { Icon } from '@/components/Icon'

const act = <T,>(model: string, method: string, ids: number[], kw: Record<string, unknown>) => rpc<T>({ method, model, args: [ids, kw] })

/** Printable QR cards: one general code and one per table, each deep-linking to the register's self-order page. */
export default function QrCards() {
  const { t } = useT()
  const [cards, setCards] = useState<{ label: string; url: string }[]>([]); const [err, setErr] = useState(''); const [mode, setMode] = useState('')
  useEffect(() => {
    const config = Number(new URLSearchParams(location.search).get('config')); if (!config) return
    ;(async () => {
      try {
        const link = await act<{ token: string; mode: string }>('pos.config', 'kiosk_link', [config], {}); setMode(link.mode)
        const base = `${location.origin}/order/?config=${config}&token=${link.token}`
        const floors = await act<{ name: string; tables: { id: number; name: string }[] }[]>('pos.config', 'floor_plan', [config], {}).catch(() => [])
        setCards([{ label: t('kiosk.general'), url: base }, { label: t('shop.title'), url: `${location.origin}/shop/?config=${config}&token=${link.token}` }, ...floors.flatMap((f) => f.tables.map((tb) => ({ label: `${f.name} · ${t('pos.table')} ${tb.name}`, url: `${base}&table=${tb.name}` })))])
      } catch (e) { setErr(String((e as Error).message ?? e)) }
    })()
  }, []) // eslint-disable-line react-hooks/exhaustive-deps
  return (
    <div className="pos-reg"><div className="bar noprint"><Link href="/pos/"><Icon name="arrow-left" size="var(--icon-md)" /></Link><h1>{t('kiosk.qr_title')}</h1><span className="grow" /><button className="btn p" onClick={() => window.print()}><Icon name="printer" size="var(--icon-md)" /> {t('pos.print')}</button></div>
      {err && <div className="err">{err}</div>}
      {mode === 'nothing' && <div className="err noprint">{t('kiosk.disabled_hint')}</div>}
      <div className="qr-grid">{cards.map((c) => <div key={c.url} className="card qr-card"><div dangerouslySetInnerHTML={{ __html: qrSvg(c.url, 200) }} /><b>{c.label}</b><small className="muted">{t('kiosk.scan_to_order')}</small><a className="noprint" href={c.url} target="_blank" rel="noreferrer">{t('kiosk.open_link')}</a></div>)}</div></div>
  )
}
