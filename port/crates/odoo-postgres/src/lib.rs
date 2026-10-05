//! PostgreSQL backend (also used for orbit-rs via `PgStore::open_with(url, Box::new(OrbitRs))`).
//! Parameters are inlined as escaped literals through the simple-query path, which sidesteps per-column
//! binary type negotiation; every string is quote-escaped and control bytes are rejected.
use odoo_core::dialect::{Dialect, OrbitRs, Postgres};
use odoo_core::error::{OdooError, Result};
use odoo_core::store::{Conn, Store};
use odoo_core::value::{Row, Value};
use postgres::{Client, NoTls, SimpleQueryMessage};
use std::sync::Mutex;

pub struct PgStore { client: Mutex<Client>, dialect: Box<dyn Dialect> }
fn err(e: postgres::Error) -> OdooError { OdooError::Storage(fmt_err(&e)) }
fn fmt_err(e: &postgres::Error) -> String { match e.as_db_error() { Some(d) => format!("{}: {}{}", d.code().code(), d.message(), d.detail().map(|x| format!(" ({x})")).unwrap_or_default()), None => e.to_string() } }

impl PgStore {
    pub fn open(url: &str) -> Result<Self> { Self::open_with(url, Box::new(Postgres)) }
    pub fn open_orbit(url: &str) -> Result<Self> { Self::open_with(url, Box::new(OrbitRs)) }
    pub fn open_with(url: &str, dialect: Box<dyn Dialect>) -> Result<Self> { Ok(PgStore { client: Mutex::new(Client::connect(url, NoTls).map_err(err)?), dialect }) }
}

fn lit(v: &Value) -> Result<String> {
    Ok(match v {
        Value::Null => "NULL".into(), Value::Bool(b) => if *b { "TRUE".into() } else { "FALSE".into() },
        Value::Int(i) => i.to_string(), Value::Float(f) if f.is_finite() => format!("{f}"), Value::Float(_) => "NULL".into(),
        Value::Text(s) => { if s.contains('\0') { return Err(OdooError::Storage("NUL byte in text parameter".into())); } format!("'{}'", s.replace('\'', "''")) }
        o => format!("'{}'", o.to_json().to_string().replace('\'', "''")),
    })
}

/// Replace `$n` placeholders (outside quotes) with literals.
pub fn inline(sql: &str, params: &[Value]) -> Result<String> {
    let mut out = String::with_capacity(sql.len() + 16 * params.len());
    let b: Vec<char> = sql.chars().collect(); let mut i = 0; let mut q = false;
    while i < b.len() {
        let c = b[i];
        if c == '\'' { q = !q; out.push(c); i += 1; }
        else if c == '$' && !q && i + 1 < b.len() && b[i + 1].is_ascii_digit() {
            let mut j = i + 1; let mut n = 0usize;
            while j < b.len() && b[j].is_ascii_digit() { n = n * 10 + b[j].to_digit(10).unwrap() as usize; j += 1; }
            out.push_str(&lit(params.get(n - 1).ok_or_else(|| OdooError::Storage(format!("missing param ${n}")))?)?); i = j;
        } else { out.push(c); i += 1; }
    }
    Ok(out)
}

fn parse_cell(s: Option<&str>) -> Value {
    match s { None => Value::Null, Some(t) => if let Ok(i) = t.parse::<i64>() { Value::Int(i) } else if let Ok(f) = t.parse::<f64>() { Value::Float(f) } else { match t { "t" => Value::Bool(true), "f" => Value::Bool(false), _ => Value::Text(t.into()) } } }
}

struct Tx<'a>(Mutex<postgres::Transaction<'a>>, &'a dyn Dialect);
impl Conn for Tx<'_> {
    fn dialect(&self) -> &dyn Dialect { self.1 }
    fn execute(&self, sql: &str, params: &[Value]) -> Result<u64> {
        let q = inline(sql, params)?;
        let mut g = self.0.lock().unwrap();
        let mut n = 0;
        for m in g.simple_query(&q).map_err(|e| OdooError::Storage(format!("{} -- {sql}", fmt_err(&e))))? { if let SimpleQueryMessage::CommandComplete(c) = m { n = c; } }
        Ok(n)
    }
    fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<Row>> {
        let q = inline(sql, params)?;
        let mut g = self.0.lock().unwrap();
        let mut out = vec![];
        for m in g.simple_query(&q).map_err(|e| OdooError::Storage(format!("{} -- {sql}", fmt_err(&e))))? {
            if let SimpleQueryMessage::Row(r) = m { out.push(r.columns().iter().enumerate().map(|(i, c)| (c.name().to_string(), parse_cell(r.get(i)))).collect()); }
        }
        Ok(out)
    }
    fn insert(&self, sql: &str, params: &[Value]) -> Result<i64> {
        self.query(sql, params)?.first().and_then(|r| r.get("id")).and_then(|v| v.as_i64()).ok_or_else(|| OdooError::Storage("insert returned no id".into()))
    }
}

impl Store for PgStore {
    fn dialect(&self) -> &dyn Dialect { self.dialect.as_ref() }
    fn transaction(&self, f: &mut dyn FnMut(&dyn Conn) -> Result<()>) -> Result<()> {
        let mut g = self.client.lock().map_err(|_| OdooError::Storage("poisoned lock".into()))?;
        let tx = g.transaction().map_err(err)?;
        let t = Tx(Mutex::new(tx), self.dialect.as_ref());
        match f(&t) { Ok(()) => t.0.into_inner().unwrap().commit().map_err(err), Err(e) => { let _ = t.0.into_inner().unwrap().rollback(); Err(e) } }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn inlines_and_escapes() {
        assert_eq!(inline("SELECT $1, $2 FROM t WHERE a = $1 AND b = '$3'", &[Value::Text("O'Brien".into()), Value::Int(3)]).unwrap(), "SELECT 'O''Brien', 3 FROM t WHERE a = 'O''Brien' AND b = '$3'");
        assert!(inline("SELECT $1", &[Value::Text("a\0b".into())]).is_err());
    }
}
