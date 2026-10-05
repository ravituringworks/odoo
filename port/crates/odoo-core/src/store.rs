//! Storage ports. A backend implements `Store` (+ `Conn` inside a transaction) and a `Dialect`.
use crate::dialect::Dialect;
use crate::error::Result;
use crate::value::{Row, Value};

pub trait Conn {
    fn dialect(&self) -> &dyn Dialect;
    fn execute(&self, sql: &str, params: &[Value]) -> Result<u64>;
    fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<Row>>;
    /// INSERT returning the new primary key (RETURNING or last_insert_id depending on backend).
    fn insert(&self, sql: &str, params: &[Value]) -> Result<i64>;
    /// Nested rollback scope; required on PostgreSQL where a failed statement aborts the whole transaction.
    fn savepoint(&self, name: &str) -> Result<()> { self.execute(&format!("SAVEPOINT {name}"), &[]).map(|_| ()) }
    fn release(&self, name: &str) -> Result<()> { self.execute(&format!("RELEASE SAVEPOINT {name}"), &[]).map(|_| ()) }
    fn rollback_to(&self, name: &str) -> Result<()> { self.execute(&format!("ROLLBACK TO SAVEPOINT {name}"), &[]).map(|_| ()) }
}

/// Run `f` inside a savepoint: Ok keeps its effects, Err undoes only them.
pub fn nested<R>(c: &dyn Conn, f: impl FnOnce() -> Result<R>) -> Result<R> {
    c.savepoint("odoo_sp")?;
    match f() { Ok(v) => { c.release("odoo_sp")?; Ok(v) } Err(e) => { c.rollback_to("odoo_sp")?; let _ = c.release("odoo_sp"); Err(e) } }
}

pub trait Store: Send + Sync {
    fn dialect(&self) -> &dyn Dialect;
    /// Run `f` atomically; rolls back on Err.
    fn transaction(&self, f: &mut dyn FnMut(&dyn Conn) -> Result<()>) -> Result<()>;
}

/// Ergonomic generic wrapper over the object-safe `transaction`.
pub fn run<R>(store: &dyn Store, mut f: impl FnMut(&dyn Conn) -> Result<R>) -> Result<R> {
    let mut out = None;
    store.transaction(&mut |c| { out = Some(f(c)?); Ok(()) })?;
    Ok(out.expect("transaction body ran"))
}

/// Apply DDL in bounded transactions (PostgreSQL's lock table caps locks per transaction; Odoo also commits in stages).
pub fn apply_plan(store: &dyn Store, plan: &[String]) -> Result<()> {
    for chunk in plan.chunks(40) { run(store, |c| { for s in chunk { c.execute(s, &[])?; } Ok(()) })?; }
    Ok(())
}

/// Apply an upgrade plan: each statement in its own transaction; a failing `ADD COLUMN` (already added by a previous,
/// interrupted attempt) is ignored so installs can be retried.
pub fn apply_upgrade(store: &dyn Store, plan: &[String]) -> Result<()> {
    for s in plan {
        let r = run(store, |c| c.execute(s, &[]).map(|_| ()));
        if let Err(e) = r { if !s.starts_with("ALTER TABLE") || !s.contains("ADD COLUMN") { return Err(e); } }
    }
    Ok(())
}

/// Tables and columns that currently exist (via the dialect's information-schema query).
pub fn existing_columns(store: &dyn Store) -> Result<std::collections::BTreeMap<String, std::collections::BTreeSet<String>>> {
    run(store, |c| {
        let mut out: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> = Default::default();
        for r in c.query(c.dialect().columns_query(), &[])? {
            if let (Some(t), Some(col)) = (r.get("table_name").and_then(|v| v.as_str()), r.get("column_name").and_then(|v| v.as_str())) { out.entry(t.to_string()).or_default().insert(col.to_string()); }
        }
        Ok(out)
    })
}

/// Bring an existing database up to date with the registry (idempotent; a no-op on an up-to-date schema).
pub fn sync_schema(store: &dyn Store, reg: &crate::Registry) -> Result<usize> {
    let plan = crate::ddl::sync_plan(reg, &existing_columns(store)?, store.dialect());
    apply_upgrade(store, &plan)?;
    Ok(plan.len())
}
