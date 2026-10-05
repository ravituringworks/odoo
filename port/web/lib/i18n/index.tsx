'use client'
import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react'
import { en, type Key } from './en'
import { es } from './es'; import { fr } from './fr'; import { de } from './de'; import { pt } from './pt'; import { it } from './it'
import { hi } from './hi'; import { ar } from './ar'; import { zh } from './zh'; import { ja } from './ja'; import { ru } from './ru'

export type Locale = { code: string; name: string; native: string; dir: 'ltr' | 'rtl'; odoo: string }
// `odoo` is the Odoo language code used to pick the server-side translation catalog (field labels, menus, views)
export const LOCALES: Locale[] = [
  { code: 'en', name: 'English', native: 'English', dir: 'ltr', odoo: 'en_US' },
  { code: 'es', name: 'Spanish', native: 'Español', dir: 'ltr', odoo: 'es' },
  { code: 'fr', name: 'French', native: 'Français', dir: 'ltr', odoo: 'fr' },
  { code: 'de', name: 'German', native: 'Deutsch', dir: 'ltr', odoo: 'de' },
  { code: 'pt', name: 'Portuguese', native: 'Português', dir: 'ltr', odoo: 'pt_BR' },
  { code: 'it', name: 'Italian', native: 'Italiano', dir: 'ltr', odoo: 'it' },
  { code: 'ru', name: 'Russian', native: 'Русский', dir: 'ltr', odoo: 'ru' },
  { code: 'hi', name: 'Hindi', native: 'हिन्दी', dir: 'ltr', odoo: 'hi' },
  { code: 'ar', name: 'Arabic', native: 'العربية', dir: 'rtl', odoo: 'ar' },
  { code: 'zh', name: 'Chinese', native: '简体中文', dir: 'ltr', odoo: 'zh_CN' },
  { code: 'ja', name: 'Japanese', native: '日本語', dir: 'ltr', odoo: 'ja' },
]
const CATALOGS: Record<string, Partial<Record<Key, string>>> = { en, es, fr, de, pt, it, hi, ar, zh, ja, ru }
const KEY = 'odoo-rs-lang'

export const getLang = (): string => { try { return localStorage.getItem(KEY) || (navigator.language || 'en').split('-')[0] } catch { return 'en' } }
export const odooLang = (code = getLang()): string => LOCALES.find((l) => l.code === code)?.odoo ?? 'en_US'

/** Pure translate: locale catalog -> English -> key; `{name}` placeholders are interpolated. */
export const translate = (lang: string, key: Key, vars?: Record<string, string | number>): string => {
  const raw = CATALOGS[lang]?.[key] ?? en[key] ?? key
  return vars ? raw.replace(/\{(\w+)\}/g, (_, k) => String(vars[k] ?? `{${k}}`)) : raw
}

type Ctx = { lang: string; locale: Locale; setLang: (c: string) => void; t: (k: Key, v?: Record<string, string | number>) => string }
const I18n = createContext<Ctx>({ lang: 'en', locale: LOCALES[0], setLang: () => {}, t: (k) => en[k] })

export function I18nProvider({ children }: { children: React.ReactNode }) {
  const [lang, setLangState] = useState('en')
  useEffect(() => { const l = getLang(); setLangState(CATALOGS[l] ? l : 'en') }, [])
  const locale = LOCALES.find((l) => l.code === lang) ?? LOCALES[0]
  useEffect(() => { document.documentElement.lang = lang; document.documentElement.dir = locale.dir }, [lang, locale.dir])
  const setLang = useCallback((c: string) => { try { localStorage.setItem(KEY, c) } catch { /* ignore */ } location.reload() }, [])  // reload: server-translated labels/menus are cached per language
  const t = useCallback((k: Key, v?: Record<string, string | number>) => translate(lang, k, v), [lang])
  const value = useMemo(() => ({ lang, locale, setLang, t }), [lang, locale, setLang, t])
  return <I18n.Provider value={value}>{children}</I18n.Provider>
}
export const useT = () => useContext(I18n)
