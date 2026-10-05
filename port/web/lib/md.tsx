// Tiny safe markdown renderer for assistant replies: paragraphs, bullet lists, tables, **bold**, `code`, fenced code.
// Builds React elements only (never injects HTML), so model output cannot inject markup or scripts.
import type { ReactNode } from 'react'

const inline = (s: string, k: string): ReactNode[] =>
  s.split(/(\*\*[^*]+\*\*|`[^`]+`)/g).filter(Boolean).map((p, i) =>
    p.startsWith('**') && p.endsWith('**') ? <b key={`${k}${i}`}>{p.slice(2, -2)}</b> : p.startsWith('`') && p.endsWith('`') ? <code key={`${k}${i}`} style={{ background: 'var(--bg-tertiary)', padding: '0 4px', borderRadius: 4 }}>{p.slice(1, -1)}</code> : p)

export function Markdown({ text }: { text: string }) {
  const lines = text.replace(/\r/g, '').split('\n'); const out: ReactNode[] = []; let i = 0
  while (i < lines.length) {
    const l = lines[i]
    if (l.startsWith('```')) { const code: string[] = []; i++; while (i < lines.length && !lines[i].startsWith('```')) code.push(lines[i++]); i++; out.push(<pre key={i} style={{ background: 'var(--bg-tertiary)', padding: 8, borderRadius: 6, overflow: 'auto', margin: '6px 0' }}>{code.join('\n')}</pre>); continue }
    if (/^\s*[-*] /.test(l)) { const items: string[] = []; while (i < lines.length && /^\s*[-*] /.test(lines[i])) items.push(lines[i++].replace(/^\s*[-*] /, '')); out.push(<ul key={i} style={{ margin: '4px 0', paddingInlineStart: 20 }}>{items.map((x, j) => <li key={j}>{inline(x, `li${i}${j}`)}</li>)}</ul>); continue }
    if (l.includes('|') && /^\s*\|?.+\|.+/.test(l) && lines[i + 1] && /^\s*\|?[\s:-]+\|[\s|:-]*$/.test(lines[i + 1])) {
      const row = (r: string) => r.trim().replace(/^\||\|$/g, '').split('|').map((c) => c.trim()); const head = row(l); i += 2; const body: string[][] = []
      while (i < lines.length && lines[i].includes('|')) body.push(row(lines[i++]))
      out.push(<div key={i} style={{ overflow: 'auto' }}><table style={{ fontSize: 12 }}><thead><tr>{head.map((h, j) => <th key={j}>{h}</th>)}</tr></thead><tbody>{body.map((r, j) => <tr key={j} style={{ cursor: 'default' }}>{r.map((c, k) => <td key={k}>{inline(c, `c${j}${k}`)}</td>)}</tr>)}</tbody></table></div>); continue
    }
    if (l.trim() === '') { i++; continue }
    const h = /^(#{1,3})\s+(.*)$/.exec(l)
    out.push(h ? <div key={i} style={{ fontWeight: 600, margin: '8px 0 2px' }}>{inline(h[2], `h${i}`)}</div> : <p key={i} style={{ margin: '4px 0' }}>{inline(l, `p${i}`)}</p>); i++
  }
  return <>{out}</>
}
