//! sale_management: quotation templates and optional products
//! (ports of addons/sale_management/models/{sale_order,sale_order_line,sale_order_option,sale_order_template*}.py).
use crate::sale_methods::{order_pricing, price_line, product_line_name};
use crate::sale_util::*;
use crate::util::*;
use odoo_core::orm::{self, Env};
use odoo_core::{OdooError, Result, Row, Rules, Value};

fn rid(r: &Row) -> i64 { r["id"].as_i64().unwrap_or(0) }
fn one(ids_: &[i64]) -> Result<i64> { ids_.first().copied().ok_or_else(|| OdooError::User("Expected singleton".into())) }

/// Template-driven defaults for a new order (`_compute_note/require_signature/require_payment/prepayment_percent/validity_date/journal_id`).
pub fn template_defaults(env: &Env, v: &mut Row) -> Result<()> {
    if !has_model(env, "sale.order.template") { return Ok(()); }
    let company = v.get("company_id").and_then(|c| c.as_i64()).or_else(|| crate::pricelist_methods::env_company(env));
    if v.get("sale_order_template_id").map_or(true, |t| t.is_null()) {
        // `_compute_sale_order_template_id`: the company's default template
        if let Some(t) = company.and_then(|c| rec(env, "res.company", c).ok()).and_then(|c| opt_id(&c, "sale_order_template_id")) { v.insert("sale_order_template_id".into(), t.into()); }
    }
    let Some(tid) = v.get("sale_order_template_id").and_then(|t| t.as_i64()) else { return Ok(()) };
    let t = rec(env, "sale.order.template", tid)?;
    let set = |v: &mut Row, k: &str, val: Value| { if !v.contains_key(&format!("__explicit_{k}")) { v.insert(k.into(), val); } };
    if text(&t, "note").map_or(false, |n| !n.trim().is_empty() && n != "<p><br></p>") && !v.contains_key("note") { v.insert("note".into(), t["note"].clone()); }
    set(v, "require_signature", flag(&t, "require_signature").into());
    set(v, "require_payment", flag(&t, "require_payment").into());
    if flag(&t, "require_payment") { set(v, "prepayment_percent", t.get("prepayment_percent").cloned().unwrap_or(1.0.into())); }
    let days = t.get("number_of_days").and_then(|d| d.as_i64()).unwrap_or(0);
    if days > 0 { set(v, "validity_date", orm::shift_date(&orm::today(), days).into()); }
    if let Some(j) = opt_id(&t, "journal_id") { v.entry("journal_id".into()).or_insert(j.into()); }
    v.retain(|k, _| !k.starts_with("__explicit_"));
    Ok(())
}

/// Description of `product` taken from the template line when the order's template defines one (`sale.order.line._compute_name`).
pub fn template_line_name_if_installed(env: &Env, order: &Row, product: i64) -> Option<String> { if has_model(env, "sale.order.template.line") { template_line_name(env, order, product) } else { None } }
pub fn template_line_name(env: &Env, order: &Row, product: i64) -> Option<String> {
    let tid = opt_id(order, "sale_order_template_id")?;
    for l in children(env, "sale.order.template.line", "sale_order_template_id", tid).ok()? {
        if opt_id(&l, "product_id") == Some(product) { if let Some(n) = text(&l, "name").filter(|n| !n.is_empty()) { return Some(n); } }
    }
    None
}

/// Recompute price/discount of the optional-product lines of `order` (`_recompute_prices` of sale_management).
pub fn recompute_option_prices(env: &Env, order: i64) -> Result<()> {
    if !has_model(env, "sale.order.option") { return Ok(()); }
    let e = env.sudo();
    let o = rec(&e, "sale.order", order)?;
    let op = order_pricing(&e, &o);
    for opt in children(&e, "sale.order.option", "order_id", order)? {
        let Some(p) = opt_id(&opt, "product_id") else { continue };
        let lp = price_line(&e, &op, p, num(&opt, "quantity"), opt_id(&opt, "uom_id"))?;
        orm::write(&e, "sale.order.option", &[rid(&opt)], row(&[("price_unit", lp.price_unit.into()), ("discount", lp.discount.into())]))?;
    }
    Ok(())
}

fn option_defaults(env: &Env, v: &mut Row) -> Result<()> {
    let Some(pid) = v.get("product_id").and_then(|p| p.as_i64()) else { return Ok(()) };
    let t = tmpl_of(env, pid)?;
    if v.get("name").and_then(|n| n.as_str()).map_or(true, |n| n.is_empty()) { v.insert("name".into(), product_line_name(env, pid)?.into()); }
    if v.get("uom_id").map_or(true, |u| u.is_null()) { if let Some(u) = opt_id(&t, "uom_id") { v.insert("uom_id".into(), u.into()); } }
    if !v.contains_key("price_unit") || !v.contains_key("discount") {
        let order = v.get("order_id").and_then(|o| o.as_i64()).map(|o| rec(env, "sale.order", o)).transpose()?;
        let op = match &order { Some(o) => order_pricing(env, o), None => crate::sale_methods::default_pricing(env) };
        let lp = price_line(env, &op, pid, v.get("quantity").and_then(|q| q.as_f64()).unwrap_or(1.0), v.get("uom_id").and_then(|u| u.as_i64()))?;
        v.entry("price_unit".into()).or_insert(lp.price_unit.into());
        v.entry("discount".into()).or_insert(lp.discount.into());
    }
    Ok(())
}

/// `sale.order.template.line._prepare_order_line_values`
fn template_line_values(l: &Row) -> Row {
    let mut v = row(&[("display_type", l.get("display_type").cloned().unwrap_or(Value::Null)), ("product_id", idv(opt_id(l, "product_id"))), ("product_uom_qty", num(l, "product_uom_qty").into()), ("product_uom", idv(opt_id(l, "product_uom_id"))), ("sequence", l.get("sequence").cloned().unwrap_or(10.into()))]);
    if let Some(n) = text(l, "name").filter(|n| !n.is_empty()) { v.insert("name".into(), n.into()); }
    v
}
fn option_values(o: &Row) -> Row { row(&[("name", o.get("name").cloned().unwrap_or(Value::Null)), ("product_id", idv(opt_id(o, "product_id"))), ("quantity", num(o, "quantity").into()), ("uom_id", idv(opt_id(o, "uom_id")))]) }

/// `_onchange_sale_order_template_id`: replace the lines and options of `order` by those of its template.
pub fn apply_template(env: &Env, order: i64) -> Result<()> {
    let e = env.sudo();
    let o = rec(&e, "sale.order", order)?;
    let Some(tid) = opt_id(&o, "sale_order_template_id") else { return Ok(()) };
    // lines: clear then create (first created line gets sequence -99). Confirmed lines can't be removed, like in Odoo.
    let old: Vec<i64> = children(&e, "sale.order.line", "order_id", order)?.iter().map(rid).collect();
    if !old.is_empty() { orm::unlink(&e, "sale.order.line", &old)?; }
    let mut tl = children(&e, "sale.order.template.line", "sale_order_template_id", tid)?;
    tl.sort_by_key(|l| (l.get("sequence").and_then(|s| s.as_i64()).unwrap_or(10), rid(l)));
    for (i, l) in tl.iter().enumerate() {
        let mut v = template_line_values(l);
        v.insert("order_id".into(), order.into());
        if i == 0 { v.insert("sequence".into(), (-99).into()); }
        v.retain(|_, x| !x.is_null());
        orm::create(&e, "sale.order.line", v)?;
    }
    let oldo: Vec<i64> = children(&e, "sale.order.option", "order_id", order)?.iter().map(rid).collect();
    if !oldo.is_empty() { orm::unlink(&e, "sale.order.option", &oldo)?; }
    for opt in children(&e, "sale.order.template.option", "sale_order_template_id", tid)? {
        let mut v = option_values(&opt);
        v.insert("order_id".into(), order.into());
        v.retain(|_, x| !x.is_null());
        orm::create(&e, "sale.order.option", v)?;
    }
    Ok(())
}

fn can_be_edited_on_portal(o: &Row) -> bool { matches!(text(o, "state").as_deref(), Some("draft" | "sent")) }

fn add_option(env: &Env, opt: i64) -> Result<i64> {
    let e = env.sudo();
    let o = rec(&e, "sale.order.option", opt)?;
    let order = opt_id(&o, "order_id").ok_or_else(|| OdooError::Required("sale.order.option".into(), "order_id".into()))?;
    if !can_be_edited_on_portal(&rec(&e, "sale.order", order)?) { return user_error("You cannot add options to a confirmed order."); }
    let seq = children(&e, "sale.order.line", "order_id", order)?.iter().map(|l| l.get("sequence").and_then(|s| s.as_i64()).unwrap_or(0)).max().unwrap_or(0) + 1;
    let v = row(&[("order_id", order.into()), ("price_unit", num(&o, "price_unit").into()), ("technical_price_unit", num(&o, "price_unit").into()), ("name", text(&o, "name").unwrap_or_default().into()),
        ("product_id", idv(opt_id(&o, "product_id"))), ("product_uom_qty", num(&o, "quantity").into()), ("product_uom", idv(opt_id(&o, "uom_id"))), ("discount", num(&o, "discount").into()), ("sequence", seq.into())]);
    let line = orm::create(&e, "sale.order.line", v)?;
    orm::write(&e, "sale.order.option", &[opt], row(&[("line_id", line.into())]))?;
    Ok(line)
}

pub fn rules() -> Rules {
    Rules::default()
        .before_create("sale.order.template", |env, mut v| {
            let c = v.get("company_id").and_then(|c| c.as_i64()).or_else(|| crate::pricelist_methods::env_company(env)).and_then(|c| rec(env, "res.company", c).ok());
            if let Some(c) = c {
                v.entry("require_signature".into()).or_insert(flag(&c, "portal_confirmation_sign").into());
                v.entry("require_payment".into()).or_insert(flag(&c, "portal_confirmation_pay").into());
                v.entry("prepayment_percent".into()).or_insert(c.get("prepayment_percent").cloned().unwrap_or(1.0.into()));
            }
            Ok(v)
        })
        .after_create("sale.order.template", |env, ids_, _| check_prepayment(env, ids_))
        .after_write("sale.order.template", |env, ids_, v| {
            check_prepayment(env, ids_)?;
            // archiving a template unsets it as the company default
            if v.get("active").map_or(false, |a| !a.truthy()) && has_field(env, "res.company", "sale_order_template_id") {
                let e = env.sudo();
                for c in orm::search(&e, "res.company", &odoo_core::Domain::Term("sale_order_template_id".into(), "in".into(), Value::List(ids_.iter().map(|i| Value::Int(*i)).collect())), None, None, 0)? { orm::write(&e, "res.company", &[c], row(&[("sale_order_template_id", Value::Null)]))?; }
            }
            Ok(())
        })
        .before_create("sale.order.template.line", |env, mut v| {
            if text(&v, "display_type").map_or(false, |d| !d.is_empty()) { v.insert("product_id".into(), Value::Null); v.insert("product_uom_qty".into(), 0.into()); v.insert("product_uom_id".into(), Value::Null); }
            else if let Some(p) = v.get("product_id").and_then(|p| p.as_i64()) { if v.get("product_uom_id").map_or(true, |u| u.is_null()) { if let Some(u) = opt_id(&tmpl_of(env, p)?, "uom_id") { v.insert("product_uom_id".into(), u.into()); } } }
            Ok(v)
        })
        .before_create("sale.order.template.option", |env, mut v| {
            if let Some(p) = v.get("product_id").and_then(|p| p.as_i64()) {
                if v.get("name").and_then(|n| n.as_str()).map_or(true, |n| n.is_empty()) { v.insert("name".into(), product_line_name(env, p)?.into()); }
                if v.get("uom_id").map_or(true, |u| u.is_null()) { if let Some(u) = opt_id(&tmpl_of(env, p)?, "uom_id") { v.insert("uom_id".into(), u.into()); } }
            }
            Ok(v)
        })
        .before_create("sale.order.option", |env, mut v| { option_defaults(env, &mut v)?; Ok(v) })
        .compute("sale.order.option", "is_present", |env, r| {
            let (Some(o), Some(p)) = (opt_id(r, "order_id"), opt_id(r, "product_id")) else { return Ok(false.into()) };
            Ok(children(env, "sale.order.line", "order_id", o)?.iter().any(|l| opt_id(l, "product_id") == Some(p)).into())
        })
        .action("sale.order.option", "add_option_to_order", |env, ids_, _| Ok(Value::Int(add_option(env, one(ids_)?)?)))
        .action("sale.order.option", "button_add_to_order", |env, ids_, _| { add_option(env, one(ids_)?)?; Ok(Value::Bool(true)) })
        .action("sale.order.option", "_get_values_to_add_to_order", |env, ids_, _| {
            let o = rec(env, "sale.order.option", one(ids_)?)?;
            let order = opt_id(&o, "order_id").unwrap_or(0);
            let seq = children(env, "sale.order.line", "order_id", order)?.iter().map(|l| l.get("sequence").and_then(|s| s.as_i64()).unwrap_or(0)).max().unwrap_or(0) + 1;
            Ok(Value::Map(row(&[("order_id", order.into()), ("price_unit", num(&o, "price_unit").into()), ("technical_price_unit", num(&o, "price_unit").into()), ("name", text(&o, "name").unwrap_or_default().into()), ("product_id", idv(opt_id(&o, "product_id"))), ("product_uom_qty", num(&o, "quantity").into()), ("product_uom", idv(opt_id(&o, "uom_id"))), ("discount", num(&o, "discount").into()), ("sequence", seq.into())]).into_iter().collect()))
        })
        .action("sale.order.template.line", "_prepare_order_line_values", |env, ids_, _| Ok(Value::Map(template_line_values(&rec(env, "sale.order.template.line", one(ids_)?)?).into_iter().collect())))
        .action("sale.order.template.option", "_prepare_option_line_values", |env, ids_, _| Ok(Value::Map(option_values(&rec(env, "sale.order.template.option", one(ids_)?)?).into_iter().collect())))
        .action("sale.order", "_onchange_sale_order_template_id", |env, ids_, _| { for i in ids_ { apply_template(env, *i)?; } Ok(Value::Bool(true)) })
        .action("sale.order", "_can_be_edited_on_portal", |env, ids_, _| Ok(can_be_edited_on_portal(&rec(env, "sale.order", one(ids_)?)?).into()))
}

fn check_prepayment(env: &Env, ids_: &[i64]) -> Result<()> {
    for id in ids_ {
        let t = rec(env, "sale.order.template", *id)?;
        let p = num(&t, "prepayment_percent");
        if flag(&t, "require_payment") && !(p > 0.0 && p <= 1.0) { return Err(OdooError::Validation("Prepayment percentage must be a valid percentage.".into())); }
    }
    Ok(())
}
