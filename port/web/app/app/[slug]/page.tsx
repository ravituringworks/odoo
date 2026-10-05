import { AppLanding } from '@/components/Landing'
import { APPS } from '@/lib/apps'
export const dynamicParams = false
export function generateStaticParams() { return APPS.map((a) => ({ slug: a.slug })) }
export default async function Page({ params }: { params: Promise<{ slug: string }> }) { return <AppLanding slug={(await params).slug} /> }
