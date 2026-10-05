//! Small pure helpers shared by module rule sets.
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, Result, Row, Value};

pub fn row(pairs: &[(&str, Value)]) -> Row { pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect() }
pub fn num(r: &Row, k: &str) -> f64 { r.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0) }
pub fn id_of(r: &Row, k: &str) -> Option<i64> { match r.get(k)? { Value::Int(i) => Some(*i), Value::List(l) => l.first()?.as_i64(), _ => None } }
pub fn text(r: &Row, k: &str) -> Option<String> { r.get(k).and_then(|v| v.as_str()).map(String::from) }
pub fn ids(r: &Row, k: &str) -> Vec<i64> { match r.get(k) { Some(Value::List(l)) => l.iter().filter_map(|v| v.as_i64()).collect(), _ => vec![] } }
pub fn r2(x: f64) -> f64 { (x * 100.0).round() / 100.0 }
pub fn term(f: &str, op: &str, v: impl Into<Value>) -> Domain { Domain::Term(f.into(), op.into(), v.into()) }
pub fn find_one(env: &Env, model: &str, dom: Domain) -> Result<Option<i64>> { Ok(orm::search(env, model, &dom, None, Some(1), 0)?.first().copied()) }
pub fn find_or_create(env: &Env, model: &str, dom: Domain, vals: Row) -> Result<i64> {
    match find_one(env, model, dom)? { Some(i) => Ok(i), None => orm::create(env, model, vals) }
}
/// Raw record (stored cols + x2many id lists), sudo.
pub fn rec(env: &Env, model: &str, id: i64) -> Result<Row> { orm::browse_raw(&env.sudo(), env.reg.model(model)?, id) }
pub fn children(env: &Env, model: &str, fk: &str, parent: i64) -> Result<Vec<Row>> {
    let e = env.sudo();
    let ids = orm::search(&e.with_ctx("active_test", Value::Bool(false)), model, &term(fk, "=", parent), Some("id"), None, 0)?;
    ids.into_iter().map(|i| rec(env, model, i)).collect()
}

/// ir.sequence `next_by_code`, creating the sequence on first use (standard implementation).
pub fn next_seq(env: &Env, code: &str, prefix: &str, padding: i64) -> Result<String> {
    let e = env.sudo();
    let sid = match find_one(&e, "ir.sequence", term("code", "=", code))? {
        Some(i) => i,
        None => orm::create(&e, "ir.sequence", row(&[("name", code.into()), ("code", code.into()), ("prefix", prefix.into()), ("padding", padding.into()), ("number_next", 1.into()), ("number_increment", 1.into())]))?,
    };
    let s = rec(&e, "ir.sequence", sid)?;
    let n = s.get("number_next").and_then(|v| v.as_i64()).unwrap_or(1);
    let inc = s.get("number_increment").and_then(|v| v.as_i64()).unwrap_or(1).max(1);
    let pad = s.get("padding").and_then(|v| v.as_i64()).unwrap_or(padding).max(0) as usize;
    orm::write(&e, "ir.sequence", &[sid], row(&[("number_next", (n + inc).into())]))?;
    Ok(format!("{}{:0pad$}", text(&s, "prefix").unwrap_or_default(), n, pad = pad))
}
