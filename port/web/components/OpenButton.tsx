'use client'
import { useState } from 'react'
import { useT } from '@/lib/i18n'
import { portal } from '@/lib/portal'
import { setDb, setToken } from '@/lib/rpc'

/** Mints a session inside the owner's trial database and goes there (no password involved). */
export function OpenButton({ db, primary }: { db: string; primary?: boolean }) {
  const { t } = useT(); const [busy, setBusy] = useState(false); const [err, setErr] = useState<string>()
  const open = async () => {
    setBusy(true); setErr(undefined)
    try { const r = await portal<{ db: string; token: string }>('portal_open', [db]); setDb(r.db); setToken(r.token); location.href = '/' } catch (e) { setErr((e as Error).message); setBusy(false) }
  }
  return <><button className={`btn ${primary ? 'p' : ''}`} disabled={busy} onClick={open}>{t('portal.db_open')} →</button>{err && <span className="tr-err">{err}</span>}</>
}
