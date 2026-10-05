# Architecture

```
tools/*.py ──► schema/*.json, data/*.json   (Odoo source -> declarative metadata, regenerable)
                    │
   odoo-core  ◄─────┘   Registry · Dialect · DDL · Domain · ORM (pure fns over Env) · Store port
      ▲  ▲
      │  └── odoo-sqlite / odoo-duckdb   (Store adapters; add postgres/mysql/orbit-rs here)
   odoo-modules   business rules as `Rules` (hook tables of pure fns): sale, account, stock, purchase, crm
      ▲
   odoo-app       single `dispatch(Request)` service  ── used by both transports:
      ├── odoo-server  (axum, POST /api/rpc)  ◄── web/ (Next.js static export, fetch)
      └── src-tauri    (invoke("rpc"))        ◄── same web/out bundled in the desktop app
```

## `odoo-app` modules
`lib.rs` (dispatch, tenants, install), `security.rs` (sessions with active/allowed companies, ACL, record rules), `trial.rs` + `portal.rs` (signup, customer portal), `admin.rs` (database console: backup/restore/archive/delete, per-account app rules),
`ai.rs` + `ai_guides.rs` (assistant loop, providers, setup guides), `email.rs`, `terminal.rs`/`epos.rs` (POS hardware), `i18n.rs`. Tenants are separate `App`s (one SQLite file each) opened lazily; the main `App` owns the registry in `odoo_trials`.
The `Store` port has an optional `snapshot(dest)` used for backups (SQLite implements it).

## Functional-style decisions
- No inheritance or global registry mutation: `Registry` is built by folding module data; `Rules` is an immutable map of
  function pointers composed with `merge`; ORM operations are free functions of an immutable `Env`.
- Business transitions (`post`, `validate`, `create_invoice`) are plain functions returning `Result`; all effects flow through `Conn`.
- Transactions are scoped closures (`store::run`), rolled back on `Err`.
- UI: reducers for form state, pure view-model helpers (`lib/model.ts`), one transport function (`lib/rpc.ts`).

## Adding a datastore
Implement `Store` + `Conn` (see `odoo-sqlite`, ~80 lines) and pick/define a `Dialect` (types, placeholders, PK, ILIKE, RETURNING).
Run `odoo-core` DDL plan for it: `ddl::install_plan(&registry, dialect)`. The shared flow test-suite should pass unchanged.
