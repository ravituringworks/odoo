import { ICONS, type IconName } from '@/lib/icons'

/** Thin line icon. Weight comes from the --icon-stroke token and size from the --icon-* tokens, so it follows the theme. */
export function Icon({ name, size, className, title }: { name: IconName; size?: number | string; className?: string; title?: string }) {
  const s = size ?? 'var(--icon-xl, 24px)'
  return (
    <svg className={`vibe-icon ${className ?? ''}`} width={s} height={s} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="var(--icon-stroke, 1.5)" strokeLinecap="round" strokeLinejoin="round" role={title ? 'img' : undefined} aria-label={title} aria-hidden={title ? undefined : true} focusable="false" style={{ flexShrink: 0, verticalAlign: 'middle' }}>
      {ICONS[name].map((d, i) => <path key={i} d={d} />)}
    </svg>
  )
}
