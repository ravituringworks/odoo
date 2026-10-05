import './vibe-tokens.css'
import './globals.css'
import { Shell } from '@/components/Shell'
import { I18nProvider } from '@/lib/i18n'

const BOOT = `try{var d=document.documentElement,c=localStorage.getItem('odoo-rs-theme-css'),m=localStorage.getItem('odoo-rs-theme');if(!m){m=matchMedia('(prefers-color-scheme: light)').matches?'light':'dark'}d.setAttribute('data-theme',m);if(c)d.style.cssText=c;var l=localStorage.getItem('odoo-rs-lang')||(navigator.language||'en').split('-')[0];d.lang=l;d.dir=['ar','he','fa','ur'].indexOf(l)>=0?'rtl':'ltr'}catch(e){}`

export const metadata = { title: 'Odoo RS', description: 'Functional Rust/Tauri/Next.js port of Odoo' }

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        {/* paint the saved theme + language before hydration so there is no flash */}
        <script dangerouslySetInnerHTML={{ __html: BOOT }} />
      </head>
      <body><I18nProvider><Shell>{children}</Shell></I18nProvider></body>
    </html>
  )
}
