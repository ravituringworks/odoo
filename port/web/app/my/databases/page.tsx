'use client'
import Link from 'next/link'
import { Suspense, useState } from 'react'
import { useSearchParams } from 'next/navigation'
import { Portal } from '@/components/Portal'
import { OpenButton } from '@/components/OpenButton'
import { useT } from '@/lib/i18n'
import { daysLeftKind, formatDate, portal } from '@/lib/portal'

function Databases() {
  const { t, lang } = useT(); const fresh = useSearchParams().get('new'); const [confirm, setConfirm] = useState<string | null>(null); const [err, setErr] = useState<string>()
  return (
    <Portal title={t('portal.databases')} active="databases">
      {(me, reload) => (
        <>
          {err && <div className="err">{err}</div>}
          {me.databases.length === 0 && <div className="card form"><p className="muted">{t('portal.db_none')}</p></div>}
          <div className="pt-dbs">
            {me.databases.map((d) => {
              const k = daysLeftKind(d.days_left)
              return (
                <div key={d.db} className={`card pt-db ${fresh === d.db ? 'pt-fresh' : ''}`}>
                  <div className="pt-db-main">
                    <b>{d.company || d.db}</b> <span className={`pill ${k === 'ok' ? 'sale' : k === 'warn' ? 'sent' : 'cancel'}`}>{d.days_left > 0 ? t('portal.db_days', { n: d.days_left }) : t('portal.db_expired')}</span>
                    <div className="muted" style={{ fontSize: 12 }}><code>{d.db}</code> · {t('portal.db_created', { date: formatDate(d.created_at, lang) })}</div>
                    <div className="pt-chips">{d.apps.map((a) => <span key={a} className="pill">{a}</span>)}</div>
                  </div>
                  <div className="pt-db-actions">
                    <OpenButton db={d.db} primary />
                    {confirm === d.db
                      ? <span className="pt-confirm">{t('portal.db_confirm')} <button className="btn" onClick={async () => { try { await portal('portal_delete', [d.db]); setConfirm(null); reload() } catch (e) { setErr((e as Error).message) } }}>{t('portal.db_confirm_yes')}</button> <button className="btn" onClick={() => setConfirm(null)}>{t('portal.db_cancel')}</button></span>
                      : <button className="btn" onClick={() => setConfirm(d.db)}>{t('portal.db_delete')}</button>}
                  </div>
                </div>
              )
            })}
          </div>
          <p>{me.databases.length < me.max_databases ? <Link className="btn p" href="/trial/">+ {t('portal.db_new')}</Link> : <span className="muted">{t('portal.db_limit', { n: me.max_databases })}</span>}</p>
        </>
      )}
    </Portal>
  )
}
export default function Page() { return <Suspense><Databases /></Suspense> }
