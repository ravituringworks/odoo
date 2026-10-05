# Port status ledger (honest accounting)

Generated from `tools/extract_schema.py` / `tools/extract_data.py` plus the test suite. **This port is not 100% complete**;
this file states exactly what is and is not covered so completion can be tracked.

## Source surface (Odoo 18, this repo)
- Modules: **655**, model declarations: **3557**, fields: **14698**, methods: **20712**, Python lines: **494190**
- Extracted to declarative JSON (`schema/`, `data/`): 100% of model/field declarations, menus, window actions, ACL rows, seed records.

## Ported and tested
| Layer | Status |
|---|---|
| Model registry (`_inherit`, `_inherits`, field overlay, mixins) | Done, all 655 modules loadable |
| DDL generation | Done for SQLite, PostgreSQL, MySQL, DuckDB, orbit-rs (SQL-generation tested for all five) |
| ORM: create/read/write/unlink/search/search_count/read_group/name_search, x2many commands, related + delegated fields, cascade/restrict/set-null, active filter, defaults | Done; identical flow suites pass on SQLite and DuckDB (DuckDB found a bug SQLite masked) |
| Domains (prefix notation, dotted paths, x2many, `any` NOT supported) | Done except `child_of`/`parent_of`/`any` |
| Business flows | `base`: ORM, partner, seed (countries/currencies/langs partial); `account`: invoices/bills, posting (balanced double entry), payments (simplified); `sale`: quotation, confirm, invoice, taxes, totals; `sale_stock`: delivery picking on confirm, qty_delivered; `stock`: pickings, moves, quants, validate; `purchase`: RFQ, confirm, bills, receipts; `purchase_stock`: receipt picking, qty_received; `crm`: stages, won/lost |
| Seed data loader (xmlid updates, savepoint-isolated records, ir.model.data persisted), menu/action catalog | Done: ~1.6k records load, ~20 fail (res.lang `code`, a few users), ~980 skipped by design (views, cron, mail templates...) |
| Auth: salted iterated SHA-256 passwords, sessions, login UI, group-based ACL (ir.model.access), record rules (ir.rule subset, fail-closed) | Done; unit + manual HTTP verification (non-admin denied account.move/ir.rule); no password reset, 2FA, API keys, OAuth |
| Generic BaseModel methods for every model: `copy` (honours `copy=` flags, m2m links, opt-in o2m), `action_archive`/`action_unarchive`/`toggle_active`, `exists`, `name_create` | Done + tested; Duplicate/Archive buttons in the UI |
| Search views: filter chips from the real `<search>` arch (OR within group, AND across groups; filters needing Python evaluation are dropped) | Done in list view; group-by and custom filters not yet |
| Stored `related` fields (193 in Odoo 18) filled generically by following the path; `res.partner.complete_name` | Done; no dependency-triggered refresh when the *source* record changes later |
| Form-time `default_get` / `onchange` (sale lines) | Done for sale.order.line only |
| HR leave workflow, project tasks, user default groups | Done (basic) |
| PostgreSQL driver (`--features postgres`) | **Verified against PostgreSQL 18**: all flow suites + seeding pass (set `ODOO_PG_URL`; tests skip without it). Uses inlined, escaped literals instead of bound params |
| MySQL driver (`--features mysql`), orbit-rs (via the Postgres driver + `OrbitRs` dialect) | Compile-only: no MySQL server or orbit-rs instance was available to test |
| HTTP transport (axum), Tauri shell, Next.js UI (list, form, kanban, inline lines, workflow buttons) | Done; verified in Chrome: login, app launcher, list (58 seeded contacts), new-order form defaults, order confirm → invoice → post. Typed-input/mouse-click automation was flaky in the harness, so some steps were driven via DOM calls |

## Views (new)
`tools/extract_views.py` resolves `ir.ui.view` inheritance (xpath/field positions, attributes, replace) into one merged arch per model/type:
684 models, 1,494 views, 1,014 extensions applied. The UI renders forms from the arch (header buttons limited to ported actions, statusbar,
groups, notebook tabs, smart-button counters, inline one2many columns, `invisible`/`readonly`/`required` expressions via a safe evaluator) and
list columns from the `<list>` arch.

## Stored computed fields: the main remaining logic debt
1,345 stored computed fields have no ported compute (they get neutral fallbacks, so list columns can be blank/zero). Each needs real logic;
only the sale/purchase/account/stock/crm/hr/project ones listed above and `res.partner.complete_name` are done. The PostgreSQL test file
(`crates/odoo-postgres/tests/flows.rs`) is a snapshot of `odoo-modules/tests/flows.rs` and lacks the newest tests.

## NOT ported (known gaps)
- ~20712 Python methods: only the flows above are ported. Everything else returns `method ... is not ported yet`.
- Computed stored fields without a registered compute use neutral fallbacks (0 / first selection) instead of Odoo's real logic.
- Field-level `groups`, record-rule expressions beyond the textual subset (those fail closed).
- Password reset, 2FA, API keys, OAuth, session expiry.
- XML view architectures (`ir.ui.view`): the UI generates generic views from field metadata instead; no QWeb, reports (PDF), website/portal, mail/chatter, discuss, calendar, onchange, wizards, scheduled actions, i18n, localizations (`l10n_*`), POS, MRP, payment providers, EDI.
- PostgreSQL/MySQL/orbit-rs drivers are unverified against real servers (none available here).
- Accounting is simplified: no fiscal positions, tax repartition, reconciliation, multi-currency, analytic.

## Largest app modules by source size (remaining port backlog)
| module | models | methods | py lines |
|---|---|---|---|
| account | 84 | 1580 | 38762 |
| mail | 76 | 896 | 24479 |
| stock | 66 | 843 | 16679 |
| mrp | 48 | 627 | 11447 |
| website | 50 | 410 | 10469 |
| point_of_sale | 68 | 523 | 8872 |
| website_sale | 31 | 207 | 7505 |
| hr_holidays | 25 | 267 | 6080 |
| project | 22 | 324 | 5809 |
| website_slides | 22 | 167 | 5020 |
| survey | 10 | 179 | 4888 |
| crm | 22 | 213 | 4626 |
| mass_mailing | 30 | 195 | 4427 |
| purchase | 19 | 180 | 3707 |
| calendar | 17 | 180 | 3524 |
| hr_expense | 24 | 182 | 3152 |
| im_livechat | 17 | 93 | 2739 |
| hr | 25 | 156 | 2456 |
| hr_recruitment | 24 | 131 | 2380 |
| hr_attendance | 9 | 66 | 1761 |
| repair | 16 | 118 | 1736 |
| lunch | 12 | 66 | 1429 |
| website_event | 10 | 52 | 1398 |
| fleet | 16 | 53 | 1235 |
| mass_mailing_sms | 11 | 57 | 1184 |

## Bugs found by running on more than one engine / in the browser (all fixed, regression-tested)
DuckDB: invalid many2one list values; one2many with delegated inverse. PostgreSQL: `DEFAULT FALSE` on integer columns, date types,
models sharing a physical table, custom `_table` names, lock-table limits on one huge DDL transaction. Browser: wrong ambient defaults,
`_id` labels, computed fields editable, unresolved selection constants / `selection_add`, related fields not searchable.
