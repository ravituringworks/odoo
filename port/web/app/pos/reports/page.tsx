'use client'
import Link from 'next/link'
import { useEffect, useState } from 'react'
import { rpc } from '@/lib/rpc'
import { useT } from '@/lib/i18n'

type Report = { days: number; orders: number; total: number; average: number; registers: { id: number; name: string; total: number; orders: number }[]; top_products: { name: string; qty: number; total: number }[]; payments: { name: string; amount: number }[]; by_hour: number[]; by_day: { day: string; amount: number }[] }

/** Sales across all registers (multi-store view): totals, per register, best sellers, payment mix, busiest hours. */
export default function Reports() {
  const { t } = useT()
  const [days, setDays] = useState(7); const [r, setR] = useState<Report | null>(null); const [err, setErr] = useState('')
  useEffect(() => { setR(null); rpc<Report>({ method: 'sales_report', model: 'pos.config', args: [[], { days }] }).then(setR).catch((e) => setErr(String(e.message ?? e))) }, [days])
  const money = (n: number) => n.toLocaleString(undefined, { style: 'decimal', minimumFractionDigits: 2, maximumFractionDigits: 2 })
  const max = (xs: number[]) => Math.max(1, ...xs)
  const Bars = ({ data, label }: { data: number[]; label: (i: number) => string }) => <div className="rp-bars">{data.map((v, i) => <div key={i} className="rp-bar" title={`${label(i)}: ${money(v)}`}><i style={{ height: `${(v / max(data)) * 100}%` }} /><small>{label(i)}</small></div>)}</div>
  return (
    <div className="pos-reg"><div className="bar"><Link href="/pos/">←</Link><h1>{t('pos.reports')}</h1><span className="grow" />
      {[1, 7, 30, 90].map((d) => <button key={d} className={`btn ${d === days ? 'p' : ''}`} onClick={() => setDays(d)}>{d}d</button>)}</div>
      {err && <div className="err">{err}</div>}{!r && !err && <p className="muted">{t('common.loading')}</p>}
      {r && <>
        <div className="rp-kpis"><div className="card"><small className="muted">{t('pos.total')}</small><b>{money(r.total)}</b></div><div className="card"><small className="muted">{t('pos.orders')}</small><b>{r.orders}</b></div><div className="card"><small className="muted">{t('pos.avg_order')}</small><b>{money(r.average)}</b></div></div>
        <div className="rp-cols">
          <div className="card"><h3>{t('pos.by_register')}</h3>{r.registers.map((x) => <div key={x.id} className="pos-rl"><span>{x.name} <small className="muted">({x.orders})</small></span><b>{money(x.total)}</b></div>)}</div>
          <div className="card"><h3>{t('pos.payment_mix')}</h3>{r.payments.map((x) => <div key={x.name} className="pos-rl"><span>{x.name}</span><b>{money(x.amount)}</b></div>)}</div>
          <div className="card"><h3>{t('pos.top_products')}</h3>{r.top_products.map((x) => <div key={x.name} className="pos-rl"><span>{x.qty} × {x.name}</span><b>{money(x.total)}</b></div>)}</div>
          <div className="card"><h3>{t('pos.busy_hours')}</h3><Bars data={r.by_hour} label={(i) => String(i)} /></div>
          {r.by_day.length > 1 && <div className="card"><h3>{t('pos.by_day')}</h3><Bars data={r.by_day.map((x) => x.amount)} label={(i) => r.by_day[i].day.slice(5)} /></div>}
        </div></>}
    </div>
  )
}
