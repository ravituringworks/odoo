//! Functional ORM: stateless operations over an immutable `Env`. Business logic is
//! injected as pure hook functions through `Rules` (no inheritance, no global state).
use crate::domain::{compile, Domain};
use crate::error::{OdooError, Result};
use crate::schema::{FieldDef, ModelDef, Registry};
use crate::store::Conn;
use crate::value::{Row, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub type Hook = fn(&Env, Row) -> Result<Row>;
pub type After = fn(&Env, &[i64], &Row) -> Result<()>;
pub type Compute = fn(&Env, &Row) -> Result<Value>;
pub type Action = fn(&Env, &[i64], &Row) -> Result<Value>;
pub type Dependents = fn(&Env, &[i64]) -> Result<Vec<(String, Vec<i64>)>>;
pub type DefaultFn = fn(&Env) -> Result<Value>;

/// Immutable table of behaviours keyed by model; built with the `with_*` combinators.
#[derive(Clone, Default)]
pub struct Rules {
    pub before_create: HashMap<String, Vec<Hook>>,
    pub before_write: HashMap<String, Vec<Hook>>,
    pub after_create: HashMap<String, Vec<After>>,
    pub after_write: HashMap<String, Vec<After>>,
    pub computes: HashMap<(String, String), Compute>,
    pub actions: HashMap<(String, String), Action>,
    pub defaults: HashMap<(String, String), DefaultFn>,
    /// Called before unlink; returns (model, ids) whose stored computes must be refreshed after deletion.
    pub unlink_dependents: HashMap<String, Vec<Dependents>>,
    /// Form-time hooks: current (unsaved) values -> values to merge into the form.
    pub onchange: HashMap<String, Vec<Hook>>,
}
impl Rules {
    pub fn before_create(mut self, m: &str, h: Hook) -> Self { self.before_create.entry(m.into()).or_default().push(h); self }
    pub fn before_write(mut self, m: &str, h: Hook) -> Self { self.before_write.entry(m.into()).or_default().push(h); self }
    pub fn after_create(mut self, m: &str, h: After) -> Self { self.after_create.entry(m.into()).or_default().push(h); self }
    pub fn after_write(mut self, m: &str, h: After) -> Self { self.after_write.entry(m.into()).or_default().push(h); self }
    pub fn compute(mut self, m: &str, f: &str, c: Compute) -> Self { self.computes.insert((m.into(), f.into()), c); self }
    pub fn action(mut self, m: &str, name: &str, a: Action) -> Self { self.actions.insert((m.into(), name.into()), a); self }
    pub fn default_for(mut self, m: &str, f: &str, d: DefaultFn) -> Self { self.defaults.insert((m.into(), f.into()), d); self }
    pub fn onchange(mut self, m: &str, h: Hook) -> Self { self.onchange.entry(m.into()).or_default().push(h); self }
    pub fn on_unlink(mut self, m: &str, d: Dependents) -> Self { self.unlink_dependents.entry(m.into()).or_default().push(d); self }
    pub fn merge(mut self, o: Rules) -> Self {
        for (k, v) in o.before_create { self.before_create.entry(k).or_default().extend(v) }
        for (k, v) in o.before_write { self.before_write.entry(k).or_default().extend(v) }
        for (k, v) in o.after_create { self.after_create.entry(k).or_default().extend(v) }
        for (k, v) in o.after_write { self.after_write.entry(k).or_default().extend(v) }
        for (k, v) in o.onchange { self.onchange.entry(k).or_default().extend(v) }
        for (k, v) in o.unlink_dependents { self.unlink_dependents.entry(k).or_default().extend(v) }
        self.computes.extend(o.computes); self.actions.extend(o.actions); self.defaults.extend(o.defaults); self
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Op { Read, Write, Create, Unlink }
impl Op { fn name(self) -> &'static str { match self { Op::Read => "read", Op::Write => "write", Op::Create => "create", Op::Unlink => "unlink" } } }

/// Access control port (ir.model.access equivalent).
pub trait Access: Send + Sync { fn check(&self, uid: i64, model: &str, op: Op) -> bool; }
pub struct AllowAll;
impl Access for AllowAll { fn check(&self, _: i64, _: &str, _: Op) -> bool { true } }

#[derive(Clone, Default)]
pub struct AccessTable { pub entries: Vec<AccessEntry>, pub user_groups: BTreeMap<i64, BTreeSet<String>>, pub user_group_ids: BTreeMap<i64, Vec<i64>> }
#[derive(Clone)]
pub struct AccessEntry { pub model: String, pub group: Option<String>, pub read: bool, pub write: bool, pub create: bool, pub unlink: bool }
impl Access for AccessTable {
    fn check(&self, uid: i64, model: &str, op: Op) -> bool {
        if uid == 1 { return true; } // superuser
        let groups = self.user_groups.get(&uid);
        self.entries.iter().any(|e| e.model == model && match &e.group { None => true, Some(g) => groups.map_or(false, |s| s.contains(g)) } && match op { Op::Read => e.read, Op::Write => e.write, Op::Create => e.create, Op::Unlink => e.unlink })
    }
}

/// Record-rule port (ir.rule equivalent): extra domain a user must satisfy for an operation; `None` = unrestricted.
pub trait RecordRules: Send + Sync {
    fn domain(&self, uid: i64, model: &str, op: Op) -> Option<Domain>;
    /// Same, with the request context (`company_id` / `allowed_company_ids` drive multi-company rules).
    fn domain_ctx(&self, uid: i64, model: &str, op: Op, _ctx: &BTreeMap<String, Value>) -> Option<Domain> { self.domain(uid, model, op) }
}
pub struct NoRules;
impl RecordRules for NoRules { fn domain(&self, _: i64, _: &str, _: Op) -> Option<Domain> { None } }
static NO_RULES: NoRules = NoRules;

pub struct Env<'a> {
    pub reg: &'a Registry,
    pub conn: &'a dyn Conn,
    pub rules: &'a Rules,
    pub access: &'a dyn Access,
    pub rr: &'a dyn RecordRules,
    pub uid: i64,
    pub ctx: BTreeMap<String, Value>,
}
impl<'a> Env<'a> {
    pub fn new(reg: &'a Registry, conn: &'a dyn Conn, rules: &'a Rules, access: &'a dyn Access, uid: i64) -> Self { Env { reg, conn, rules, access, rr: &NO_RULES, uid, ctx: BTreeMap::new() } }
    pub fn with_record_rules(self, rr: &'a dyn RecordRules) -> Env<'a> { Env { rr, ..self } }
    pub fn sudo(&self) -> Env<'a> { Env { reg: self.reg, conn: self.conn, rules: self.rules, access: &AllowAll, rr: &NO_RULES, uid: 1, ctx: self.ctx.clone() } }
    pub fn with_ctx(&self, k: &str, v: Value) -> Env<'a> { let mut c = self.ctx.clone(); c.insert(k.into(), v); Env { reg: self.reg, conn: self.conn, rules: self.rules, access: self.access, rr: self.rr, uid: self.uid, ctx: c } }
    fn d(&self) -> &dyn crate::dialect::Dialect { self.conn.dialect() }
    fn check(&self, model: &str, op: Op) -> Result<()> {
        if self.access.check(self.uid, model, op) { Ok(()) } else { Err(OdooError::AccessDenied { op: op.name().into(), model: model.into(), uid: self.uid }) }
    }
    fn q(&self, i: &str) -> String { self.d().quote(i) }
    fn ph(&self, n: usize) -> String { self.d().placeholder(n) }
}

fn now_str() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    let z = days + 719468; let era = z.div_euclid(146097); let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1; let m = if mp < 10 { mp + 3 } else { mp - 9 }; let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}", rem / 3600, rem % 3600 / 60, rem % 60)
}
fn scoped(env: &Env, model: &str, op: Op, d: &Domain) -> Domain {
    match env.rr.domain_ctx(env.uid, model, op, &env.ctx) { Some(r) => d.clone().and(r), None => d.clone() }
}

/// Ensure every id satisfies the record rules for `op` (write/unlink); else AccessDenied.
fn enforce_ids(env: &Env, model: &str, ids: &[i64], op: Op) -> Result<()> {
    let Some(r) = env.rr.domain_ctx(env.uid, model, op, &env.ctx) else { return Ok(()) };
    let dom = Domain::Term("id".into(), "in".into(), Value::List(ids.iter().map(|i| Value::Int(*i)).collect())).and(r);
    let m = env.reg.model(model)?;
    let mut params = vec![];
    let w = compile(env.reg, env.d(), model, "t", &dom, &mut params)?;
    let c = env.conn.query(&format!("SELECT COUNT(*) AS c FROM {} t WHERE {w}", env.q(&m.table())), &params)?;
    if c.first().and_then(|r| r["c"].as_i64()).unwrap_or(0) as usize != ids.iter().collect::<BTreeSet<_>>().len() { return Err(OdooError::AccessDenied { op: op.name().into(), model: model.into(), uid: env.uid }); }
    Ok(())
}

pub fn today() -> String { now_str()[..10].to_string() }
pub fn now() -> String { now_str() }

/// Days since 1970-01-01 of an ISO date (inverse of `shift_date` arithmetic; tolerant of `YYYY-MM-DD HH:MM:SS`).
pub fn epoch_days(d: &str) -> i64 {
    let p: Vec<i64> = d.get(..10).unwrap_or("1970-01-01").split('-').filter_map(|x| x.parse().ok()).collect();
    let (y, m, dd) = (p.first().copied().unwrap_or(1970), p.get(1).copied().unwrap_or(1), p.get(2).copied().unwrap_or(1));
    let (yy, mm) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = yy.div_euclid(400); let yoe = yy.rem_euclid(400); let doy = (153 * mm + 2) / 5 + dd - 1; let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// ISO date `d` shifted by `days` (civil calendar arithmetic, no external crates).
pub fn shift_date(d: &str, days: i64) -> String {
    let p: Vec<i64> = d.get(..10).unwrap_or("1970-01-01").split('-').filter_map(|x| x.parse().ok()).collect();
    let (y, m, dd) = (p.first().copied().unwrap_or(1970), p.get(1).copied().unwrap_or(1), p.get(2).copied().unwrap_or(1));
    let (yy, mm) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = yy.div_euclid(400); let yoe = yy.rem_euclid(400); let doy = (153 * mm + 2) / 5 + dd - 1; let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let z = era * 146097 + doe - 719468 + days + 719468;
    let era = z.div_euclid(146097); let doe = z.rem_euclid(146097); let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; let y2 = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); let mp = (5 * doy + 2) / 153; let d2 = doy - (153 * mp + 2) / 5 + 1; let m2 = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02}", if m2 <= 2 { y2 + 1 } else { y2 }, m2, d2)
}

fn json_default(j: &serde_json::Value) -> Value { Value::from_json(j) }

fn split_vals<'m>(model: &'m ModelDef, vals: Row) -> Result<(Row, Vec<(String, &'m FieldDef, Value)>)> {
    let mut stored = Row::new(); let mut x2m = vec![];
    for (k, v) in vals {
        let f = model.fields.get(&k).ok_or_else(|| OdooError::UnknownField(model.name.clone(), k.clone()))?;
        // Odoo semantics: `False` means "unset" for every non-boolean field
        let v = if f.ty != "Boolean" && matches!(v, Value::Bool(false)) && !f.is_x2many() { Value::Null } else { v };
        // many2one arrives as id, [id, name] or []; scalars must never be lists
        let v = match (f.ty.as_str(), v) {
            ("Many2one", Value::List(l)) => l.into_iter().next().filter(|x| matches!(x, Value::Int(_))).unwrap_or(Value::Null),
            ("Many2one", Value::Bool(true)) => Value::Null,
            (_, v) => v,
        };
        if f.is_x2many() { x2m.push((k, f, v)); } else if f.is_stored() { stored.insert(k, v); }
        // non-stored computed/related fields are ignored on write, like Odoo for readonly computed
    }
    Ok((stored, x2m))
}

pub fn create(env: &Env, model: &str, vals: Row) -> Result<i64> {
    env.check(model, Op::Create)?;
    let m = env.reg.model(model)?;
    let mut vals = vals;
    for (n, f) in m.stored_fields() {
        if vals.contains_key(n) { continue; }
        if f.is_computed() { continue; }
        if let Some(d) = env.rules.defaults.get(&(model.to_string(), n.clone())) { vals.insert(n.clone(), d(env)?); }
        else if let Some(d) = f.static_default() { vals.insert(n.clone(), json_default(&d)); }
        else if n == "tz" && f.default.is_some() { vals.insert(n.clone(), Value::Text("UTC".into())); }
        else if f.default.is_some() && matches!(f.ty.as_str(), "Date" | "Datetime") { vals.insert(n.clone(), Value::Text(if f.ty == "Date" { today() } else { now() })); }
        else if f.ty == "Many2one" && (f.is_required() || f.default.is_some()) {
            if let Some(id) = ambient_default(env, f)? { vals.insert(n.clone(), Value::Int(id)); }
        }
    }
    for h in env.rules.before_create.get(model).into_iter().flatten() { vals = h(env, vals)?; }
    // neutral fallbacks (after hooks, so hooks can distinguish "not provided"): stored computed fields without a
    // registered compute, and required selections without a usable default
    for (n, f) in m.stored_fields() {
        if vals.contains_key(n) { continue; }
        let uncomputed = f.is_computed() && !env.rules.computes.contains_key(&(model.to_string(), n.clone()));
        if uncomputed || (f.is_required() && f.ty == "Selection") {
            match f.ty.as_str() { "Selection" => { if let Some((k, _)) = f.selection_values().into_iter().next() { vals.insert(n.clone(), Value::Text(k)); } } "Float" | "Monetary" | "Integer" if uncomputed => { vals.insert(n.clone(), Value::Int(0)); } _ => {} }
        }
    }
    vals = delegate_create(env, m, vals)?;
    let (mut stored, x2m) = split_vals(m, vals.clone())?;
    // stored computed fields get their first value from rules.computes (after base insert); required check skips them
    for (n, f) in m.stored_fields() {
        // mail.alias.mixin: Odoo's create() override auto-creates the alias record; that override is not ported, so its fk is not enforced
        if f.ty == "Many2one" && f.comodel_name().as_deref() == Some("mail.alias") { continue; }
        if f.is_required() && f.ty != "Boolean" && !f.is_computed() && n != "id" && stored.get(n).map_or(true, |v| v.is_null() || matches!(v, Value::Bool(false))) {
            return Err(OdooError::Required(model.into(), n.clone()));
        }
    }
    let ts = Value::Text(now());
    for (c, v) in [("create_uid", Value::Int(env.uid)), ("write_uid", Value::Int(env.uid)), ("create_date", ts.clone()), ("write_date", ts)] { if !m.fields.contains_key(c) { stored.insert(c.into(), v); } else { stored.entry(c.into()).or_insert(v); } }
    for (_, v) in stored.iter_mut() { if let Value::Bool(_) | Value::List(_) | Value::Map(_) = v { if let Value::List(_) | Value::Map(_) = v { *v = Value::Text(v.to_json().to_string()); } } }
    let cols: Vec<String> = stored.keys().map(|c| env.q(c)).collect();
    let phs: Vec<String> = (1..=stored.len()).map(|i| env.ph(i)).collect();
    let params: Vec<Value> = stored.values().cloned().collect();
    let sql = if env.d().supports_returning() { format!("INSERT INTO {} ({}) VALUES ({}) RETURNING id", env.q(&m.table()), cols.join(","), phs.join(",")) } else { format!("INSERT INTO {} ({}) VALUES ({})", env.q(&m.table()), cols.join(","), phs.join(",")) };
    let id = env.conn.insert(&sql, &params)?;
    apply_x2m(env, m, id, x2m)?;
    recompute(env, m, &[id])?;
    for h in env.rules.after_create.get(model).into_iter().flatten() { h(env, &[id], &vals)?; }
    Ok(id)
}

/// Resolve Odoo's `default=lambda self: self.env.<x>` style many2one defaults generically.
fn ambient_default(env: &Env, f: &FieldDef) -> Result<Option<i64>> {
    let Some(co) = f.comodel_name() else { return Ok(None) };
    if co == "res.company" { if let Some(Value::Int(c)) = env.ctx.get("company_id") { return Ok(Some(*c)); } }   // the session's active company
    let expr = f.default.as_ref().map(|d| d.to_string()).unwrap_or_default();
    if co == "res.users" && (expr.contains("env.uid") || expr.contains("env.user")) { return Ok(Some(env.uid)); }
    // only well-known "ambient" comodels fall back to the first record; business records (partner, user, product...) stay empty
    let hint = expr.to_lowercase();
    let ambient = ["company", "currency", "uom", "categ", "warehouse", "picking_type", "journal", "stage", "location", "calendar", "country"].iter().any(|h| hint.contains(h) || co.contains(h));
    if !ambient || co == "res.partner" || co == "res.users" { return Ok(None); }
    if env.reg.model(&co).is_err() { return Ok(None); }
    let m = env.reg.model(&co)?;
    let dom = if m.fields.get("active").map_or(false, |a| a.is_stored()) { Domain::Term("active".into(), "=".into(), Value::Bool(true)) } else { Domain::True };
    Ok(search(&env.sudo(), &co, &dom, Some("id"), Some(1), 0).ok().and_then(|v| v.first().copied()))
}

pub fn write(env: &Env, model: &str, ids: &[i64], vals: Row) -> Result<()> {
    if ids.is_empty() { return Ok(()); }
    env.check(model, Op::Write)?;
    enforce_ids(env, model, ids, Op::Write)?;
    let m = env.reg.model(model)?;
    let mut vals = vals;
    for h in env.rules.before_write.get(model).into_iter().flatten() { vals = h(env, vals)?; }
    vals = delegate_write(env, m, ids, vals)?;
    let (mut stored, x2m) = split_vals(m, vals.clone())?;
    for (n, f) in m.stored_fields() { if f.is_required() && f.ty != "Boolean" { if let Some(v) = stored.get(n) { if v.is_null() { return Err(OdooError::Required(model.into(), n.clone())); } } } }
    if !m.fields.contains_key("write_uid") { stored.insert("write_uid".into(), Value::Int(env.uid)); }
    if !m.fields.contains_key("write_date") { stored.insert("write_date".into(), Value::Text(now())); }
    for (_, v) in stored.iter_mut() { if let Value::List(_) | Value::Map(_) = v { *v = Value::Text(v.to_json().to_string()); } }
    if !stored.is_empty() {
        let sets: Vec<String> = stored.keys().enumerate().map(|(i, c)| format!("{} = {}", env.q(c), env.ph(i + 1))).collect();
        let mut params: Vec<Value> = stored.values().cloned().collect();
        let n0 = params.len();
        let in_ph: Vec<String> = ids.iter().enumerate().map(|(i, _)| env.ph(n0 + i + 1)).collect();
        params.extend(ids.iter().map(|i| Value::Int(*i)));
        let n = env.conn.execute(&format!("UPDATE {} SET {} WHERE id IN ({})", env.q(&m.table()), sets.join(", "), in_ph.join(",")), &params)?;
        if (n as usize) < ids.len() { let found = count_ids(env, m, ids)?; if found < ids.len() { if std::env::var("ODOO_DEBUG").is_ok() { eprintln!("WRITE-NOTFOUND {model} {ids:?} n={n} found={found} stored={:?}", stored.keys().collect::<Vec<_>>()); } return Err(OdooError::NotFound(model.into(), ids[0])); } }
    }
    for id in ids { apply_x2m(env, m, *id, x2m.clone())?; }
    recompute(env, m, ids)?;
    for h in env.rules.after_write.get(model).into_iter().flatten() { h(env, ids, &vals)?; }
    Ok(())
}

/// Fields delegated through `_inherits` are `related="<fk>.<name>"`, non-stored. Returns (fk, parent_name) -> keys.
fn delegated<'m>(m: &'m ModelDef, vals: &Row) -> Vec<(String, String, Vec<(String, String)>)> {
    // group by first-hop many2one: editable single-hop `related="fk.field"` fields write through to the comodel
    let mut groups: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for k in vals.keys() {
        let Some(f) = m.fields.get(k) else { continue };
        if f.readonly.as_ref().and_then(|r| r.as_bool()) != Some(false) { continue; } // related is readonly unless explicitly editable
        let Some(r) = f.related.as_ref().and_then(|r| r.as_str()) else { continue };
        let Some((fk, rest)) = r.split_once('.') else { continue };
        if rest.contains('.') { continue; }
        if m.fields.get(fk).map_or(true, |x| x.ty != "Many2one") { continue; }
        groups.entry(fk.to_string()).or_default().push((k.clone(), rest.to_string()));
    }
    groups.into_iter().filter_map(|(fk, keys)| Some((fk.clone(), m.fields.get(&fk)?.comodel_name()?, keys))).collect()
}

/// `_inherits` create: create the parent record from delegated vals first, then link via the FK.
/// A parent is also created when no delegated field was given, and parent fields the parent requires (e.g. `name` when the child
/// redefines it as its own computed field) are taken from the child's vals.
fn delegate_create(env: &Env, m: &ModelDef, mut vals: Row) -> Result<Row> {
    let mut groups = delegated(m, &vals);
    for (parent, fk) in &m.inherits { if !groups.iter().any(|g| &g.0 == fk) { groups.push((fk.clone(), parent.clone(), vec![])); } }
    for (fk, parent, keys) in groups {
        if vals.get(&fk).map_or(false, |v| !v.is_null()) && keys.is_empty() { continue; }
        let mut pv = Row::new();
        for (k, pf) in &keys { let stored = m.fields[k].is_stored(); let v = if stored { vals.get(k).cloned() } else { vals.remove(k) }; if let Some(v) = v { pv.insert(pf.clone(), v); } }
        if vals.get(&fk).map_or(true, |v| v.is_null()) {
            if let Ok(pm) = env.reg.model(&parent) { for (rf, f) in &pm.fields { if f.is_required() && f.is_stored() && !pv.contains_key(rf) { if let Some(v) = vals.get(rf) { pv.insert(rf.clone(), v.clone()); } } } }
            let pid = create(env, &parent, pv)?; vals.insert(fk, Value::Int(pid));
        }
        else if !pv.is_empty() { if let Some(pid) = vals.get(&fk).and_then(|v| v.as_i64()) { write(env, &parent, &[pid], pv)?; } }
    }
    Ok(vals)
}

fn delegate_write(env: &Env, m: &ModelDef, ids: &[i64], mut vals: Row) -> Result<Row> {
    for (fk, parent, keys) in delegated(m, &vals) {
        if keys.is_empty() { continue; }
        let mut pv = Row::new();
        for (k, pf) in &keys { let stored = m.fields[k].is_stored(); let v = if stored { vals.get(k).cloned() } else { vals.remove(k) }; if let Some(v) = v { pv.insert(pf.clone(), v); } }
        let ph: Vec<String> = (1..=ids.len()).map(|i| env.ph(i)).collect();
        let pids: Vec<i64> = env.conn.query(&format!("SELECT {} AS p FROM {} WHERE id IN ({})", env.q(&fk), env.q(&m.table()), ph.join(",")), &ids.iter().map(|i| Value::Int(*i)).collect::<Vec<_>>())?.iter().filter_map(|r| r["p"].as_i64()).collect::<BTreeSet<_>>().into_iter().collect();
        write(env, &parent, &pids, pv)?;
    }
    Ok(vals)
}

fn count_ids(env: &Env, m: &ModelDef, ids: &[i64]) -> Result<usize> {
    let ph: Vec<String> = (1..=ids.len()).map(|i| env.ph(i)).collect();
    let r = env.conn.query(&format!("SELECT COUNT(*) AS c FROM {} WHERE id IN ({})", env.q(&m.table()), ph.join(",")), &ids.iter().map(|i| Value::Int(*i)).collect::<Vec<_>>())?;
    Ok(r.first().and_then(|r| r.get("c")).and_then(|v| v.as_i64()).unwrap_or(0) as usize)
}

pub fn unlink(env: &Env, model: &str, ids: &[i64]) -> Result<()> {
    if ids.is_empty() { return Ok(()); }
    env.check(model, Op::Unlink)?;
    enforce_ids(env, model, ids, Op::Unlink)?;
    let m = env.reg.model(model)?;
    let mut deps = vec![];
    for h in env.rules.unlink_dependents.get(model).into_iter().flatten() { deps.extend(h(env, ids)?); }
    let ph: Vec<String> = (1..=ids.len()).map(|i| env.ph(i)).collect();
    let params: Vec<Value> = ids.iter().map(|i| Value::Int(*i)).collect();
    // cascade: o2m children whose inverse m2o is ondelete=cascade; m2m link rows; set-null elsewhere
    for cm in env.reg.models.values().filter(|c| c.has_table()) {
        for (n, f) in cm.stored_fields() {
            if f.ty == "Many2one" && f.comodel_name().as_deref() == Some(model) {
                match f.ondelete.as_ref().and_then(|v| v.as_str()) {
                    Some("cascade") => { let kids: Vec<i64> = env.conn.query(&format!("SELECT id FROM {} WHERE {} IN ({})", env.q(&cm.table()), env.q(n), ph.join(",")), &params)?.iter().filter_map(|r| r["id"].as_i64()).collect(); unlink(&env.sudo(), &cm.name, &kids)?; }
                    Some("restrict") => { let c = env.conn.query(&format!("SELECT COUNT(*) AS c FROM {} WHERE {} IN ({})", env.q(&cm.table()), env.q(n), ph.join(",")), &params)?; if c[0]["c"].as_i64().unwrap_or(0) > 0 { return Err(OdooError::User(format!("cannot delete {model}: referenced by {}.{n}", cm.name))); } }
                    _ => { env.conn.execute(&format!("UPDATE {} SET {} = NULL WHERE {} IN ({})", env.q(&cm.table()), env.q(n), env.q(n), ph.join(",")), &params)?; }
                }
            }
        }
    }
    for (n, f) in &m.fields { if let Some((rt, c1, _)) = f.relation_table_name(model, n, env.reg).filter(|_| f.ty == "Many2many") { env.conn.execute(&format!("DELETE FROM {} WHERE {} IN ({})", env.q(&rt), env.q(&c1), ph.join(",")), &params)?; } }
    env.conn.execute(&format!("DELETE FROM {} WHERE id IN ({})", env.q(&m.table()), ph.join(",")), &params)?;
    for (dm, dids) in deps { recompute_ids(env, &dm, &dids)?; }
    Ok(())
}

pub fn recompute_ids(env: &Env, model: &str, ids: &[i64]) -> Result<()> { recompute(env, env.reg.model(model)?, ids) }

/// Odoo x2many command tuples: (0,0,vals) (1,id,vals) (2,id) (3,id) (4,id) (5) (6,0,ids)
fn apply_x2m(env: &Env, m: &ModelDef, id: i64, cmds: Vec<(String, &FieldDef, Value)>) -> Result<()> {
    for (name, f, v) in cmds {
        let Value::List(list) = v else { continue };
        let co = f.comodel_name().ok_or_else(|| OdooError::Validation(format!("{name}: no comodel")))?;
        let cmds: Vec<Vec<Value>> = if list.iter().all(|x| matches!(x, Value::Int(_))) && !list.is_empty() { vec![vec![Value::Int(6), Value::Int(0), Value::List(list.clone())]] } else { list.iter().filter_map(|c| if let Value::List(l) = c { Some(l.clone()) } else { None }).collect() };
        for c in cmds {
            let code = c.first().and_then(|v| v.as_i64()).unwrap_or(-1);
            let arg = |i: usize| c.get(i).and_then(|v| v.as_i64());
            if f.ty == "One2many" {
                let inv = f.inverse().ok_or_else(|| OdooError::Validation(format!("{name}: o2m without inverse")))?;
                match code {
                    0 => { if let Some(Value::Map(vals)) = c.get(2) { let mut vals = vals.clone(); vals.insert(inv.clone(), Value::Int(id)); create(env, &co, vals)?; } }
                    1 => { if let (Some(i), Some(Value::Map(vals))) = (arg(1), c.get(2)) { write(env, &co, &[i], vals.clone())?; } }
                    2 | 3 => { if let Some(i) = arg(1) { if code == 2 || f.ondelete.is_some() { unlink(env, &co, &[i])?; } else { write(env, &co, &[i], [(inv.clone(), Value::Null)].into())?; } } }
                    4 => { if let Some(i) = arg(1) { write(env, &co, &[i], [(inv.clone(), Value::Int(id))].into())?; } }
                    5 => { let kids: Vec<i64> = env.conn.query(&format!("SELECT id FROM {} WHERE {} = {}", env.q(&env.reg.table(&co)), env.q(&inv), env.ph(1)), &[Value::Int(id)])?.iter().filter_map(|r| r["id"].as_i64()).collect(); unlink(env, &co, &kids)?; }
                    6 => { if let Some(Value::List(ids)) = c.get(2) { for i in ids.iter().filter_map(|v| v.as_i64()) { write(env, &co, &[i], [(inv.clone(), Value::Int(id))].into())?; } } }
                    _ => {}
                }
            } else if let Some((rt, c1, c2)) = f.relation_table_name(&m.name, &name, env.reg) {
                let link = |other: i64| -> Result<()> { let ins = match env.d().name() { "mysql" => "INSERT IGNORE INTO", _ => "INSERT INTO" }; let tail = if env.d().name() == "mysql" { "" } else { " ON CONFLICT DO NOTHING" }; env.conn.execute(&format!("{ins} {} ({}, {}) VALUES ({}, {}){tail}", env.q(&rt), env.q(&c1), env.q(&c2), env.ph(1), env.ph(2)), &[Value::Int(id), Value::Int(other)]).map(|_| ()) };
                match code {
                    0 => { if let Some(Value::Map(vals)) = c.get(2) { let n = create(env, &co, vals.clone())?; link(n)?; } }
                    2 | 3 => { if let Some(i) = arg(1) { env.conn.execute(&format!("DELETE FROM {} WHERE {} = {} AND {} = {}", env.q(&rt), env.q(&c1), env.ph(1), env.q(&c2), env.ph(2)), &[Value::Int(id), Value::Int(i)])?; if code == 2 { unlink(env, &co, &[i])?; } } }
                    4 => { if let Some(i) = arg(1) { link(i)?; } }
                    5 | 6 => { env.conn.execute(&format!("DELETE FROM {} WHERE {} = {}", env.q(&rt), env.q(&c1), env.ph(1)), &[Value::Int(id)])?; if let Some(Value::List(ids)) = c.get(2) { for i in ids.iter().filter_map(|v| v.as_i64()) { link(i)?; } } }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

/// Recompute registered stored-compute fields for `ids` (full-record recompute; dependency pruning is a perf TODO).
fn recompute(env: &Env, m: &ModelDef, ids: &[i64]) -> Result<()> {
    let fields: Vec<(&String, Compute)> = m.stored_fields().filter_map(|(n, _)| env.rules.computes.get(&(m.name.clone(), n.clone())).map(|c| (n, *c))).collect();
    // stored `related` fields without a ported compute are filled by following their path (193 such fields in Odoo 18)
    let related: Vec<(&String, String)> = m.stored_fields().filter(|(n, f)| f.compute.is_none() && !env.rules.computes.contains_key(&(m.name.clone(), (*n).clone()))).filter_map(|(n, f)| f.related.as_ref().and_then(|r| r.as_str()).map(|r| (n, r.to_string()))).collect();
    if fields.is_empty() && related.is_empty() { return Ok(()); }
    for id in ids {
        let mut sets = Row::new();
        if !fields.is_empty() {
            let rec = browse_raw(env, m, *id)?;
            for (n, c) in &fields { sets.insert((*n).clone(), c(env, &rec)?); }
        }
        for (n, path) in &related {
            // unresolvable paths (x2many hops, missing comodels) are skipped rather than failing the write
            if let Ok(v) = related_values(&env.sudo(), &m.name, &[*id], path) { if let Some(val) = v.get(id) { if !matches!(val, Value::List(_)) || m.fields[*n].ty == "Many2one" { sets.insert((*n).clone(), match val { Value::List(l) if m.fields[*n].ty == "Many2one" => l.first().cloned().unwrap_or(Value::Null), Value::Bool(false) => Value::Null, o => o.clone() }); } } }
        }
        if sets.is_empty() { continue; }
        let cols: Vec<String> = sets.keys().enumerate().map(|(i, c)| format!("{} = {}", env.q(c), env.ph(i + 1))).collect();
        let mut p: Vec<Value> = sets.values().cloned().collect(); p.push(Value::Int(*id));
        env.conn.execute(&format!("UPDATE {} SET {} WHERE id = {}", env.q(&m.table()), cols.join(", "), env.ph(p.len())), &p)?;
    }
    Ok(())
}

/// Raw stored columns plus x2many ids, no access check — what compute functions receive.
pub fn browse_raw(env: &Env, m: &ModelDef, id: i64) -> Result<Row> {
    let mut r = env.conn.query(&format!("SELECT * FROM {} WHERE id = {}", env.q(&m.table()), env.ph(1)), &[Value::Int(id)])?;
    let mut row = r.pop().ok_or_else(|| { if std::env::var("ODOO_DEBUG").is_ok() { eprintln!("{}", std::backtrace::Backtrace::force_capture()); } OdooError::NotFound(m.name.clone(), id) })?;
    for (n, f) in m.fields.iter().filter(|(_, f)| f.is_x2many()) { row.insert(n.clone(), Value::List(x2m_ids(env, m, n, f, &[id])?.remove(&id).unwrap_or_default().into_iter().map(Value::Int).collect())); }
    for (n, f) in &m.fields { if f.ty == "Boolean" { if let Some(v) = row.get_mut(n) { if let Value::Int(i) = v { *v = Value::Bool(*i != 0); } } } }
    Ok(row)
}

fn x2m_ids(env: &Env, m: &ModelDef, name: &str, f: &FieldDef, ids: &[i64]) -> Result<BTreeMap<i64, Vec<i64>>> {
    let co = f.comodel_name().unwrap_or_default();
    let ph: Vec<String> = (1..=ids.len()).map(|i| env.ph(i)).collect();
    let params: Vec<Value> = ids.iter().map(|i| Value::Int(*i)).collect();
    if co.is_empty() { return Ok(BTreeMap::new()); }  // related/dynamic x2many without a static comodel
    let (sql, kc) = if f.ty == "One2many" {
        let inv = f.inverse().unwrap_or_default();
        // inverse must be a real column on the comodel (it may be delegated/related, e.g. via _inherits)
        if !env.reg.models.get(&co).and_then(|c| c.fields.get(&inv)).map_or(false, |x| x.is_stored()) { return Ok(BTreeMap::new()); }
        (format!("SELECT id AS rid, {} AS owner FROM {} WHERE {} IN ({}) ORDER BY id", env.q(&inv), env.q(&env.reg.table(&co)), env.q(&inv), ph.join(",")), "owner")
    } else {
        let (rt, c1, c2) = f.relation_table_name(&m.name, name, env.reg).unwrap();
        (format!("SELECT {} AS rid, {} AS owner FROM {} WHERE {} IN ({})", env.q(&c2), env.q(&c1), env.q(&rt), env.q(&c1), ph.join(",")), "owner")
    };
    let mut out: BTreeMap<i64, Vec<i64>> = BTreeMap::new();
    for r in env.conn.query(&sql, &params)? { if let (Some(o), Some(i)) = (r[kc].as_i64(), r["rid"].as_i64()) { out.entry(o).or_default().push(i); } }
    Ok(out)
}

pub fn display_names(env: &Env, model: &str, ids: &[i64]) -> Result<BTreeMap<i64, String>> {
    if ids.is_empty() { return Ok(BTreeMap::new()); }
    let m = env.reg.model(model)?;
    let ph: Vec<String> = (1..=ids.len()).map(|i| env.ph(i)).collect();
    let params: Vec<Value> = ids.iter().map(|i| Value::Int(*i)).collect();
    let mut out = BTreeMap::new();
    match m.display_field().filter(|f| m.fields[*f].is_stored()) {
        Some(df) => { for r in env.conn.query(&format!("SELECT id, {} AS dn FROM {} WHERE id IN ({})", env.q(df), env.q(&m.table()), ph.join(",")), &params)? { let id = r["id"].as_i64().unwrap_or(0); out.insert(id, match &r["dn"] { Value::Text(s) => s.clone(), Value::Null => format!("{model},{id}"), v => v.to_json().to_string() }); } }
        None => { for i in ids { out.insert(*i, format!("{model},{i}")); } }
    }
    Ok(out)
}

/// Read `fields` (all stored + x2many if empty) for `ids`. Many2one => [id, display_name]; x2many => [ids].
pub fn read(env: &Env, model: &str, ids: &[i64], fields: &[String]) -> Result<Vec<Row>> {
    env.check(model, Op::Read)?;
    let m = env.reg.model(model)?;
    if ids.is_empty() { return Ok(vec![]); }
    if env.rr.domain_ctx(env.uid, model, Op::Read, &env.ctx).is_some() { enforce_ids(env, model, ids, Op::Read)?; }
    // default read = stored + x2many + non-stored fields that have a ported compute (views reference them, e.g. `invisible="hide_x"`)
    let want: Vec<String> = if fields.is_empty() { m.fields.iter().filter(|(n, f)| f.is_stored() || f.is_x2many() || env.rules.computes.contains_key(&(model.to_string(), (*n).clone()))).map(|(n, _)| n.clone()).chain(["id".into()]).collect() } else { fields.to_vec() };
    let stored: Vec<&String> = want.iter().filter(|n| *n == "id" || m.fields.get(*n).map_or(false, |f| f.is_stored())).collect();
    let cols: Vec<String> = std::iter::once("id".to_string()).chain(stored.iter().filter(|n| n.as_str() != "id").map(|n| n.to_string())).map(|c| env.q(&c)).collect();
    let ph: Vec<String> = (1..=ids.len()).map(|i| env.ph(i)).collect();
    let rows = env.conn.query(&format!("SELECT {} FROM {} WHERE id IN ({})", cols.join(","), env.q(&m.table()), ph.join(",")), &ids.iter().map(|i| Value::Int(*i)).collect::<Vec<_>>())?;
    let mut by_id: BTreeMap<i64, Row> = rows.into_iter().filter_map(|r| r["id"].as_i64().map(|i| (i, r))).collect();
    for n in &want {
        let Some(f) = m.fields.get(n) else { if n != "id" { return Err(OdooError::UnknownField(model.into(), n.clone())); } continue };
        match f.ty.as_str() {
            "Many2one" if f.is_stored() => {
                let co = f.comodel_name().unwrap_or_default();
                let tgt: Vec<i64> = by_id.values().filter_map(|r| r.get(n).and_then(|v| v.as_i64())).collect::<BTreeSet<_>>().into_iter().collect();
                let names = display_names(&env.sudo(), &co, &tgt)?;
                for r in by_id.values_mut() { let v = match r.get(n).and_then(|v| v.as_i64()) { Some(i) => Value::List(vec![Value::Int(i), Value::Text(names.get(&i).cloned().unwrap_or_default())]), None => Value::Bool(false) }; r.insert(n.clone(), v); }
            }
            "One2many" | "Many2many" => { let map = x2m_ids(env, m, n, f, ids)?; for (id, r) in by_id.iter_mut() { r.insert(n.clone(), Value::List(map.get(id).cloned().unwrap_or_default().into_iter().map(Value::Int).collect())); } }
            "Boolean" => for r in by_id.values_mut() { if let Some(Value::Int(i)) = r.get(n).cloned() { r.insert(n.clone(), Value::Bool(i != 0)); } },
            "Json" | "Properties" => for r in by_id.values_mut() { if let Some(Value::Text(s)) = r.get(n).cloned() { if let Ok(j) = serde_json::from_str::<serde_json::Value>(&s) { r.insert(n.clone(), Value::from_json(&j)); } } },
            _ => for r in by_id.values_mut() { if matches!(r.get(n), Some(Value::Null)) { r.insert(n.clone(), Value::Bool(false)); } },
        }
    }
    // non-stored related fields: walk the dotted path with sudo reads
    for n in &want {
        let Some(f) = m.fields.get(n) else { continue };
        if f.is_stored() || f.is_x2many() { continue; }
        // non-stored computed field with a ported compute: evaluate per record
        if let Some(c) = env.rules.computes.get(&(model.to_string(), n.clone())) {
            for (id, r) in by_id.iter_mut() { let raw = browse_raw(&env.sudo(), m, *id)?; r.insert(n.clone(), c(env, &raw)?); }
            continue;
        }
        let Some(path) = f.related.as_ref().and_then(|r| r.as_str()) else { continue };
        let vals = related_values(&env.sudo(), model, ids, path)?;
        for (id, r) in by_id.iter_mut() { r.insert(n.clone(), vals.get(id).cloned().unwrap_or(Value::Bool(false))); }
    }
    Ok(ids.iter().filter_map(|i| by_id.remove(i)).collect())
}

/// Resolve `a.b.c` for each id by chaining reads along many2one hops.
fn related_values(env: &Env, model: &str, ids: &[i64], path: &str) -> Result<BTreeMap<i64, Value>> {
    let (head, rest) = match path.split_once('.') { Some((h, r)) => (h, Some(r)), None => (path, None) };
    let rows = read(env, model, ids, &[head.to_string()])?;
    let Some(rest) = rest else { return Ok(rows.into_iter().filter_map(|r| Some((r["id"].as_i64()?, r.get(head).cloned().unwrap_or(Value::Null)))).collect()) };
    let co = env.reg.field(model, head)?.comodel_name().ok_or_else(|| OdooError::Validation(format!("related path {path}: {head} has no comodel")))?;
    let link: BTreeMap<i64, i64> = rows.iter().filter_map(|r| { let id = r["id"].as_i64()?; let t = match r.get(head)? { Value::List(l) => l.first()?.as_i64()?, _ => return None }; Some((id, t)) }).collect();
    let tids: Vec<i64> = link.values().cloned().collect::<BTreeSet<_>>().into_iter().collect();
    let sub = related_values(env, &co, &tids, rest)?;
    Ok(link.into_iter().filter_map(|(id, t)| sub.get(&t).map(|v| (id, v.clone()))).collect())
}

fn order_sql(env: &Env, m: &ModelDef, order: Option<&str>, alias: &str) -> Result<String> {
    let spec = order.or(m.order.as_deref()).unwrap_or("id");
    let mut parts = vec![];
    for p in spec.split(',') {
        let mut it = p.split_whitespace();
        let Some(f) = it.next() else { continue };
        let dir = match it.next().map(|s| s.to_lowercase()) { Some(d) if d == "desc" => "DESC", Some(d) if d == "asc" => "ASC", None => "ASC", Some(d) => return Err(OdooError::Domain(format!("bad order direction {d}"))) };
        if f != "id" && !m.fields.get(f).map_or(false, |x| x.is_stored()) { continue; } // non-stored order keys ignored (Odoo would error)
        parts.push(format!("{alias}.{} {dir}", env.q(f)));
    }
    if !parts.iter().any(|p| p.contains(".\"id\"") || p.contains(".`id`")) { parts.push(format!("{alias}.{} ASC", env.q("id"))); }
    Ok(parts.join(", "))
}

pub fn search(env: &Env, model: &str, domain: &Domain, order: Option<&str>, limit: Option<usize>, offset: usize) -> Result<Vec<i64>> {
    env.check(model, Op::Read)?;
    let m = env.reg.model(model)?;
    let mut params = vec![];
    let w = compile(env.reg, env.d(), model, "t", &scoped(env, model, Op::Read, &with_active(m, domain, env)), &mut params)?;
    let lim = limit.map(|l| format!(" LIMIT {l} OFFSET {offset}")).unwrap_or_else(|| if offset > 0 { format!(" LIMIT -1 OFFSET {offset}").replace("-1", &i64::MAX.to_string()) } else { String::new() });
    let rows = env.conn.query(&format!("SELECT t.id FROM {} t WHERE {w} ORDER BY {}{lim}", env.q(&m.table()), order_sql(env, m, order, "t")?), &params)?;
    Ok(rows.iter().filter_map(|r| r["id"].as_i64()).collect())
}

pub fn search_count(env: &Env, model: &str, domain: &Domain) -> Result<i64> {
    env.check(model, Op::Read)?;
    let m = env.reg.model(model)?;
    let mut params = vec![];
    let w = compile(env.reg, env.d(), model, "t", &scoped(env, model, Op::Read, &with_active(m, domain, env)), &mut params)?;
    Ok(env.conn.query(&format!("SELECT COUNT(*) AS c FROM {} t WHERE {w}", env.q(&m.table())), &params)?.first().and_then(|r| r["c"].as_i64()).unwrap_or(0))
}

/// Odoo's implicit `active = True` filter unless the domain mentions `active` or ctx.active_test is false.
fn with_active(m: &ModelDef, d: &Domain, env: &Env) -> Domain {
    fn mentions(d: &Domain) -> bool { match d { Domain::Term(f, ..) => f == "active", Domain::And(v) | Domain::Or(v) => v.iter().any(mentions), Domain::Not(x) => mentions(x), _ => false } }
    let off = matches!(env.ctx.get("active_test"), Some(Value::Bool(false)));
    if m.fields.get("active").map_or(false, |f| f.is_stored()) && !mentions(d) && !off { d.clone().and(Domain::Term("active".into(), "=".into(), Value::Bool(true))) } else { d.clone() }
}

pub fn search_read(env: &Env, model: &str, domain: &Domain, fields: &[String], order: Option<&str>, limit: Option<usize>, offset: usize) -> Result<Vec<Row>> {
    let ids = search(env, model, domain, order, limit, offset)?;
    read(env, model, &ids, fields)
}

pub fn name_search(env: &Env, model: &str, name: &str, limit: usize) -> Result<Vec<(i64, String)>> {
    let m = env.reg.model(model)?;
    let dom = match m.display_field() { Some(f) if !name.is_empty() => Domain::Term(f.into(), "ilike".into(), Value::Text(name.into())), _ => Domain::True };
    let ids = search(env, model, &dom, None, Some(limit), 0)?;
    Ok(display_names(env, model, &ids)?.into_iter().collect())
}

/// Dispatch a ported business method (button/action) registered in `Rules.actions`.
pub fn call(env: &Env, model: &str, method: &str, ids: &[i64], args: &Row) -> Result<Value> {
    match env.rules.actions.get(&(model.to_string(), method.to_string())) {
        Some(a) => a(env, ids, args),
        None => generic_method(env, model, method, ids, args).unwrap_or_else(|| Err(OdooError::User(format!("method {model}.{method} is not ported yet")))),
    }
}

/// `fields_get` equivalent for UI generation.
pub fn fields_get(reg: &Registry, model: &str) -> Result<serde_json::Value> {
    let m = reg.model(model)?;
    let mut out = serde_json::Map::new();
    for (n, f) in &m.fields {
        out.insert(n.clone(), serde_json::json!({
            "type": f.ty.to_lowercase().replace("many2one", "many2one"), "string": f.label(n), "required": f.is_required(),
            "readonly": f.is_readonly(), "relation": f.comodel_name(), "relation_field": f.inverse(),
            "selection": f.selection_values(), "store": f.is_stored(), "searchable": f.is_stored() || crate::domain::VIRTUAL_FALSE.contains(&n.as_str()) || (n == "search_date_category" && crate::domain::DATE_CATEGORY.iter().any(|(mm, _)| *mm == model)) || f.related.is_some() && !f.is_x2many(),
        }));
    }
    Ok(serde_json::Value::Object(out))
}

/// `read_group` (single groupby): counts and sums of numeric stored fields per group, with a drill-down domain.
pub fn read_group(env: &Env, model: &str, domain: &Domain, fields: &[String], groupby: &str, order: Option<&str>) -> Result<Vec<Row>> {
    env.check(model, Op::Read)?;
    let m = env.reg.model(model)?;
    let gf = m.fields.get(groupby).filter(|f| f.is_stored()).ok_or_else(|| OdooError::UnknownField(model.into(), groupby.into()))?;
    let (gname, _) = groupby.split_once(':').map(|(a, b)| (a, b)).unwrap_or((groupby, ""));
    let mut params = vec![];
    let w = compile(env.reg, env.d(), model, "t", &scoped(env, model, Op::Read, &with_active(m, domain, env)), &mut params)?;
    let aggs: Vec<String> = fields.iter().filter_map(|f| { let n = f.split(':').next()?; let fd = m.fields.get(n)?; if fd.is_stored() && matches!(fd.ty.as_str(), "Integer" | "Float" | "Monetary") { Some(n.to_string()) } else { None } }).collect();
    let sel: Vec<String> = aggs.iter().map(|a| format!("SUM(t.{q}) AS {q}", q = env.q(a))).collect();
    let g = format!("t.{}", env.q(gname));
    let ord = match order { Some(o) if o.starts_with(gname) => o.to_string(), _ => format!("{gname} ASC") };
    let sql = format!("SELECT {g} AS grp, COUNT(*) AS cnt{} FROM {} t WHERE {w} GROUP BY {g} ORDER BY {}", if sel.is_empty() { String::new() } else { format!(", {}", sel.join(", ")) }, env.q(&m.table()), ord.replace(gname, "grp"));
    let rows = env.conn.query(&sql, &params)?;
    let m2o: BTreeMap<i64, String> = if gf.ty == "Many2one" { let ids: Vec<i64> = rows.iter().filter_map(|r| r["grp"].as_i64()).collect(); display_names(&env.sudo(), &gf.comodel_name().unwrap_or_default(), &ids)? } else { BTreeMap::new() };
    Ok(rows.into_iter().map(|r| {
        let mut o = Row::new();
        let gv = match (&gf.ty[..], r.get("grp")) { ("Many2one", Some(Value::Int(i))) => Value::List(vec![Value::Int(*i), Value::Text(m2o.get(i).cloned().unwrap_or_default())]), (_, Some(Value::Null)) | (_, None) => Value::Bool(false), (_, Some(v)) => v.clone() };
        let dom = match r.get("grp") { Some(Value::Null) | None => Value::List(vec![Value::Text(gname.into()), Value::Text("=".into()), Value::Bool(false)]), Some(v) => Value::List(vec![Value::Text(gname.into()), Value::Text("=".into()), v.clone()]) };
        o.insert(groupby.to_string(), gv);
        o.insert(format!("{gname}_count"), r["cnt"].clone()); o.insert("__count".into(), r["cnt"].clone());
        o.insert("__domain".into(), Value::List(vec![dom]));
        for a in &aggs { o.insert(a.clone(), r.get(a).cloned().unwrap_or(Value::Float(0.0))); }
        o
    }).collect())
}

/// Defaults for a new record (static, ambient, rule-provided) restricted to `fields` (all stored if empty).
pub fn default_get(env: &Env, model: &str, fields: &[String]) -> Result<Row> {
    let m = env.reg.model(model)?;
    let mut out = Row::new();
    for (n, f) in m.stored_fields() {
        if !fields.is_empty() && !fields.contains(n) { continue; }
        if f.is_computed() || n == "id" { continue; }
        if let Some(d) = env.rules.defaults.get(&(model.to_string(), n.clone())) { out.insert(n.clone(), d(env)?); }
        else if let Some(d) = f.static_default() { out.insert(n.clone(), json_default(&d)); }
        else if f.ty == "Many2one" && f.is_required() { if let Some(id) = ambient_default(env, f)? { let nm = display_names(&env.sudo(), &f.comodel_name().unwrap_or_default(), &[id])?; out.insert(n.clone(), Value::List(vec![Value::Int(id), Value::Text(nm.get(&id).cloned().unwrap_or_default())])); } }
        else if f.default.is_some() && matches!(f.ty.as_str(), "Date" | "Datetime") { out.insert(n.clone(), Value::Text(if f.ty == "Date" { today() } else { now() })); }
    }
    Ok(out)
}

/// Run form-time hooks against unsaved values (many2one given as ids or [id, name]); returns changed values.
pub fn onchange(env: &Env, model: &str, vals: Row) -> Result<Row> {
    let mut cur = vals.clone();
    for (k, v) in cur.iter_mut() { if let (Some(f), Value::List(l)) = (env.reg.model(model)?.fields.get(k), v.clone()) { if f.ty == "Many2one" { *v = l.first().cloned().unwrap_or(Value::Null); } } }
    for h in env.rules.onchange.get(model).into_iter().flatten() { cur = h(env, cur)?; }
    Ok(cur.into_iter().filter(|(k, v)| vals.get(k) != Some(v)).collect())
}

/// Methods every Odoo model inherits from BaseModel; `None` = not a generic method.
fn generic_method(env: &Env, model: &str, method: &str, ids: &[i64], args: &Row) -> Option<Result<Value>> {
    let m = env.reg.model(model).ok()?;
    let has_active = m.fields.get("active").map_or(false, |f| f.is_stored());
    Some(match method {
        "action_archive" | "action_unarchive" | "toggle_active" if has_active => (|| {
            let e = env.with_ctx("active_test", Value::Bool(false));
            for r in read(&e, model, ids, &["active".to_string()])? {
                let id = r["id"].as_i64().unwrap_or(0);
                let cur = r.get("active").map_or(true, |v| v.truthy());
                let next = match method { "action_archive" => false, "action_unarchive" => true, _ => !cur };
                write(&e, model, &[id], [("active".to_string(), Value::Bool(next))].into())?;
            }
            Ok(Value::Bool(true))
        })(),
        "exists" => (|| {
            let e = env.with_ctx("active_test", Value::Bool(false));
            let found = search(&e, model, &Domain::Term("id".into(), "in".into(), Value::List(ids.iter().map(|i| Value::Int(*i)).collect())), None, None, 0)?;
            Ok(Value::List(found.into_iter().map(Value::Int).collect()))
        })(),
        "copy" => (|| { let mut out = vec![]; for id in ids { out.push(Value::Int(copy_record(env, model, *id, args)?)); } Ok(Value::List(out)) })(),
        "name_create" => (|| {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let rn = m.display_field().ok_or_else(|| OdooError::User(format!("{model} has no name field")))?.to_string();
            let id = create(env, model, [(rn, Value::Text(name.clone()))].into())?;
            Ok(Value::List(vec![Value::Int(id), Value::Text(name)]))
        })(),
        _ => return None,
    })
}

/// Duplicate a record honoring field `copy` flags; many2many links are copied, one2many only when `copy=True`.
pub fn copy_record(env: &Env, model: &str, id: i64, defaults: &Row) -> Result<i64> {
    let m = env.reg.model(model)?;
    let src = browse_raw(&env.sudo(), m, id)?;
    let mut vals = Row::new();
    for (n, f) in &m.fields {
        if n == "id" || !f.is_copyable() || (!f.is_stored() && !f.is_x2many()) || matches!(n.as_str(), "create_uid" | "create_date" | "write_uid" | "write_date") { continue; }
        let Some(v) = src.get(n) else { continue };
        if v.is_null() { continue; }
        match f.ty.as_str() {
            "Many2many" => if let Value::List(l) = v { if !l.is_empty() { vals.insert(n.clone(), Value::List(vec![Value::List(vec![Value::Int(6), Value::Int(0), Value::List(l.clone())])])); } },
            "One2many" => if let (Value::List(l), Some(co), Some(inv)) = (v, f.comodel_name(), f.inverse()) {
                let cm = env.reg.model(&co)?;
                let mut cmds = vec![];
                for cid in l.iter().filter_map(|x| x.as_i64()) {
                    let mut child = browse_raw(&env.sudo(), cm, cid)?;
                    child.remove("id"); child.remove(&inv);
                    let cvals: Row = child.into_iter().filter(|(k, v)| !v.is_null() && cm.fields.get(k).map_or(false, |cf| cf.is_stored() && cf.is_copyable() && !matches!(k.as_str(), "create_uid" | "create_date" | "write_uid" | "write_date"))).collect();
                    cmds.push(Value::List(vec![Value::Int(0), Value::Int(0), Value::Map(cvals.into_iter().collect())]));
                }
                if !cmds.is_empty() { vals.insert(n.clone(), Value::List(cmds)); }
            },
            _ => { vals.insert(n.clone(), match (f.ty.as_str(), v) { ("Boolean", Value::Int(i)) => Value::Bool(*i != 0), (_, v) => v.clone() }); }
        }
    }
    // `copy` of a many2one-dependent default (e.g. sequence-named records) is handled by before_create hooks; unique-ish names get Odoo's suffix
    for (k, v) in defaults { vals.insert(k.clone(), v.clone()); }
    create(env, model, vals)
}

#[cfg(test)]
mod date_tests {
    use super::{epoch_days, shift_date};
    #[test] fn epoch_days_matches_known_dates() { assert_eq!(epoch_days("1970-01-01"), 0); assert_eq!(epoch_days("2026-10-05"), 20731); assert_eq!(epoch_days("2026-10-20 08:00:00") - epoch_days("2026-10-05"), 15); assert_eq!(epoch_days(&shift_date("2026-03-01", -1)), epoch_days("2026-03-01") - 1); }
    #[test] fn shifts_across_month_year_leap() {
        assert_eq!(shift_date("2026-10-05", 1), "2026-10-06"); assert_eq!(shift_date("2026-10-31", 1), "2026-11-01");
        assert_eq!(shift_date("2026-12-31", 1), "2027-01-01"); assert_eq!(shift_date("2026-03-01", -1), "2026-02-28");
        assert_eq!(shift_date("2028-03-01", -1), "2028-02-29"); assert_eq!(shift_date("2026-10-05", -280), "2025-12-29"); assert_eq!(shift_date("2026-10-05", 0), "2026-10-05");
    }
}
