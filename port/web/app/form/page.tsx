'use client'
import { Suspense } from 'react'
import { useSearchParams } from 'next/navigation'
import { FormView } from '@/components/FormView'

function Inner() {
  const p = useSearchParams()
  const model = p.get('model'); if (!model) return <p className="muted">No model.</p>
  const id = p.get('id')
  return <FormView key={`${model}:${id}`} model={model} id={id ? Number(id) : undefined} />
}
export default function Page() { return <Suspense><Inner /></Suspense> }
