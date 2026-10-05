//! DuckDB backend for odoo-core (analytics-friendly embedded store). Same ORM, different `Dialect`.
use duckdb::types::{Value as DV, ValueRef};
use odoo_core::dialect::{Dialect, DuckDb};
use odoo_core::error::{OdooError, Result};
use odoo_core::store::{Conn, Store};
use odoo_core::value::{Row, Value};
use std::sync::Mutex;

pub struct DuckStore { conn: Mutex<duckdb::Connection>, dialect: DuckDb }
fn err(e: duckdb::Error) -> OdooError { OdooError::Storage(e.to_string()) }

impl DuckStore {
    pub fn open(path: &str) -> Result<Self> {
        let c = if path == ":memory:" { duckdb::Connection::open_in_memory() } else { duckdb::Connection::open(path) }.map_err(err)?;
        Ok(DuckStore { conn: Mutex::new(c), dialect: DuckDb })
    }
}

/// days since 1970-01-01 -> (y, m, d) (Howard Hinnant's civil_from_days)
fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719468; let era = z.div_euclid(146097); let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1; let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}
fn iso_date(days: i64) -> String { let (y, m, d) = civil(days); format!("{y:04}-{m:02}-{d:02}") }
fn iso_ts(micros: i64) -> String { let s = micros.div_euclid(1_000_000); let (days, rem) = (s.div_euclid(86400), s.rem_euclid(86400)); format!("{} {:02}:{:02}:{:02}", iso_date(days), rem / 3600, rem % 3600 / 60, rem % 60) }

fn to_dv(v: &Value) -> DV {
    match v { Value::Null => DV::Null, Value::Bool(b) => DV::Boolean(*b), Value::Int(i) => DV::BigInt(*i), Value::Float(f) => DV::Double(*f), Value::Text(s) => DV::Text(s.clone()), o => DV::Text(o.to_json().to_string()) }
}

struct Tx<'a>(&'a duckdb::Connection, &'a DuckDb);
impl Conn for Tx<'_> {
    fn dialect(&self) -> &dyn Dialect { self.1 }
    fn execute(&self, sql: &str, params: &[Value]) -> Result<u64> {
        let ps: Vec<DV> = params.iter().map(to_dv).collect();
        self.0.execute(sql, duckdb::params_from_iter(ps.iter())).map(|n| n as u64).map_err(|e| OdooError::Storage(format!("{e} -- {sql}")))
    }
    fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<Row>> {
        let ps: Vec<DV> = params.iter().map(to_dv).collect();
        let mut st = self.0.prepare(sql).map_err(|e| OdooError::Storage(format!("{e} -- {sql}")))?;
        let mut rows = st.query(duckdb::params_from_iter(ps.iter())).map_err(|e| OdooError::Storage(format!("{e} -- {sql}")))?;
        let names: Vec<String> = rows.as_ref().map(|s| s.column_names()).unwrap_or_default();
        let mut out = vec![];
        while let Some(r) = rows.next().map_err(err)? {
            let mut row = Row::new();
            for (i, n) in names.iter().enumerate() {
                row.insert(n.clone(), match r.get_ref(i).map_err(err)? {
                    ValueRef::Null => Value::Null, ValueRef::Boolean(b) => Value::Bool(b),
                    ValueRef::TinyInt(x) => Value::Int(x as i64), ValueRef::SmallInt(x) => Value::Int(x as i64), ValueRef::Int(x) => Value::Int(x as i64), ValueRef::BigInt(x) => Value::Int(x),
                    ValueRef::UTinyInt(x) => Value::Int(x as i64), ValueRef::USmallInt(x) => Value::Int(x as i64), ValueRef::UInt(x) => Value::Int(x as i64), ValueRef::UBigInt(x) => Value::Int(x as i64),
                    ValueRef::HugeInt(x) => Value::Int(x as i64),
                    ValueRef::Float(x) => Value::Float(x as f64), ValueRef::Double(x) => Value::Float(x),
                    ValueRef::Text(t) => Value::Text(String::from_utf8_lossy(t).into()),
                    ValueRef::Date32(d) => Value::Text(iso_date(d as i64)),
                    ValueRef::Timestamp(unit, v) => Value::Text(iso_ts(match unit { duckdb::types::TimeUnit::Second => v * 1_000_000, duckdb::types::TimeUnit::Millisecond => v * 1_000, duckdb::types::TimeUnit::Microsecond => v, duckdb::types::TimeUnit::Nanosecond => v / 1_000 })),
                    other => Value::Text(format!("{other:?}")),
                });
            }
            out.push(row);
        }
        Ok(out)
    }
    // DuckDB has no SAVEPOINT; statement errors do not abort the surrounding transaction, so nested scopes are no-ops.
    fn savepoint(&self, _: &str) -> Result<()> { Ok(()) }
    fn release(&self, _: &str) -> Result<()> { Ok(()) }
    fn rollback_to(&self, _: &str) -> Result<()> { Ok(()) }
    fn insert(&self, sql: &str, params: &[Value]) -> Result<i64> {
        self.query(sql, params)?.first().and_then(|r| r.get("id")).and_then(|v| v.as_i64()).ok_or_else(|| OdooError::Storage("insert returned no id".into()))
    }
}

impl Store for DuckStore {
    fn dialect(&self) -> &dyn Dialect { &self.dialect }
    fn transaction(&self, f: &mut dyn FnMut(&dyn Conn) -> Result<()>) -> Result<()> {
        let g = self.conn.lock().map_err(|_| OdooError::Storage("poisoned lock".into()))?;
        g.execute_batch("BEGIN").map_err(err)?;
        match f(&Tx(&g, &self.dialect)) { Ok(()) => g.execute_batch("COMMIT").map_err(err), Err(e) => { let _ = g.execute_batch("ROLLBACK"); Err(e) } }
    }
}

#[cfg(test)]
mod tests {
    #[test] fn dates() { assert_eq!(super::iso_date(20731), "2026-10-05"); assert_eq!(super::iso_ts(1_791_158_400_000_000 + 3_723_000_000), "2026-10-05 01:02:03"); }
}
