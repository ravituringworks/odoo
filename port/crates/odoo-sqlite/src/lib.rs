//! SQLite backend for odoo-core (default storage).
use odoo_core::dialect::{Dialect, Sqlite};
use odoo_core::error::{OdooError, Result};
use odoo_core::store::{Conn, Store};
use odoo_core::value::{Row, Value};
use rusqlite::types::{ToSqlOutput, Value as SV, ValueRef};
use std::sync::Mutex;

pub struct SqliteStore { conn: Mutex<rusqlite::Connection>, dialect: Sqlite }

fn err(e: rusqlite::Error) -> OdooError { OdooError::Storage(e.to_string()) }

impl SqliteStore {
    pub fn open(path: &str) -> Result<Self> {
        let c = if path == ":memory:" { rusqlite::Connection::open_in_memory() } else { rusqlite::Connection::open(path) }.map_err(err)?;
        c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=OFF; PRAGMA synchronous=NORMAL;").map_err(err)?;
        Ok(SqliteStore { conn: Mutex::new(c), dialect: Sqlite })
    }
}

struct P<'a>(&'a Value);
impl rusqlite::ToSql for P<'_> {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(match self.0 {
            Value::Null => SV::Null, Value::Bool(b) => SV::Integer(*b as i64), Value::Int(i) => SV::Integer(*i), Value::Float(f) => SV::Real(*f),
            Value::Text(s) => SV::Text(s.clone()), v => SV::Text(v.to_json().to_string()),
        }))
    }
}

struct Tx<'a>(&'a rusqlite::Connection, &'a Sqlite);
impl Conn for Tx<'_> {
    fn dialect(&self) -> &dyn Dialect { self.1 }
    fn execute(&self, sql: &str, params: &[Value]) -> Result<u64> {
        let ps: Vec<P> = params.iter().map(P).collect();
        self.0.prepare_cached(sql).and_then(|mut s| s.execute(rusqlite::params_from_iter(ps.iter()))).map(|n| n as u64).map_err(|e| OdooError::Storage(format!("{e} -- {sql}")))
    }
    fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<Row>> {
        let ps: Vec<P> = params.iter().map(P).collect();
        let mut st = self.0.prepare_cached(sql).map_err(|e| OdooError::Storage(format!("{e} -- {sql}")))?;
        let cols: Vec<String> = st.column_names().iter().map(|s| s.to_string()).collect();
        let rows = st.query_map(rusqlite::params_from_iter(ps.iter()), |r| {
            let mut row = Row::new();
            for (i, c) in cols.iter().enumerate() {
                row.insert(c.clone(), match r.get_ref(i)? { ValueRef::Null => Value::Null, ValueRef::Integer(i) => Value::Int(i), ValueRef::Real(f) => Value::Float(f), ValueRef::Text(t) => Value::Text(String::from_utf8_lossy(t).into()), ValueRef::Blob(_) => Value::Text("<blob>".into()) });
            }
            Ok(row)
        }).map_err(err)?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(err)
    }
    fn insert(&self, sql: &str, params: &[Value]) -> Result<i64> {
        let r = self.query(sql, params)?;
        match r.first().and_then(|r| r.get("id")).and_then(|v| v.as_i64()) { Some(i) => Ok(i), None => Ok(self.0.last_insert_rowid()) }
    }
}

impl Store for SqliteStore {
    fn dialect(&self) -> &dyn Dialect { &self.dialect }
    fn snapshot(&self, dest: &std::path::Path) -> Result<()> {
        let g = self.conn.lock().map_err(|_| OdooError::Storage("poisoned lock".into()))?;
        g.execute("VACUUM INTO ?1", [dest.to_string_lossy().as_ref()]).map(|_| ()).map_err(err)   // consistent even while the WAL has unmerged pages
    }
    fn transaction(&self, f: &mut dyn FnMut(&dyn Conn) -> Result<()>) -> Result<()> {
        let mut g = self.conn.lock().map_err(|_| OdooError::Storage("poisoned lock".into()))?;
        let tx = g.transaction().map_err(err)?;
        f(&Tx(&tx, &self.dialect))?;
        tx.commit().map_err(err)
    }
}
