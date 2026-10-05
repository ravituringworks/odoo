import { IndustryLanding } from '@/components/Landing'
import { INDUSTRIES } from '@/lib/industries'
export const dynamicParams = false
export function generateStaticParams() { return INDUSTRIES.map((i) => ({ slug: i.slug })) }
export default async function Page({ params }: { params: Promise<{ slug: string }> }) { return <IndustryLanding slug={(await params).slug} /> }
