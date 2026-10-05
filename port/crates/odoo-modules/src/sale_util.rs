//! Shared helpers for the sale / pricelist / delivery ports: float & UoM & currency arithmetic, group lookups,
//! x2many command builders. Pure functions over `Env`; semantics follow `odoo.tools.float_utils`, `uom.uom` and `res.currency`.
use crate::util::*;
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, OdooError, Result, Row, Value};
use std::collections::BTreeSet;

pub fn has_field(env: &Env, model: &str, f: &str) -> bool { env.reg.field(model, f).is_ok() }
pub fn has_model(env: &Env, model: &str) -> bool { env.reg.models.contains_key(model) }
pub fn idv(o: Option<i64>) -> Value { o.map(Value::Int).unwrap_or(Value::Null) }
/// Insert `val` under `f` only when the field exists in this registry and the value is set.
pub fn put(env: &Env, model: &str, v: &mut Row, f: &str, val: Value) { if !val.is_null() && has_field(env, model, f) { v.insert(f.into(), val); } }
pub fn set6(ids: &[i64]) -> Value { Value::List(vec![Value::List(vec![6.into(), 0.into(), Value::List(ids.iter().map(|i| Value::Int(*i)).collect())])]) }
pub fn link4(id: i64) -> Value { Value::List(vec![Value::List(vec![4.into(), id.into()])]) }
pub fn flag(r: &Row, k: &str) -> bool { r.get(k).map_or(false, |v| v.truthy()) }
pub fn opt_id(r: &Row, k: &str) -> Option<i64> { id_of(r, k).filter(|i| *i != 0) }
pub fn date_part(s: &str) -> String { s.chars().take(10).collect() }

/// `odoo.tools.float_round` for the rounding methods used in sale flows ("HALF-UP", "UP", "DOWN").
pub fn float_round(v: f64, rounding: f64, method: &str) -> f64 {
    if rounding == 0.0 || v == 0.0 { return v; }
    let n = v / rounding;
    let s = n.signum();
    let a = n.abs();
    let r = match method { "UP" => (a - 1e-9).ceil().max(0.0), "DOWN" => (a + 1e-9).floor(), _ => (a + 1e-9).round() };
    r * s * rounding
}
pub fn float_is_zero(v: f64, rounding: f64) -> bool { v.abs() < rounding / 2.0 }
/// `float_compare` with precision_digits: -1 / 0 / 1.
pub fn float_compare(a: f64, b: f64, digits: i32) -> i32 { let r = 10f64.powi(-digits); let d = float_round(a - b, r, "HALF-UP"); if d > 0.0 { 1 } else if d < 0.0 { -1 } else { 0 } }
/// decimal.precision 'Product Unit of Measure' default.
pub const UOM_DIGITS: i32 = 2;

// ---------------------------------------------------------------- UoM

pub fn uom_compute_quantity(env: &Env, qty: f64, from: Option<i64>, to: Option<i64>, round: bool, method: &str, raise: bool) -> Result<f64> {
    let (Some(f), Some(t)) = (from, to) else { return Ok(qty) };
    if qty == 0.0 { return Ok(qty); }
    let fu = rec(env, "uom.uom", f)?; let tu = rec(env, "uom.uom", t)?;
    if f != t && opt_id(&fu, "category_id") != opt_id(&tu, "category_id") {
        if raise { return Err(OdooError::User(format!("The unit of measure {} defined on the order line doesn't belong to the same category as the unit of measure {} defined on the product. Please correct the unit of measure defined on the order line or on the product. They should belong to the same category.", text(&fu, "name").unwrap_or_default(), text(&tu, "name").unwrap_or_default()))); }
        return Ok(qty);
    }
    let amount = if f == t { qty } else { let ff = num(&fu, "factor"); (if ff == 0.0 { qty } else { qty / ff }) * num(&tu, "factor") };
    Ok(if round { float_round(amount, num(&tu, "rounding"), method) } else { amount })
}
pub fn uom_compute_price(env: &Env, price: f64, from: Option<i64>, to: Option<i64>) -> Result<f64> {
    let (Some(f), Some(t)) = (from, to) else { return Ok(price) };
    if price == 0.0 || f == t { return Ok(price); }
    let fu = rec(env, "uom.uom", f)?; let tu = rec(env, "uom.uom", t)?;
    if opt_id(&fu, "category_id") != opt_id(&tu, "category_id") { return Ok(price); }
    let tf = num(&tu, "factor");
    Ok(if tf == 0.0 { price } else { price * num(&fu, "factor") / tf })
}

// ---------------------------------------------------------------- currency

pub fn cur_rounding(env: &Env, cur: Option<i64>) -> f64 {
    cur.and_then(|c| rec(env, "res.currency", c).ok()).map(|r| num(&r, "rounding")).filter(|r| *r > 0.0).unwrap_or(0.01)
}
pub fn cur_round(env: &Env, cur: Option<i64>, v: f64) -> f64 { float_round(v, cur_rounding(env, cur), "HALF-UP") }
pub fn cur_is_zero(env: &Env, cur: Option<i64>, v: f64) -> bool { float_is_zero(v, cur_rounding(env, cur)) }
pub fn company_currency(env: &Env, company: Option<i64>) -> Option<i64> {
    company.and_then(|c| rec(env, "res.company", c).ok()).and_then(|c| opt_id(&c, "currency_id"))
        .or_else(|| find_one(&env.sudo(), "res.company", Domain::True).ok().flatten().and_then(|c| rec(env, "res.company", c).ok()).and_then(|c| opt_id(&c, "currency_id")))
}
/// `res.currency._get_rates`: latest rate dated <= `date` (else the oldest one), 1.0 when there is none.
pub fn cur_rate(env: &Env, cur: i64, date: &str) -> f64 {
    if !has_model(env, "res.currency.rate") { return 1.0; }
    let e = env.sudo();
    let ids = orm::search(&e, "res.currency.rate", &term("currency_id", "=", cur), Some("name desc, id desc"), None, 0).unwrap_or_default();
    let rows: Vec<Row> = ids.iter().filter_map(|i| rec(&e, "res.currency.rate", *i).ok()).collect();
    let d = date_part(date);
    rows.iter().find(|r| text(r, "name").map_or(false, |n| date_part(&n) <= d)).or_else(|| rows.last()).map(|r| num(r, "rate")).filter(|r| *r != 0.0).unwrap_or(1.0)
}
/// `res.currency._convert` (no company-specific rates): amount * to_rate / from_rate.
pub fn convert(env: &Env, amount: f64, from: Option<i64>, to: Option<i64>, date: &str, round: bool) -> f64 {
    let (Some(f), Some(t)) = (from, to) else { return amount };
    if f == t { return amount; }
    let v = amount * cur_rate(env, t, date) / cur_rate(env, f, date);
    if round { cur_round(env, Some(t), v) } else { v }
}

// ---------------------------------------------------------------- groups

/// `res.users.has_group(xmlid)` including implied groups.
pub fn user_has_group(env: &Env, uid: i64, xmlid: &str) -> bool {
    if !has_model(env, "ir.model.data") || !has_field(env, "res.users", "groups_id") { return false; }
    let e = env.sudo();
    let (module, name) = xmlid.split_once('.').unwrap_or(("", xmlid));
    let Ok(Some(d)) = find_one(&e, "ir.model.data", term("module", "=", module).and(term("name", "=", name)).and(term("model", "=", "res.groups"))) else { return false };
    let Some(gid) = rec(&e, "ir.model.data", d).ok().and_then(|r| r.get("res_id").and_then(|v| v.as_i64())) else { return false };
    let Ok(u) = rec(&e, "res.users", uid) else { return false };
    let mut todo: Vec<i64> = ids(&u, "groups_id");
    let mut seen = BTreeSet::new();
    while let Some(g) = todo.pop() {
        if !seen.insert(g) { continue; }
        if g == gid { return true; }
        if has_field(&e, "res.groups", "implied_ids") { if let Ok(r) = rec(&e, "res.groups", g) { todo.extend(ids(&r, "implied_ids")); } }
    }
    false
}

// ---------------------------------------------------------------- misc record helpers

pub fn tmpl_of(env: &Env, product: i64) -> Result<Row> {
    let p = rec(env, "product.product", product)?;
    match opt_id(&p, "product_tmpl_id") { Some(t) => rec(env, "product.template", t), None => Ok(Row::new()) }
}
/// Product `display_name`: "[code] name" (variant attribute suffix not modelled).
pub fn product_display_name(env: &Env, product: i64) -> Result<String> {
    let p = rec(env, "product.product", product)?;
    let t = tmpl_of(env, product)?;
    let name = text(&t, "name").unwrap_or_default();
    Ok(match text(&p, "default_code").filter(|c| !c.is_empty()) { Some(c) => format!("[{c}] {name}"), None => name })
}
/// Product's company-visible tax set: `_filter_taxes_by_company` (company is the tax company or one of its descendants' parents).
pub fn filter_taxes_by_company(env: &Env, taxes: &[i64], company: Option<i64>) -> Vec<i64> {
    let mut chain = vec![]; let mut c = company;
    while let Some(id) = c { if chain.contains(&id) { break; } chain.push(id); c = rec(env, "res.company", id).ok().and_then(|r| opt_id(&r, "parent_id")); }
    taxes.iter().copied().filter(|t| rec(env, "account.tax", *t).map_or(false, |r| match opt_id(&r, "company_id") { None => true, Some(tc) => company.is_none() || chain.contains(&tc) })).collect()
}
/// `account.fiscal.position.map_tax`.
pub fn map_tax(env: &Env, fpos: Option<i64>, taxes: &[i64]) -> Vec<i64> {
    let Some(fp) = fpos else { return taxes.to_vec() };
    if !has_model(env, "account.fiscal.position.tax") { return taxes.to_vec(); }
    let maps = children(env, "account.fiscal.position.tax", "position_id", fp).unwrap_or_default();
    let mut out = vec![];
    for t in taxes {
        // tax_map: src -> active destination taxes; a mapping row without an active destination removes the tax
        let corr: Vec<&Row> = maps.iter().filter(|m| opt_id(m, "tax_src_id") == Some(*t)).collect();
        let res: Vec<i64> = if corr.is_empty() { vec![*t] } else { corr.iter().filter_map(|m| opt_id(m, "tax_dest_id")).filter(|d| rec(env, "account.tax", *d).map_or(false, |r| r.get("active").map_or(true, |v| v.truthy()))).collect() };
        for r in res { if !out.contains(&r) { out.push(r); } }
    }
    out
}
pub fn user_error<T>(msg: impl Into<String>) -> Result<T> { Err(OdooError::User(msg.into())) }

/// `account.tax` records as `tax::Tax`, resolving `price_include` the way Odoo 18 does
/// (override `tax_included`/`tax_excluded`, else the company setting `account_price_include`).
pub fn load_taxes(env: &Env, ids_: &[i64]) -> Result<Vec<crate::tax::Tax>> {
    ids_.iter().map(|i| {
        let r = rec(env, "account.tax", *i)?;
        let inc = match text(&r, "price_include_override").as_deref() {
            Some("tax_included") => true,
            Some("tax_excluded") => false,
            _ => r.get("price_include").map_or_else(|| opt_id(&r, "company_id").and_then(|c| rec(env, "res.company", c).ok()).map_or(false, |c| text(&c, "account_price_include").as_deref() == Some("tax_included")), |v| v.truthy()),
        };
        Ok(crate::tax::Tax { amount: num(&r, "amount"), kind: text(&r, "amount_type").unwrap_or_else(|| "percent".into()), price_include: inc })
    }).collect()
}
