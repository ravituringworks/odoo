// Pure view-model helpers (no React, no I/O): field metadata -> columns, form layout, value coercion.
export type FieldMeta = {
  type: string; string: string; required: boolean; readonly: boolean; relation: string | null; relation_field: string | null
  selection: [string, string][]; store: boolean; searchable?: boolean
}
export type Fields = Record<string, FieldMeta>
export type Rec = Record<string, unknown> & { id?: number }
export type Domain = unknown[]

const TECHNICAL = new Set(['id', 'create_uid', 'create_date', 'write_uid', 'write_date', 'message_ids', 'message_follower_ids', 'message_partner_ids', 'activity_ids', 'website_message_ids', 'message_main_attachment_id', 'access_token', 'display_name'])
const isTech = (n: string) => TECHNICAL.has(n) || n.startsWith('message_') || n.startsWith('activity_') || n.endsWith('_ids') && false

const SCALAR = new Set(['char', 'text', 'integer', 'float', 'monetary', 'boolean', 'date', 'datetime', 'selection', 'many2one', 'html'])

export const listColumns = (fields: Fields, max = 7): string[] => {
  const pref = ['name', 'display_name', 'partner_id', 'date_order', 'create_date', 'state', 'amount_total', 'email', 'phone', 'stage_id', 'user_id']
  const ok = (n: string) => fields[n]?.store && SCALAR.has(fields[n].type) && !isTech(n) && fields[n].type !== 'html' && fields[n].type !== 'text'
  const chosen = pref.filter((n) => n in fields && ok(n))
  const rest = Object.keys(fields).filter((n) => ok(n) && !chosen.includes(n) && fields[n].required)
  const more = Object.keys(fields).filter((n) => ok(n) && !chosen.includes(n) && !rest.includes(n))
  return [...chosen, ...rest, ...more].slice(0, max)
}

export type Section = { title: string; fields: string[] }
const PRIORITY = ['name', 'partner_id', 'date_order', 'state', 'user_id', 'company_id', 'currency_id']
export const formSections = (fields: Fields): { main: string[]; x2many: string[] } => {
  const hidden = (n: string) => /^(campaign|medium|source|utm|website_|access_|signature|signed_|prepayment|authorized|transaction|require_)/.test(n)
  const names = Object.keys(fields).filter((n) => !isTech(n) && !hidden(n))
  const editable = (n: string) => SCALAR.has(fields[n].type) && fields[n].store
  const rank = (n: string) => (PRIORITY.indexOf(n) >= 0 ? PRIORITY.indexOf(n) : 100) - (fields[n].required ? 50 : 0) + (fields[n].readonly ? 20 : 0)
  const main = names.filter(editable).sort((a, b) => rank(a) - rank(b)).slice(0, 36)
  const x2many = names.filter((n) => fields[n].type === 'one2many' && fields[n].relation).slice(0, 4)
  return { main, x2many }
}

export const display = (v: unknown): string => {
  if (v === false || v === null || v === undefined) return ''
  if (Array.isArray(v)) return typeof v[1] === 'string' ? v[1] : `${v.length} records`
  if (typeof v === 'number') return Number.isInteger(v) ? String(v) : v.toFixed(2)
  return String(v)
}

export const m2oId = (v: unknown): number | null => (Array.isArray(v) ? (v[0] as number) : typeof v === 'number' ? v : null)

/** Convert edited UI state into create/write vals (many2one -> id, empty -> false, only changed keys). */
export const toVals = (fields: Fields, draft: Rec, original: Rec): Rec =>
  Object.fromEntries(
    Object.keys(draft)
      .filter((k) => k in fields && !fields[k].readonly && (fields[k].store || fields[k].type === 'many2many') && JSON.stringify(draft[k]) !== JSON.stringify(original[k]))
      .map((k) => [k, fields[k].type === 'many2one' ? m2oId(draft[k]) ?? false : fields[k].type === 'many2many' ? [[6, 0, Array.isArray(draft[k]) ? draft[k] : []]] : draft[k]]),
  )

// Workflow buttons per model: [method, label, visible-when(state)]. Mirrors Odoo's statusbar header buttons.
export const BUTTONS: Record<string, { method: string; label: string; when: (r: Rec) => boolean; primary?: boolean }[]> = {
  'sale.order': [
    { method: 'action_confirm', label: 'Confirm', primary: true, when: (r) => r.state === 'draft' || r.state === 'sent' },
    { method: '_create_invoices', label: 'Create Invoice', primary: true, when: (r) => r.state === 'sale' && r.invoice_status === 'to invoice' },
    { method: 'action_cancel', label: 'Cancel', when: (r) => r.state !== 'cancel' && r.state !== 'done' },
    { method: 'action_draft', label: 'Set to Quotation', when: (r) => r.state === 'cancel' },
  ],
  'purchase.order': [
    { method: 'button_confirm', label: 'Confirm Order', primary: true, when: (r) => r.state === 'draft' || r.state === 'sent' },
    { method: 'action_create_invoice', label: 'Create Bill', primary: true, when: (r) => r.state === 'purchase' },
    { method: 'button_cancel', label: 'Cancel', when: (r) => r.state === 'draft' || r.state === 'sent' },
  ],
  'account.move': [
    { method: 'action_post', label: 'Post', primary: true, when: (r) => r.state === 'draft' },
    { method: 'action_register_payment', label: 'Register Payment', primary: true, when: (r) => r.state === 'posted' && r.payment_state !== 'paid' && String(r.move_type).includes('invoice') },
    { method: 'button_draft', label: 'Reset to Draft', when: (r) => r.state === 'posted' },
    { method: 'button_cancel', label: 'Cancel', when: (r) => r.state === 'draft' },
  ],
  'stock.picking': [
    { method: 'action_confirm', label: 'Mark as To Do', when: (r) => r.state === 'draft' },
    { method: 'button_validate', label: 'Validate', primary: true, when: (r) => r.state !== 'done' && r.state !== 'cancel' },
    { method: 'action_cancel', label: 'Cancel', when: (r) => r.state !== 'done' && r.state !== 'cancel' },
  ],
  'hr.leave': [
    { method: 'action_approve', label: 'Approve', primary: true, when: (r) => r.state === 'confirm' || r.state === 'validate1' },
    { method: 'action_refuse', label: 'Refuse', when: (r) => ['confirm', 'validate1', 'validate'].includes(String(r.state)) },
    { method: 'action_draft', label: 'Reset to Draft', when: (r) => r.state === 'refuse' || r.state === 'cancel' },
  ],
  'project.task': [
    { method: 'action_done', label: 'Mark Done', primary: true, when: (r) => r.state === '01_in_progress' },
    { method: 'action_reopen', label: 'Reopen', when: (r) => r.state === '1_done' || r.state === '1_canceled' },
    { method: 'action_cancel', label: 'Cancel', when: (r) => r.state === '01_in_progress' },
  ],
  'crm.lead': [
    { method: 'action_set_won', label: 'Won', primary: true, when: (r) => r.active !== false },
    { method: 'action_set_lost', label: 'Lost', when: (r) => r.active !== false },
  ],
}
