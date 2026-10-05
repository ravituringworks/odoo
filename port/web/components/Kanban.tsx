'use client'
import { call } from '@/lib/rpc'
import { useAsync, useFields } from '@/lib/hooks'
import { display, m2oId, type Domain, type Rec } from '@/lib/model'

// Group by stage_id when present, else state; cards show name + key figure. Pure projection of `search_read`.
export function Kanban({ model, title, domain }: { model: string; title: string; domain: Domain }) {
  const { data: fields } = useFields(model)
  const gb = fields && ('stage_id' in fields ? 'stage_id' : 'state' in fields ? 'state' : null)
  const money = fields && ['expected_revenue', 'amount_total', 'list_price'].find((f) => f in fields)
  const d = useAsync(async () => {
    if (!fields) return undefined
    const nameF = 'name' in fields ? 'name' : 'display_name'
    const recs = await call<Rec[]>(model, 'search_read', [], { domain, fields: [nameF, ...(gb ? [gb] : []), ...(money ? [money] : [])], limit: 500 })
    if (!gb) return { recs: recs.map((r): Rec => ({ ...r, name: r[nameF] })), stages: [[0, title]] as [number, string][] }   // nothing to group by: one flat column
    const stages = fields[gb].type === 'many2one' ? await call<[number, string][]>(fields[gb].relation!, 'name_search', [''], { limit: 50 }) : fields[gb].selection.map(([k, l], i) => [i, l] as [number, string])
    return { recs, stages: stages.sort((a, b) => a[0] - b[0]) }
  }, [model, JSON.stringify(domain), !!fields])
  const key = (r: Rec, st: [number, string]) => !gb || (fields![gb].type === 'many2one' ? m2oId(r[gb]) === st[0] : fields![gb].selection[st[0]]?.[0] === r[gb])
  return (
    <>
      <div className="bar"><h1>{title}</h1><a className="btn p" href={`/form/?model=${model}`}>New</a></div>
      {d.error && <div className="err">{d.error}</div>}
      <div className="board">
        {d.data?.stages.map((st) => {
          const cards = d.data!.recs.filter((r) => key(r, st))
          return (
            <div className="col" key={st[0]}>
              <h3><span>{st[1]}</span><span className="muted">{cards.length}</span></h3>
              {cards.map((r) => <div className="kcard" key={String(r.id)} onClick={() => (location.href = `/form/?model=${model}&id=${r.id}`)}><b>{display(r.name)}</b>{money && <div className="muted">{display(r[money])}</div>}</div>)}
            </div>
          )
        })}
      </div>
    </>
  )
}
