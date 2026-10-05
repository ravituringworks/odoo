'use client'
import { LOCALES, useT } from '@/lib/i18n'

export function LanguageTab() {
  const { t, lang, setLang, locale } = useT()
  return (
    <div className="set-sec">
      <h2>{t('settings.language')}</h2><p className="hint">{t('settings.language_hint')}{locale.dir === 'rtl' ? ` ${t('settings.rtl_note')}` : ''}</p>
      <div className="langs">
        {LOCALES.map((l) => (
          <button key={l.code} className={`theme ${lang === l.code ? 'on' : ''}`} onClick={() => lang !== l.code && setLang(l.code)} lang={l.code} dir={l.dir}>
            <div style={{ fontWeight: 600, fontSize: 15 }}>{l.native}</div><div className="muted" style={{ fontSize: 11 }}>{l.name} · {l.code}{l.dir === 'rtl' ? ' · RTL' : ''}</div>
          </button>
        ))}
      </div>
    </div>
  )
}
