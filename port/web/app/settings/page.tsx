'use client'
import { useEffect, useState } from 'react'
import { rpc } from '@/lib/rpc'
import { useT } from '@/lib/i18n'
import { Appearance } from '@/components/settings/Appearance'
import { LanguageTab } from '@/components/settings/LanguageTab'
import { AiTab } from '@/components/settings/AiTab'
import { EmailTab } from '@/components/settings/EmailTab'
import { TerminalsTab } from '@/components/settings/TerminalsTab'
import { AccountTab, AdminTab } from '@/components/settings/AccountSystem'

const TABS = ['appearance', 'language', 'ai', 'email', 'terminals', 'account', 'administration'] as const
type Tab = (typeof TABS)[number]

export default function SettingsPage() {
  const { t } = useT()
  const [tab, setTab] = useState<Tab>('appearance'); const [uid, setUid] = useState<number | null>(null)
  useEffect(() => { rpc<{ uid: number } | null>({ method: 'whoami' }).then((r) => setUid(r?.uid ?? null)).catch(() => undefined); const h = location.hash.slice(1) as Tab; if (TABS.includes(h)) setTab(h) }, [])
  const pick = (x: Tab) => { setTab(x); history.replaceState(null, '', `#${x}`) }
  return (
    <>
      <div className="bar"><h1>{t('settings.title')}</h1></div>
      <div className="set">
        <nav className="set-tabs">{TABS.map((x) => <button key={x} className={`set-tab ${tab === x ? 'on' : ''}`} onClick={() => pick(x)}>{t(`settings.${x}` as never)}</button>)}</nav>
        <div className="set-body">
          {tab === 'appearance' && <Appearance />}
          {tab === 'language' && <LanguageTab />}
          {tab === 'ai' && <AiTab admin={uid === 1} />}
          {tab === 'email' && <EmailTab admin={uid === 1} />}
          {tab === 'terminals' && <TerminalsTab admin={uid === 1} />}
          {tab === 'account' && <AccountTab uid={uid} />}
          {tab === 'administration' && <AdminTab />}
        </div>
      </div>
    </>
  )
}
