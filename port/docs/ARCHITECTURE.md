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

## Functional-style decisions
- No inheritance or global registry mutation: `Registry` is built by folding module data; `Rules` is an immutable map of
  function pointers composed with `merge`; ORM operations are free functions of an immutable `Env`.
- Business transitions (`post`, `validate`, `create_invoice`) are plain functions returning `Result`; all effects flow through `Conn`.
- Transactions are scoped closures (`store::run`), rolled back on `Err`.
- UI: reducers for form state, pure view-model helpers (`lib/model.ts`), one transport function (`lib/rpc.ts`).

## Adding a datastore
Implement `Store` + `Conn` (see `odoo-sqlite`, ~80 lines) and pick/define a `Dialect` (types, placeholders, PK, ILIKE, RETURNING).
Run `odoo-core` DDL plan for it: `ddl::install_plan(&registry, dialect)`. The shared flow test-suite should pass unchanged.
