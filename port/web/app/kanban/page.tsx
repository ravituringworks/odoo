'use client'
import { Suspense } from 'react'
import { useSearchParams } from 'next/navigation'
import { Kanban } from '@/components/Kanban'

function Inner() {
  const p = useSearchParams()
  const model = p.get('model'); if (!model) return <p className="muted">No model.</p>
  let domain: unknown[] = []; try { domain = JSON.parse(p.get('domain') ?? '[]') } catch {}
  return <Kanban model={model} title={p.get('title') ?? model} domain={domain} />
}
export default function Page() { return <Suspense><Inner /></Suspense> }
