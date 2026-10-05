// Many2one typing rules: free text is never a value. On leaving the field the text must resolve to exactly one record, else it reverts.
export type Opt = [number, string]
/** The record the typed text means: an exact (case-insensitive) name match, else the only suggestion; null = no unambiguous match. */
export function resolveTyped(typed: string, opts: Opt[]): Opt | null {
  const t = typed.trim().toLowerCase()
  if (!t) return null
  return opts.find(([, n]) => n.trim().toLowerCase() === t) ?? (opts.length === 1 ? opts[0] : null)
}
/** Whether a view node restricted to `groups` can be shown: the web client has no debug mode and no multi-website users yet. */
export function groupsVisible(groups: string | undefined): boolean {
  if (!groups) return true
  const parts = groups.split(',').map((g) => g.trim()).filter(Boolean)
  if (parts.some((g) => g.startsWith('!'))) return true
  return !parts.every((g) => g === 'base.group_no_one' || g === 'website.group_multi_website')
}
