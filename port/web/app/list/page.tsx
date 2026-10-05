'use client'
import { Suspense } from 'react'
import { useSearchParams } from 'next/navigation'
import { ListView } from '@/components/ListView'

function Inner() {
  const p = useSearchParams()
  const model = p.get('model'); if (!model) return <p className="muted">No model.</p>
  let domain: unknown[] = []; try { domain = JSON.parse(p.get('domain') ?? '[]') } catch {}
  return <ListView model={model} title={p.get('title') ?? model} domain={domain} />
}
export default function Page() { return <Suspense><Inner /></Suspense> }
