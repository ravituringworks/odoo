'use client'
import Link from 'next/link'
import { useState } from 'react'
import { APPS, appBySlug, type AppInfo } from '@/lib/apps'
import { categoryProfile, industriesByCategory, industriesUsing, industryApps, industryBySlug, searchIndustries, trialApps } from '@/lib/industries'

function Head() {
  return (
    <header className="tr-head">
      <Link href="/trial/" className="tr-logo">Odoo RS</Link>
      <Link href="/industries/">Industries</Link><Link href="/app/">Apps</Link><span className="grow" />
      <Link href="/my/">Sign in</Link><Link href="/trial/" className="btn p">Try it free</Link>
    </header>
  )
}
const Features = ({ items }: { items: string[] }) => <ul className="ld-list">{items.map((f) => <li key={f}>{f}</li>)}</ul>
function AppCard({ a }: { a: AppInfo }) {
  return (
    <Link href={`/app/${a.slug}/`} className="ld-card">
      <b>{a.name}</b>{!a.community && <span className="tr-ent">Enterprise</span>}
      <span className="muted">{a.tagline}</span>
      <Features items={a.features.slice(0, 2)} />
    </Link>
  )
}

export function IndustriesIndex() {
  const [q, setQ] = useState('')
  const hits = new Set(searchIndustries(q).map((i) => i.slug))
  return (
    <div className="tr">
      <Head />
      <section className="tr-hero"><h1>Software for <span className="tr-under">your industry</span></h1><p>Pick yours: we start your trial with the right apps already installed.</p>
        <input className="ld-search" type="search" placeholder="Search industries…" value={q} onChange={(e) => setQ(e.target.value)} aria-label="Search industries" /></section>
      <main className="tr-body">
        {industriesByCategory().map(([cat, list]) => { const shown = list.filter((i) => hits.has(i.slug)); return shown.length ? (
          <section key={cat}><h2 className="tr-cat">{cat}</h2>
            <div className="tr-grid">{shown.map((i) => <Link key={i.slug} href={`/industries/${i.slug}/`} className="ld-card"><b>{i.name}</b><span className="muted">{i.audience}</span></Link>)}</div></section>) : null })}
        {hits.size === 0 && <p className="muted">No industry matches “{q}”.</p>}
      </main>
    </div>
  )
}

export function IndustryLanding({ slug }: { slug: string }) {
  const ind = industryBySlug(slug)
  if (!ind) return <div className="tr"><Head /><main className="tr-body"><p>Unknown industry. <Link href="/industries/">Browse all industries</Link></p></main></div>
  const prof = categoryProfile(ind.category)
  const apps = industryApps(ind), installable = trialApps(ind)
  const related = industriesByCategory().find(([c]) => c === ind.category)?.[1].filter((i) => i.slug !== ind.slug) ?? []
  return (
    <div className="tr">
      <Head />
      <section className="tr-hero">
        <p className="muted">{ind.category}</p>
        <h1>Odoo for <span className="tr-under">{ind.name}</span></h1>
        <p>{ind.audience}. {prof?.summary}</p>
        <p><Link className="btn p" href={`/trial/?industry=${ind.slug}`}>Start free with {installable.length} apps</Link></p>
      </section>
      <main className="tr-body">
        <h2 className="tr-cat">Key attributes</h2>
        <dl className="ld-attrs">
          <div><dt>Industry</dt><dd>{ind.name}</dd></div><div><dt>Segment</dt><dd>{ind.category}</dd></div>
          <div><dt>Apps included</dt><dd>{apps.length} ({installable.length} in the free trial)</dd></div><div><dt>Built for</dt><dd>{ind.audience.replace(/^For /, '')}</dd></div>
        </dl>
        {prof && <><h2 className="tr-cat">How your business runs</h2><ol className="ld-steps">{prof.workflow.map((s) => <li key={s}>{s}</li>)}</ol>
          <h2 className="tr-cat">Why it fits</h2><Features items={prof.strengths} /></>}
        <h2 className="tr-cat">Apps for {ind.name}</h2>
        <div className="tr-grid">{apps.map((a) => <AppCard key={a.slug} a={a} />)}</div>
        {related.length > 0 && <><h2 className="tr-cat">Related industries</h2><div className="ld-chips">{related.map((i) => <Link key={i.slug} href={`/industries/${i.slug}/`}>{i.name}</Link>)}</div></>}
        <p style={{ marginTop: 30 }}><Link className="btn p" href={`/trial/?industry=${ind.slug}`}>Start your free trial</Link></p>
      </main>
    </div>
  )
}

export function AppsIndex() {
  const cats = [...new Set(APPS.map((a) => a.category))]
  return (
    <div className="tr"><Head />
      <section className="tr-hero"><h1>Every app, <span className="tr-under">one platform</span></h1><p>Install only what you need, add more as you grow.</p></section>
      <main className="tr-body">{cats.map((c) => <section key={c}><h2 className="tr-cat">{c}</h2><div className="tr-grid">{APPS.filter((a) => a.category === c).map((a) => <AppCard key={a.slug} a={a} />)}</div></section>)}</main>
    </div>
  )
}

export function AppLanding({ slug }: { slug: string }) {
  const a = appBySlug(slug)
  if (!a) return <div className="tr"><Head /><main className="tr-body"><p>Unknown app. <Link href="/app/">Browse all apps</Link></p></main></div>
  const used = industriesUsing(a.name)
  return (
    <div className="tr"><Head />
      <section className="tr-hero">
        <p className="muted">{a.category}</p><h1><span className="tr-under">{a.name}</span></h1><p>{a.tagline}</p>
        <p>{a.community ? <Link className="btn p" href={`/trial/?app=${a.slug}`}>Try {a.name} free</Link> : <span className="tr-ent">Enterprise edition only</span>}</p>
      </section>
      <main className="tr-body">
        <h2 className="tr-cat">Features</h2><Features items={a.features} />
        <h2 className="tr-cat">Attributes</h2>
        <dl className="ld-attrs"><div><dt>Edition</dt><dd>{a.community ? 'Community (free trial)' : 'Enterprise'}</dd></div>
          {a.attributes.map(([k, v]) => <div key={k}><dt>{k}</dt><dd>{v}</dd></div>)}<div><dt>Used by</dt><dd>{used.length} industries</dd></div></dl>
        {used.length > 0 && <><h2 className="tr-cat">Popular in</h2><div className="ld-chips">{used.slice(0, 24).map((i) => <Link key={i.slug} href={`/industries/${i.slug}/`}>{i.name}</Link>)}</div></>}
      </main>
    </div>
  )
}
