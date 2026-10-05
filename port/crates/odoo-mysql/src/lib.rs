//! MySQL / MariaDB backend using native `?` binding.
use mysql::prelude::Queryable;
use mysql::{Conn as MyConn, Opts, TxOpts, Value as MV};
use odoo_core::dialect::{Dialect, MySql};
use odoo_core::error::{OdooError, Result};
use odoo_core::store::{Conn, Store};
use odoo_core::value::{Row, Value};
use std::sync::Mutex;

pub struct MySqlStore { conn: Mutex<MyConn>, dialect: MySql }
fn err(e: mysql::Error) -> OdooError { OdooError::Storage(e.to_string()) }

impl MySqlStore {
    pub fn open(url: &str) -> Result<Self> { Ok(MySqlStore { conn: Mutex::new(MyConn::new(Opts::from_url(url).map_err(|e| OdooError::Storage(e.to_string()))?).map_err(err)?), dialect: MySql }) }
}

fn to_mv(v: &Value) -> MV { match v { Value::Null => MV::NULL, Value::Bool(b) => MV::Int(*b as i64), Value::Int(i) => MV::Int(*i), Value::Float(f) => MV::Double(*f), Value::Text(s) => MV::Bytes(s.clone().into_bytes()), o => MV::Bytes(o.to_json().to_string().into_bytes()) } }
fn from_mv(v: MV) -> Value {
    match v { MV::NULL => Value::Null, MV::Int(i) => Value::Int(i), MV::UInt(u) => Value::Int(u as i64), MV::Float(f) => Value::Float(f as f64), MV::Double(f) => Value::Float(f),
        MV::Bytes(b) => { let s = String::from_utf8_lossy(&b).to_string(); match s.parse::<f64>() { Ok(f) if s.contains('.') && !s.contains(' ') => Value::Float(f), _ => Value::Text(s) } }
        other => Value::Text(format!("{other:?}")) }
}

struct Tx<'a>(Mutex<mysql::Transaction<'a>>, &'a MySql);
impl Conn for Tx<'_> {
    fn dialect(&self) -> &dyn Dialect { self.1 }
    fn execute(&self, sql: &str, params: &[Value]) -> Result<u64> {
        let mut g = self.0.lock().unwrap();
        g.exec_drop(sql, params.iter().map(to_mv).collect::<Vec<_>>()).map_err(|e| OdooError::Storage(format!("{e} -- {sql}")))?;
        Ok(g.affected_rows())
    }
    fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<Row>> {
        let mut g = self.0.lock().unwrap();
        let rows: Vec<mysql::Row> = g.exec(sql, params.iter().map(to_mv).collect::<Vec<_>>()).map_err(|e| OdooError::Storage(format!("{e} -- {sql}")))?;
        Ok(rows.into_iter().map(|r| { let cols: Vec<String> = r.columns_ref().iter().map(|c| c.name_str().to_string()).collect(); cols.into_iter().zip(r.unwrap()).map(|(c, v)| (c, from_mv(v))).collect() }).collect())
    }
    fn insert(&self, sql: &str, params: &[Value]) -> Result<i64> {
        let mut g = self.0.lock().unwrap();
        g.exec_drop(sql, params.iter().map(to_mv).collect::<Vec<_>>()).map_err(|e| OdooError::Storage(format!("{e} -- {sql}")))?;
        Ok(g.last_insert_id().unwrap_or(0) as i64)
    }
}

impl Store for MySqlStore {
    fn dialect(&self) -> &dyn Dialect { &self.dialect }
    fn transaction(&self, f: &mut dyn FnMut(&dyn Conn) -> Result<()>) -> Result<()> {
        let mut g = self.conn.lock().map_err(|_| OdooError::Storage("poisoned lock".into()))?;
        let tx = g.start_transaction(TxOpts::default()).map_err(err)?;
        let t = Tx(Mutex::new(tx), &self.dialect);
        match f(&t) { Ok(()) => t.0.into_inner().unwrap().commit().map_err(err), Err(e) => { let _ = t.0.into_inner().unwrap().rollback(); Err(e) } }
    }
}
