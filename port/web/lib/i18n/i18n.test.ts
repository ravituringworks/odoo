import { translate, LOCALES } from './index'
import { en } from './en'
const eq = (a: unknown, b: unknown, m: string) => { if (a !== b) { console.error('FAIL', m, a, b); process.exit(1) } }
eq(translate('es', 'form.save'), 'Guardar', 'es'); eq(translate('ar', 'form.save'), 'حفظ', 'ar'); eq(translate('ja', 'list.new'), '新規', 'ja')
eq(translate('xx', 'form.save'), en['form.save'], 'unknown locale falls back'); eq(translate('fr', 'app.name'), 'Odoo RS', 'untranslated key falls back to en')
eq(LOCALES.filter((l) => l.dir === 'rtl').map((l) => l.code).join(), 'ar', 'rtl set')
for (const l of LOCALES) for (const k of Object.keys(en) as (keyof typeof en)[]) { if (translate(l.code, k).length === 0) { console.error('empty', l.code, k); process.exit(1) } }
console.log(`i18n: ${LOCALES.length} locales ok`)
