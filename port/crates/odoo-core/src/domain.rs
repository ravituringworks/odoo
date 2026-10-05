//! Odoo domains as an algebraic data type, with a pure compiler to parameterised SQL.
use crate::dialect::Dialect;
use crate::error::{OdooError, Result};
use crate::schema::Registry;
use crate::value::Value;

#[derive(Clone, Debug, PartialEq)]
pub enum Domain {
    True,
    False,
    Term(String, String, Value),
    And(Vec<Domain>),
    Or(Vec<Domain>),
    Not(Box<Domain>),
}

impl Domain {
    /// Parse Odoo prefix notation: `["&", ["a","=",1], "|", ...]`; implicit AND between siblings.
    pub fn parse(v: &serde_json::Value) -> Result<Domain> {
        let items = v.as_array().ok_or_else(|| OdooError::Domain("domain must be a list".into()))?;
        let mut pos = 0;
        let mut terms = vec![];
        while pos < items.len() { terms.push(Self::parse_one(items, &mut pos)?); }
        Ok(match terms.len() { 0 => Domain::True, 1 => terms.pop().unwrap(), _ => Domain::And(terms) })
    }
    fn parse_one(items: &[serde_json::Value], pos: &mut usize) -> Result<Domain> {
        let it = items.get(*pos).ok_or_else(|| OdooError::Domain("truncated domain".into()))?;
        *pos += 1;
        match it {
            serde_json::Value::String(op) => match op.as_str() {
                "&" => Ok(Domain::And(vec![Self::parse_one(items, pos)?, Self::parse_one(items, pos)?])),
                "|" => Ok(Domain::Or(vec![Self::parse_one(items, pos)?, Self::parse_one(items, pos)?])),
                "!" => Ok(Domain::Not(Box::new(Self::parse_one(items, pos)?))),
                o => Err(OdooError::Domain(format!("bad operator {o}"))),
            },
            serde_json::Value::Array(t) if t.len() == 3 => {
                let (f, o) = (t[0].as_str(), t[1].as_str());
                match (f, o) {
                    (Some("1"), Some("=")) if t[2].as_i64() == Some(1) => Ok(Domain::True),
                    (Some("0"), Some("=")) if t[2].as_i64() == Some(1) => Ok(Domain::False),
                    (Some(f), Some(o)) => Ok(Domain::Term(f.into(), o.to_lowercase(), Value::from_json(&t[2]))),
                    _ => Err(OdooError::Domain(format!("bad leaf {t:?}"))),
                }
            }
            o => Err(OdooError::Domain(format!("bad token {o}"))),
        }
    }

    pub fn and(self, other: Domain) -> Domain {
        match (self, other) { (Domain::True, d) | (d, Domain::True) => d, (Domain::And(mut a), Domain::And(b)) => { a.extend(b); Domain::And(a) } (Domain::And(mut a), d) | (d, Domain::And(mut a)) => { a.push(d); Domain::And(a) } (a, b) => Domain::And(vec![a, b]) }
    }
}

/// Fields that are constantly false in this port (see `term`).
pub const VIRTUAL_FALSE: &[&str] = &["message_needaction", "message_unread", "message_has_error", "message_has_sms_error"];

/// `search_date_category` (stock.picking, mrp.production): a computed bucket of the record's scheduled date relative to today.
pub const DATE_CATEGORY: &[(&str, &str)] = &[("mrp.production", "date_start"), ("stock.picking", "scheduled_date")];

/// `[start, end)` date strings for a bucket (`before`/`yesterday`/`today`/`day_1`/`day_2`/`after`); `None` bound = open.
pub fn date_bucket(cat: &str, today: &str) -> Option<(Option<String>, Option<String>)> {
    let d = |off: i64| crate::orm::shift_date(today, off);
    Some(match cat {
        "before" => (None, Some(d(-1))), "yesterday" => (Some(d(-1)), Some(d(0))), "today" => (Some(d(0)), Some(d(1))),
        "day_1" => (Some(d(1)), Some(d(2))), "day_2" => (Some(d(2)), Some(d(3))), "after" => (Some(d(3)), None),
        _ => return None,
    })
}

pub struct Compiled { pub sql: String, pub params: Vec<Value> }

/// Compile `domain` against `model`, with the model table aliased as `alias`.
pub fn compile(reg: &Registry, d: &dyn Dialect, model: &str, alias: &str, dom: &Domain, params: &mut Vec<Value>) -> Result<String> {
    Ok(match dom {
        Domain::True => "1=1".into(),
        Domain::False => "1=0".into(),
        Domain::And(v) => join(reg, d, model, alias, v, " AND ", params)?,
        Domain::Or(v) => join(reg, d, model, alias, v, " OR ", params)?,
        Domain::Not(x) => format!("NOT ({})", compile(reg, d, model, alias, x, params)?),
        Domain::Term(f, op, val) => term(reg, d, model, alias, f, op, val, params)?,
    })
}

fn join(reg: &Registry, d: &dyn Dialect, m: &str, a: &str, v: &[Domain], sep: &str, p: &mut Vec<Value>) -> Result<String> {
    if v.is_empty() { return Ok("1=1".into()); }
    let parts: Result<Vec<_>> = v.iter().map(|x| compile(reg, d, m, a, x, p)).collect();
    Ok(format!("({})", parts?.join(sep)))
}

fn push(d: &dyn Dialect, p: &mut Vec<Value>, v: Value) -> String { p.push(v); d.placeholder(p.len()) }

#[allow(clippy::too_many_arguments)]
fn term(reg: &Registry, d: &dyn Dialect, model: &str, alias: &str, path: &str, op: &str, val: &Value, p: &mut Vec<Value>) -> Result<String> {
    // Dotted path through many2one / x2many => semi-join on the comodel.
    if let Some((head, rest)) = path.split_once('.') {
        let f = reg.field(model, head)?;
        let co = f.comodel_name().ok_or_else(|| OdooError::Domain(format!("{head} has no comodel")))?;
        let sub = term(reg, d, &co, "s", rest, op, val, p)?;
        return semijoin(reg, d, model, alias, head, &co, Some(sub), p);
    }
    let id_def = crate::schema::FieldDef { ty: "Integer".into(), ..Default::default() };   // implicit primary key
    let f = if path == "id" { &id_def } else { reg.field(model, path)? };
    if path == "search_date_category" && !f.is_stored() {
        let col_name = DATE_CATEGORY.iter().find(|(m, _)| *m == model).map(|(_, c)| *c).ok_or_else(|| OdooError::Domain(format!("{model}.search_date_category is not searchable")))?;
        let (Value::Text(cat), "=") = (val, op) else { return Err(OdooError::Domain("search_date_category supports only `=`".into())) };
        let (lo, hi) = date_bucket(cat, &crate::orm::today()).ok_or_else(|| OdooError::Domain(format!("unknown date category `{cat}`")))?;
        let col = format!("{alias}.{}", d.quote(col_name)); let mut parts = vec![];
        if let Some(l) = lo { parts.push(format!("{col} >= {}", push(d, p, Value::Text(l)))); }
        if let Some(h) = hi { parts.push(format!("{col} < {}", push(d, p, Value::Text(h)))); }
        return Ok(if parts.is_empty() { "1=1".into() } else { format!("({})", parts.join(" AND ")) });
    }
    // mail.thread flags computed from message/notification tables this port does not have: no messages exist, so none is unread/errored
    if VIRTUAL_FALSE.contains(&path) && !f.is_stored() {
        return Ok(match (op, val) {
            ("=", Value::Bool(true)) | ("!=", Value::Bool(false)) => "1=0".into(),
            ("=", Value::Bool(false)) | ("!=", Value::Bool(true)) | ("=", Value::Null) => "1=1".into(),
            _ => return Err(OdooError::Domain(format!("unsupported use of {path}"))),
        });
    }
    // non-stored related fields are searchable by following their path (e.g. product.product.name -> product_tmpl_id.name)
    if !f.is_stored() && !f.is_x2many() {
        if let Some(rel) = f.related.as_ref().and_then(|r| r.as_str()) {
            if rel != path { return term(reg, d, model, alias, rel, op, val, p); }
        }
    }
    let col = if path == "id" { format!("{}.{}", alias, d.quote("id")) } else {
        if !f.is_stored() && !f.is_x2many() { return Err(OdooError::Domain(format!("field {model}.{path} is not stored; cannot search"))); }
        format!("{}.{}", alias, d.quote(path))
    };
    if f.is_x2many() {
        let co = f.comodel_name().unwrap_or_default();
        return match (op, val) {
            ("in", Value::List(ids)) | ("=", Value::List(ids)) if !ids.is_empty() => {
                let ph: Vec<String> = ids.iter().map(|i| push(d, p, i.clone())).collect();
                semijoin_in(reg, d, model, alias, path, &co, &ph.join(","))
            }
            ("=", Value::Int(i)) => { let ph = push(d, p, Value::Int(*i)); semijoin_in(reg, d, model, alias, path, &co, &ph) }
            ("=", Value::Bool(false)) | ("=", Value::Null) => Ok(format!("NOT ({})", semijoin(reg, d, model, alias, path, &co, None, p)?)),
            ("!=", Value::Bool(false)) | ("!=", Value::Null) => semijoin(reg, d, model, alias, path, &co, None, p),
            _ => {
                // name-like on comodel
                let cm = reg.model(&co)?;
                let rn = cm.display_field().ok_or_else(|| OdooError::Domain("no rec_name".into()))?.to_string();
                let sub = term(reg, d, &co, "s", &rn, op, val, p)?;
                semijoin(reg, d, model, alias, path, &co, Some(sub), p)
            }
        };
    }
    if matches!(op, "child_of" | "parent_of") {
        // hierarchy through the comodel's parent field (falls back to plain `in` when there is none)
        let ids: Vec<Value> = match val { Value::List(l) => l.clone(), Value::Null | Value::Bool(false) => vec![], v => vec![v.clone()] };
        if ids.is_empty() { return Ok("1=0".into()); }
        let (co, target_col) = if path == "id" { (model.to_string(), col.clone()) } else { (f.comodel_name().unwrap_or_else(|| model.to_string()), col.clone()) };
        let cm = reg.model(&co)?;
        let parent = cm.parent_name.clone().unwrap_or_else(|| "parent_id".into());
        let phs: Vec<String> = ids.iter().map(|i| push(d, p, i.clone())).collect();
        let (t, pc) = (d.quote(&cm.table()), d.quote(&parent));
        if !cm.fields.get(&parent).map_or(false, |x| x.is_stored()) { return Ok(format!("{target_col} IN ({})", phs.join(","))); }
        let step = if op == "child_of" { format!("SELECT c.id FROM {t} c JOIN h ON c.{pc} = h.id") } else { format!("SELECT c.{pc} FROM {t} c JOIN h ON c.id = h.id WHERE c.{pc} IS NOT NULL") };
        return Ok(format!("{target_col} IN (WITH RECURSIVE h(id) AS (SELECT id FROM {t} WHERE id IN ({}) UNION {step}) SELECT id FROM h)", phs.join(",")));
    }
    Ok(match (op, val) {
        ("=", Value::Null) | ("=", Value::Bool(false)) if f.ty != "Boolean" => format!("{col} IS NULL"),
        ("!=", Value::Null) | ("!=", Value::Bool(false)) if f.ty != "Boolean" => format!("{col} IS NOT NULL"),
        ("=", Value::Bool(false)) => format!("({col} = {} OR {col} IS NULL)", d.bool_lit(false)),
        ("!=", Value::Bool(true)) => format!("({col} != {} OR {col} IS NULL)", d.bool_lit(true)),
        ("=" | "!=" | "<" | ">" | "<=" | ">=", v) => {
            let ph = push(d, p, bind_bool(d, v.clone()));
            if op == "!=" { format!("({col} != {ph} OR {col} IS NULL)") } else { format!("{col} {op} {ph}") }
        }
        ("in" | "not in", Value::List(vs)) => {
            if vs.is_empty() { return Ok(if op == "in" { "1=0".into() } else { "1=1".into() }); }
            let has_false = vs.iter().any(|v| matches!(v, Value::Bool(false) | Value::Null));
            let ph: Vec<String> = vs.iter().filter(|v| !matches!(v, Value::Bool(false) | Value::Null)).map(|v| push(d, p, v.clone())).collect();
            let inl = if ph.is_empty() { None } else { Some(format!("{col} {} ({})", if op == "in" { "IN" } else { "NOT IN" }, ph.join(","))) };
            match (op, inl, has_false) {
                ("in", Some(i), true) => format!("({i} OR {col} IS NULL)"),
                ("in", None, true) => format!("{col} IS NULL"),
                ("not in", Some(i), true) => format!("({i} AND {col} IS NOT NULL)"),
                ("not in", None, true) => format!("{col} IS NOT NULL"),
                ("not in", Some(i), false) => format!("({i} OR {col} IS NULL)"),
                (_, Some(i), _) => i,
                _ => "1=1".into(),
            }
        }
        ("like" | "not like" | "ilike" | "not ilike" | "=like" | "=ilike", v) => {
            let s = v.as_str().map(String::from).unwrap_or_else(|| format!("{}", v.as_f64().unwrap_or(0.0)));
            let pat = if op.starts_with('=') { s } else { format!("%{}%", s.replace('%', "\\%").replace('_', "\\_")) };
            let ph = push(d, p, Value::Text(pat));
            let insens = op.contains("ilike");
            let base = if insens { d.ilike(&col, &ph) } else { format!("{col} LIKE {ph}") };
            if op.starts_with("not") { format!("({col} IS NULL OR NOT ({base}))") } else { base }
        }
        (o, _) => return Err(OdooError::Domain(format!("unsupported operator `{o}` for {model}.{path}"))),
    })
}

fn bind_bool(_d: &dyn Dialect, v: Value) -> Value { v }

/// EXISTS/IN subquery that joins `model.field` to `comodel`, optionally filtered by `cond` (aliased `s`).
#[allow(clippy::too_many_arguments)]
fn semijoin(reg: &Registry, d: &dyn Dialect, model: &str, alias: &str, field: &str, co: &str, cond: Option<String>, _p: &mut Vec<Value>) -> Result<String> {
    let f = reg.field(model, field)?;
    let w = cond.map(|c| format!(" WHERE {c}")).unwrap_or_default();
    let cot = d.quote(&reg.table(co));
    Ok(match f.ty.as_str() {
        "Many2one" => format!("{alias}.{} IN (SELECT s.id FROM {cot} s{w})", d.quote(field)),
        "One2many" => { let inv = f.inverse().ok_or_else(|| OdooError::Domain("o2m without inverse".into()))?; format!("{alias}.id IN (SELECT s.{} FROM {cot} s{w})", d.quote(&inv)) }
        "Many2many" => { let (rt, c1, c2) = f.relation_table_name(model, field, reg).unwrap(); format!("{alias}.id IN (SELECT r.{} FROM {} r JOIN {cot} s ON s.id = r.{}{})", d.quote(&c1), d.quote(&rt), d.quote(&c2), w.replace(" WHERE ", " WHERE ")) }
        t => return Err(OdooError::Domain(format!("cannot traverse {t} field {model}.{field}"))),
    })
}

fn semijoin_in(reg: &Registry, d: &dyn Dialect, model: &str, alias: &str, field: &str, co: &str, phs: &str) -> Result<String> {
    let f = reg.field(model, field)?;
    Ok(match f.ty.as_str() {
        "One2many" => { let inv = f.inverse().ok_or_else(|| OdooError::Domain("o2m without inverse".into()))?; format!("{alias}.id IN (SELECT s.{} FROM {} s WHERE s.id IN ({phs}))", d.quote(&inv), d.quote(&reg.table(co))) }
        _ => { let (rt, c1, c2) = f.relation_table_name(model, field, reg).unwrap(); format!("{alias}.id IN (SELECT r.{} FROM {} r WHERE r.{} IN ({phs}))", d.quote(&c1), d.quote(&rt), d.quote(&c2)) }
    })
}
