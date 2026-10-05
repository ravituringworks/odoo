//! delivery: carriers, rate computation, shipping lines on sale orders
//! (ports of addons/delivery/models/{delivery_carrier,delivery_price_rule,delivery_zip_prefix,sale_order,sale_order_line}.py
//! and wizard/choose_delivery_carrier.py).
use crate::sale_methods::tax_included_unit_price;
use crate::sale_util::*;
use crate::util::*;
use crate::pricelist_methods as pl;
use odoo_core::orm::{self, Env};
use odoo_core::{OdooError, Result, Row, Rules, Value};

fn rid(r: &Row) -> i64 { r["id"].as_i64().unwrap_or(0) }
fn singleton(ids_: &[i64]) -> Result<i64> { ids_.first().copied().ok_or_else(|| OdooError::User("Expected singleton: delivery.carrier".into())) }
fn is_real(l: &Row) -> bool { text(l, "display_type").map_or(true, |d| d.is_empty()) }

pub struct Rate { pub success: bool, pub price: f64, pub error_message: Option<String>, pub warning_message: Option<String>, pub carrier_price: f64 }
impl Rate {
    pub fn to_value(&self) -> Value {
        let s = |o: &Option<String>| o.clone().map(Value::Text).unwrap_or(Value::Bool(false));
        Value::Map(row(&[("success", self.success.into()), ("price", self.price.into()), ("error_message", s(&self.error_message)), ("warning_message", s(&self.warning_message)), ("carrier_price", self.carrier_price.into())]).into_iter().collect())
    }
}
fn fail(msg: &str) -> Rate { Rate { success: false, price: 0.0, error_message: Some(msg.into()), warning_message: None, carrier_price: 0.0 } }

// ---------------------------------------------------------------- matching

/// `_match_address`
pub fn match_address(env: &Env, carrier: &Row, partner: Option<i64>) -> Result<bool> {
    let p = partner.map(|p| rec(env, "res.partner", p)).transpose()?.unwrap_or_default();
    let countries = ids(carrier, "country_ids");
    if !countries.is_empty() && !opt_id(&p, "country_id").map_or(false, |c| countries.contains(&c)) { return Ok(false); }
    let states = ids(carrier, "state_ids");
    if !states.is_empty() && !opt_id(&p, "state_id").map_or(false, |s| states.contains(&s)) { return Ok(false); }
    let prefixes: Vec<String> = ids(carrier, "zip_prefix_ids").into_iter().filter_map(|z| rec(env, "delivery.zip.prefix", z).ok().and_then(|r| text(&r, "name"))).collect();
    if !prefixes.is_empty() {
        let Some(zip) = text(&p, "zip").filter(|z| !z.is_empty()) else { return Ok(false) };
        let zip = zip.to_uppercase();
        // prefixes may be regular expressions; `$` anchors, otherwise a plain prefix test
        let ok = prefixes.iter().any(|pre| match regex_lite_match(pre, &zip) { Some(b) => b, None => zip.starts_with(pre.as_str()) });
        if !ok { return Ok(false); }
    }
    Ok(true)
}
/// Tiny matcher for the regex subset used in zip prefixes: literal chars, `.`, `\d`, `?`/`*`/`+` on a single atom, `$` end anchor.
fn regex_lite_match(pat: &str, s: &str) -> Option<bool> {
    fn m(p: &[char], s: &[char]) -> bool {
        if p.is_empty() { return true; }
        if p == ['$'] { return s.is_empty(); }
        let (atom, rest, adv): (Box<dyn Fn(char) -> bool>, &[char], usize) = if p[0] == '\\' && p.len() > 1 { let c = p[1]; (if c == 'd' { Box::new(|x: char| x.is_ascii_digit()) } else { Box::new(move |x| x == c) }, &p[2..], 2) } else if p[0] == '.' { (Box::new(|_| true), &p[1..], 1) } else { let c = p[0]; (Box::new(move |x| x == c), &p[1..], 1) };
        let _ = adv;
        match rest.first() {
            Some('?') => (!s.is_empty() && atom(s[0]) && m(&rest[1..], &s[1..])) || m(&rest[1..], s),
            Some('*') | Some('+') => { let min = if rest[0] == '+' { 1 } else { 0 }; let mut n = 0; while n < s.len() && atom(s[n]) { n += 1; } (min..=n).rev().any(|k| m(&rest[1..], &s[k..])) }
            _ => !s.is_empty() && atom(s[0]) && m(rest, &s[1..]),
        }
    }
    if !pat.chars().any(|c| matches!(c, '\\' | '.' | '$' | '?' | '*' | '+' | '[' | '(' | '|')) { return None; }
    if pat.contains('[') || pat.contains('(') || pat.contains('|') { return None; }
    let p: Vec<char> = pat.chars().collect(); let sc: Vec<char> = s.chars().collect();
    Some(m(&p, &sc))
}
fn order_lines(env: &Env, order: i64) -> Result<Vec<Row>> { children(env, "sale.order.line", "order_id", order) }
fn product_tags(env: &Env, p: i64) -> Vec<i64> {
    let mut t: Vec<i64> = rec(env, "product.product", p).map(|r| ids(&r, "additional_product_tag_ids")).unwrap_or_default();
    if let Ok(tm) = tmpl_of(env, p) { t.extend(ids(&tm, "product_tag_ids")); }
    t
}
pub fn match_must_have_tags(env: &Env, c: &Row, order: i64) -> Result<bool> {
    let must = ids(c, "must_have_tag_ids");
    if must.is_empty() { return Ok(true); }
    for l in order_lines(env, order)? { if let Some(p) = opt_id(&l, "product_id") { if product_tags(env, p).iter().any(|t| must.contains(t)) { return Ok(true); } } }
    Ok(false)
}
pub fn match_excluded_tags(env: &Env, c: &Row, order: i64) -> Result<bool> {
    let ex = ids(c, "excluded_tag_ids");
    for l in order_lines(env, order)? { if let Some(p) = opt_id(&l, "product_id") { if product_tags(env, p).iter().any(|t| ex.contains(t)) { return Ok(false); } } }
    Ok(true)
}
fn sum_attr(env: &Env, order: i64, attr: &str) -> Result<f64> {
    let mut s = 0.0;
    for l in order_lines(env, order)? { if let Some(p) = opt_id(&l, "product_id") { let pr = rec(env, "product.product", p)?; let tv = if pr.get(attr).map_or(false, |v| !v.is_null()) { num(&pr, attr) } else { num(&tmpl_of(env, p)?, attr) }; s += tv * product_qty(env, &l)?; } }
    Ok(s)
}
pub fn match_weight(env: &Env, c: &Row, order: i64) -> Result<bool> { Ok(num(c, "max_weight") == 0.0 || sum_attr(env, order, "weight")? <= num(c, "max_weight")) }
pub fn match_volume(env: &Env, c: &Row, order: i64) -> Result<bool> { Ok(num(c, "max_volume") == 0.0 || sum_attr(env, order, "volume")? <= num(c, "max_volume")) }
/// `_match`
pub fn carrier_match(env: &Env, c: &Row, partner: Option<i64>, order: i64) -> Result<bool> {
    Ok(match_address(env, c, partner)? && match_must_have_tags(env, c, order)? && match_excluded_tags(env, c, order)? && match_weight(env, c, order)? && match_volume(env, c, order)?)
}

// ---------------------------------------------------------------- rating

fn carrier_company_currency(env: &Env, c: &Row) -> Option<i64> { company_currency(env, opt_id(c, "company_id")) }
/// `_compute_currency`: "company_to_pricelist" | "pricelist_to_company".
fn compute_currency(env: &Env, c: &Row, order: &Row, price: f64, conversion: &str) -> f64 {
    let comp = carrier_company_currency(env, c); let ord = opt_id(order, "currency_id");
    let (from, to) = if conversion == "company_to_pricelist" { (comp, ord) } else { (ord, comp) };
    if from == to { return price; }
    convert(env, price, from, to, &text(order, "date_order").unwrap_or_else(orm::today), true)
}
fn amount_total_without_delivery(env: &Env, order: i64) -> Result<f64> {
    let o = rec(env, "sale.order", order)?;
    let d: f64 = order_lines(env, order)?.iter().filter(|l| flag(l, "is_delivery")).map(|l| num(l, "price_total")).sum();
    Ok(num(&o, "amount_total") - d)
}
fn var_value(name: &str, total: f64, weight: f64, volume: f64, quantity: f64, wv: f64) -> f64 { match name { "price" => total, "volume" => volume, "weight" => weight, "wv" => if wv != 0.0 { wv } else { volume * weight }, _ => quantity } }
/// `_get_price_from_picking`
pub fn price_from_picking(env: &Env, c: &Row, total: f64, weight: f64, volume: f64, quantity: f64, wv: f64) -> Result<f64> {
    let mut rules = children(env, "delivery.price.rule", "carrier_id", rid(c))?;
    rules.sort_by(|a, b| a["sequence"].as_i64().cmp(&b["sequence"].as_i64()).then(num(a, "list_price").partial_cmp(&num(b, "list_price")).unwrap_or(std::cmp::Ordering::Equal)).then(rid(a).cmp(&rid(b))));
    for r in rules {
        let v = var_value(&text(&r, "variable").unwrap_or_default(), total, weight, volume, quantity, wv);
        let mv = num(&r, "max_value");
        let ok = match text(&r, "operator").as_deref() { Some("==") => v == mv, Some("<=") => v <= mv, Some("<") => v < mv, Some(">=") => v >= mv, Some(">") => v > mv, _ => false };
        if ok { return Ok(num(&r, "list_base_price") + num(&r, "list_price") * var_value(&text(&r, "variable_factor").unwrap_or_default(), total, weight, volume, quantity, wv)); }
    }
    user_error("Not available for current order")
}
/// `_get_price_available`
pub fn price_available(env: &Env, c: &Row, order: i64) -> Result<f64> {
    let o = rec(env, "sale.order", order)?;
    let (mut weight, mut volume, mut quantity, mut wv) = (0.0, 0.0, 0.0, 0.0);
    let mut total_delivery = 0.0;
    for l in order_lines(env, order)? {
        if text(&l, "state").as_deref() == Some("cancel") { continue; }
        if flag(&l, "is_delivery") { total_delivery += num(&l, "price_total"); }
        let Some(p) = opt_id(&l, "product_id") else { continue };
        if flag(&l, "is_delivery") { continue; }
        let t = tmpl_of(env, p)?;
        if matches!(text(&t, "type").as_deref(), Some("service" | "combo")) { continue; }
        let pr = rec(env, "product.product", p)?;
        let g = |f: &str| if pr.get(f).map_or(false, |v| !v.is_null()) { num(&pr, f) } else { num(&t, f) };
        let qty = uom_compute_quantity(env, num(&l, "product_uom_qty"), opt_id(&l, "product_uom"), opt_id(&t, "uom_id"), true, "UP", true)?;
        weight += g("weight") * qty; volume += g("volume") * qty; wv += g("weight") * g("volume") * qty; quantity += qty;
    }
    let total = compute_currency(env, c, &o, num(&o, "amount_total") - total_delivery, "pricelist_to_company");
    let w = env.ctx.get("order_weight").and_then(|v| v.as_f64()).filter(|w| *w != 0.0).or(Some(num(&o, "shipping_weight")).filter(|w| *w != 0.0)).unwrap_or(weight);
    price_from_picking(env, c, total, w, volume, quantity, wv)
}
/// `fixed_rate_shipment` / `base_on_rule_rate_shipment` raw result (before margins)
fn provider_rate(env: &Env, c: &Row, order: i64, kind: &str) -> Result<Rate> {
    let o = rec(env, "sale.order", order)?;
    if !match_address(env, c, opt_id(&o, "partner_shipping_id"))? { return Ok(fail("Error: this delivery method is not available for this address.")); }
    let price = if kind == "fixed" {
        let product = opt_id(c, "product_id").ok_or_else(|| OdooError::Required("delivery.carrier".into(), "product_id".into()))?;
        // Odoo calls `_get_product_price(product, 1.0)` without a date: rules are evaluated "now"
        pl::get_product_price(env, opt_id(&o, "pricelist_id"), product, 1.0, None, "")?
    } else {
        match price_available(env, c, order) { Ok(p) => compute_currency(env, c, &o, p, "company_to_pricelist"), Err(OdooError::User(m)) => return Ok(fail(&m)), Err(e) => return Err(e) }
    };
    Ok(Rate { success: true, price, error_message: None, warning_message: None, carrier_price: 0.0 })
}
/// `rate_shipment`
pub fn rate_shipment(env: &Env, carrier: i64, order: i64) -> Result<Rate> {
    let c = rec(env, "delivery.carrier", carrier)?;
    let o = rec(env, "sale.order", order)?;
    let kind = text(&c, "delivery_type").unwrap_or_else(|| "fixed".into());
    if kind != "fixed" && kind != "base_on_rule" { return Ok(fail("Error: this delivery method is not available.")); }
    let mut res = provider_rate(env, &c, order, &kind)?;
    // apply fiscal position to the (tax-included) shipping price
    if let Some(p) = opt_id(&c, "product_id") {
        let company = opt_id(&c, "company_id").or_else(|| opt_id(&o, "company_id"));
        let t = tmpl_of(env, p)?;
        let ptax = filter_taxes_by_company(env, &ids(&t, "taxes_id"), company);
        res.price = tax_included_unit_price(env, res.price, &ptax, opt_id(&o, "fiscal_position_id"))?;
    }
    // margins (fixed price carriers are exempt)
    if kind != "fixed" {
        let fm = compute_currency(env, &c, &o, num(&c, "fixed_margin"), "company_to_pricelist");
        res.price = res.price * (1.0 + num(&c, "margin")) + fm;
    }
    res.carrier_price = res.price;
    if res.success && flag(&c, "free_over") && kind != "base_on_rule" && compute_currency(env, &c, &o, amount_total_without_delivery(env, order)?, "pricelist_to_company") >= num(&c, "amount") {
        res.warning_message = Some(format!("The shipping is free since the order amount exceeds {:.2}.", num(&c, "amount")));
        res.price = 0.0;
    }
    Ok(res)
}

// ---------------------------------------------------------------- order shipping lines

fn estimated_weight(env: &Env, order: i64) -> Result<f64> {
    let mut w = 0.0;
    for l in order_lines(env, order)? {
        let Some(p) = opt_id(&l, "product_id") else { continue };
        if flag(&l, "is_delivery") || !is_real(&l) || num(&l, "product_uom_qty") <= 0.0 { continue; }
        let t = tmpl_of(env, p)?;
        if text(&t, "type").as_deref() != Some("consu") { continue; }
        let pr = rec(env, "product.product", p)?;
        let weight = if pr.get("weight").map_or(false, |v| !v.is_null()) { num(&pr, "weight") } else { num(&t, "weight") };
        w += product_qty(env, &l)? * weight;
    }
    Ok(w)
}
fn product_qty(env: &Env, l: &Row) -> Result<f64> {
    let Some(p) = opt_id(l, "product_id") else { return Ok(0.0) };
    if opt_id(l, "product_uom").is_none() || num(l, "product_uom_qty") == 0.0 { return Ok(0.0); }
    uom_compute_quantity(env, num(l, "product_uom_qty"), opt_id(l, "product_uom"), opt_id(&tmpl_of(env, p)?, "uom_id"), true, "UP", true)
}
fn remove_delivery_lines(env: &Env, order: i64) -> Result<()> {
    let lines: Vec<Row> = order_lines(env, order)?.into_iter().filter(|l| flag(l, "is_delivery")).collect();
    if lines.is_empty() { return Ok(()); }
    let to_delete: Vec<i64> = lines.iter().filter(|l| num(l, "qty_invoiced") == 0.0).map(rid).collect();
    if to_delete.is_empty() {
        let mut m = String::from("You can not update the shipping costs on an order where it was already invoiced!\n\nThe following delivery lines (product, invoiced quantity and price) have already been processed:\n\n");
        let items: Vec<String> = lines.iter().map(|l| format!("- {}: {:?} x {:?}", opt_id(l, "product_id").and_then(|p| product_display_name(env, p).ok()).unwrap_or_default(), num(l, "qty_invoiced"), num(l, "price_unit"))).collect();
        m.push_str(&items.join("\n"));
        return Err(OdooError::User(m));
    }
    orm::unlink(env, "sale.order.line", &to_delete)
}
pub fn prepare_delivery_line_vals(env: &Env, order: i64, carrier: i64, price_unit: f64) -> Result<Row> {
    let o = rec(env, "sale.order", order)?; let c = rec(env, "delivery.carrier", carrier)?;
    let product = opt_id(&c, "product_id").ok_or_else(|| OdooError::Required("delivery.carrier".into(), "product_id".into()))?;
    let t = tmpl_of(env, product)?;
    let taxes = filter_taxes_by_company(env, &ids(&t, "taxes_id"), opt_id(&o, "company_id"));
    let tax_ids = if opt_id(&o, "partner_id").is_some() && opt_id(&o, "fiscal_position_id").is_some() { map_tax(env, opt_id(&o, "fiscal_position_id"), &taxes) } else { taxes };
    let cname = text(&c, "name").unwrap_or_default();
    let mut name = match text(&t, "description_sale").filter(|d| !d.is_empty()) { Some(d) => format!("{cname}: {d}"), None => cname };
    if flag(&c, "free_over") && cur_is_zero(env, opt_id(&o, "currency_id"), price_unit) { name = format!("{name}\nFree Shipping"); }
    let mut v = row(&[("order_id", order.into()), ("name", name.into()), ("price_unit", price_unit.into()), ("product_uom_qty", 1.0.into()), ("product_id", product.into()), ("tax_id", set6(&tax_ids)), ("is_delivery", true.into())]);
    if let Some(u) = opt_id(&t, "uom_id") { v.insert("product_uom".into(), u.into()); }
    if let Some(last) = order_lines(env, order)?.iter().max_by_key(|l| (l.get("sequence").and_then(|s| s.as_i64()).unwrap_or(10), rid(l))) { v.insert("sequence".into(), (last.get("sequence").and_then(|s| s.as_i64()).unwrap_or(10) + 1).into()); }
    Ok(v)
}
pub fn set_delivery_line(env: &Env, order: i64, carrier: i64, amount: f64) -> Result<i64> {
    let e = env.sudo();
    remove_delivery_lines(&e, order)?;
    orm::write(&e, "sale.order", &[order], row(&[("carrier_id", carrier.into())]))?;
    let v = prepare_delivery_line_vals(&e, order, carrier, amount)?;
    orm::create(&e, "sale.order.line", v)
}

pub fn rules() -> Rules {
    Rules::default()
        .before_create("delivery.zip.prefix", |_, mut v| { if let Some(n) = text(&v, "name") { v.insert("name".into(), n.to_uppercase().into()); } Ok(v) })
        .before_write("delivery.zip.prefix", |_, mut v| { if let Some(n) = text(&v, "name") { v.insert("name".into(), n.to_uppercase().into()); } Ok(v) })
        .compute("delivery.carrier", "fixed_price", |env, r| Ok(match opt_id(r, "product_id") { Some(p) => num(&tmpl_of(env, p)?, "list_price").into(), None => r.get("fixed_price").cloned().unwrap_or(0.0.into()) }))
        .after_create("delivery.carrier", |env, ids_, v| { carrier_checks(env, ids_, v)?; fixed_price_inverse(env, ids_, v) })
        .after_write("delivery.carrier", |env, ids_, v| {
            carrier_checks(env, ids_, v)?;
            fixed_price_inverse(env, ids_, v)
        })
        .compute("delivery.carrier", "weight_uom_name", |_, _| Ok("kg".into()))
        .compute("delivery.carrier", "volume_uom_name", |_, _| Ok("m\u{b3}".into()))
        .compute("delivery.carrier", "can_generate_return", |_, _| Ok(false.into()))
        .compute("delivery.carrier", "supports_shipping_insurance", |_, _| Ok(false.into()))
        .compute("delivery.price.rule", "name", |env, r| {
            let (v, op, mv) = (text(r, "variable").unwrap_or_default(), text(r, "operator").unwrap_or_default(), num(r, "max_value"));
            let base = format!("{:.2}", num(r, "list_base_price")); let price = format!("{:.2}", num(r, "list_price"));
            let _ = env;
            let head = format!("if {v} {op} {mv:.2} then");
            let fac = text(r, "variable_factor").unwrap_or_default();
            Ok(if num(r, "list_base_price") != 0.0 && num(r, "list_price") == 0.0 { format!("{head} fixed price {base}") } else if num(r, "list_price") != 0.0 && num(r, "list_base_price") == 0.0 { format!("{head} {price} times {fac}") } else { format!("{head} fixed price {base} plus {price} times {fac}") }.into())
        })
        // ---- sale.order / sale.order.line
        .compute("sale.order", "delivery_set", |env, r| Ok(order_lines(env, rid(r))?.iter().any(|l| flag(l, "is_delivery")).into()))
        .compute("sale.order", "is_all_service", |env, r| { for l in order_lines(env, rid(r))?.iter().filter(|l| is_real(l)) { if let Some(p) = opt_id(l, "product_id") { if text(&tmpl_of(env, p)?, "type").as_deref() != Some("service") { return Ok(false.into()); } } else { return Ok(false.into()); } } Ok(true.into()) })
        .compute("sale.order", "shipping_weight", |env, r| Ok(estimated_weight(env, rid(r))?.into()))
        .compute("sale.order.line", "product_qty", |env, r| Ok(product_qty(env, r)?.into()))
        .on_unlink("sale.order.line", |env, ids_| {
            for i in ids_ { let l = rec(env, "sale.order.line", *i)?; if flag(&l, "is_delivery") { if let Some(o) = opt_id(&l, "order_id") { if opt_id(&rec(env, "sale.order", o)?, "carrier_id").is_some() { orm::write(&env.sudo(), "sale.order", &[o], row(&[("carrier_id", Value::Null)]))?; } } } }
            Ok(vec![])
        })
        .action("sale.order", "_compute_amount_total_without_delivery", |env, ids_, _| Ok(amount_total_without_delivery(env, singleton(ids_)?)?.into()))
        .action("sale.order", "_get_estimated_weight", |env, ids_, _| Ok(estimated_weight(env, singleton(ids_)?)?.into()))
        .action("sale.order", "_remove_delivery_line", |env, ids_, _| { for i in ids_ { remove_delivery_lines(&env.sudo(), *i)?; } Ok(Value::Bool(true)) })
        .action("sale.order", "set_delivery_line", |env, ids_, a| {
            let carrier = a.get("carrier_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("carrier_id required".into()))?;
            let amount = a.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
            for i in ids_ { set_delivery_line(env, *i, carrier, amount)?; }
            Ok(Value::Bool(true))
        })
        .action("sale.order", "_prepare_delivery_line_vals", |env, ids_, a| Ok(Value::Map(prepare_delivery_line_vals(env, singleton(ids_)?, a.get("carrier_id").and_then(|v| v.as_i64()).unwrap_or(0), a.get("price_unit").and_then(|v| v.as_f64()).unwrap_or(0.0))?.into_iter().collect())))
        // ---- carrier API
        .action("delivery.carrier", "rate_shipment", |env, ids_, a| {
            let order = a.get("order_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("order_id required".into()))?;
            let e = match a.get("order_weight") { Some(w) => env.with_ctx("order_weight", w.clone()), None => env.with_ctx("order_weight", Value::Null) };
            Ok(rate_shipment(&e, singleton(ids_)?, order)?.to_value())
        })
        .action("delivery.carrier", "_match", |env, ids_, a| { let c = rec(env, "delivery.carrier", singleton(ids_)?)?; Ok(carrier_match(env, &c, a.get("partner_id").and_then(|v| v.as_i64()), a.get("order_id").and_then(|v| v.as_i64()).unwrap_or(0))?.into()) })
        .action("delivery.carrier", "_match_address", |env, ids_, a| { let c = rec(env, "delivery.carrier", singleton(ids_)?)?; Ok(match_address(env, &c, a.get("partner_id").and_then(|v| v.as_i64()))?.into()) })
        .action("delivery.carrier", "available_carriers", |env, ids_, a| {
            let (partner, order) = (a.get("partner_id").and_then(|v| v.as_i64()), a.get("order_id").and_then(|v| v.as_i64()).unwrap_or(0));
            let mut out = vec![]; for i in ids_ { if carrier_match(env, &rec(env, "delivery.carrier", *i)?, partner, order)? { out.push(Value::Int(*i)); } }
            Ok(Value::List(out))
        })
        .action("delivery.carrier", "_is_available_for_order", |env, ids_, a| {
            let id = singleton(ids_)?; let order = a.get("order_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("order_id required".into()))?;
            let c = rec(env, "delivery.carrier", id)?; let o = rec(env, "sale.order", order)?;
            if !carrier_match(env, &c, opt_id(&o, "partner_shipping_id"), order)? { return Ok(false.into()); }
            if text(&c, "delivery_type").as_deref() == Some("base_on_rule") { return Ok(rate_shipment(env, id, order)?.success.into()); }
            Ok(true.into())
        })
        .action("delivery.carrier", "_get_price_from_picking", |env, ids_, a| {
            let c = rec(env, "delivery.carrier", singleton(ids_)?)?; let g = |k: &str| a.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0);
            Ok(price_from_picking(env, &c, g("total"), g("weight"), g("volume"), g("quantity"), g("wv"))?.into())
        })
        .action("delivery.carrier", "toggle_prod_environment", |env, ids_, _| { for i in ids_ { let c = rec(env, "delivery.carrier", *i)?; orm::write(&env.sudo(), "delivery.carrier", &[*i], row(&[("prod_environment", (!flag(&c, "prod_environment")).into())]))?; } Ok(Value::Bool(true)) })
        .action("delivery.carrier", "toggle_debug", |env, ids_, _| { for i in ids_ { let c = rec(env, "delivery.carrier", *i)?; orm::write(&env.sudo(), "delivery.carrier", &[*i], row(&[("debug_logging", (!flag(&c, "debug_logging")).into())]))?; } Ok(Value::Bool(true)) })
        // ---- wizard
        .action("choose.delivery.carrier", "update_price", |env, ids_, _| {
            let w = rec(env, "choose.delivery.carrier", singleton(ids_)?)?;
            let (carrier, order) = (opt_id(&w, "carrier_id"), opt_id(&w, "order_id"));
            let (Some(c), Some(o)) = (carrier, order) else { return user_error("carrier and order required") };
            let e = if num(&w, "total_weight") != 0.0 { env.with_ctx("order_weight", num(&w, "total_weight").into()) } else { env.clone_ctx() };
            let r = rate_shipment(&e, c, o)?;
            if !r.success { return user_error(r.error_message.unwrap_or_default()); }
            orm::write(&env.sudo(), "choose.delivery.carrier", &[rid(&w)], row(&[("delivery_price", r.price.into()), ("display_price", r.carrier_price.into()), ("delivery_message", r.warning_message.map(Value::Text).unwrap_or(Value::Null))]))?;
            Ok(Value::Bool(true))
        })
        .action("choose.delivery.carrier", "button_confirm", |env, ids_, _| {
            let w = rec(env, "choose.delivery.carrier", singleton(ids_)?)?;
            let (Some(c), Some(o)) = (opt_id(&w, "carrier_id"), opt_id(&w, "order_id")) else { return user_error("carrier and order required") };
            set_delivery_line(env, o, c, num(&w, "delivery_price"))?;
            orm::write(&env.sudo(), "sale.order", &[o], row(&[("recompute_delivery_price", false.into()), ("delivery_message", text(&w, "delivery_message").map(Value::Text).unwrap_or(Value::Null))]))?;
            Ok(Value::Bool(true))
        })
}

fn carrier_checks(env: &Env, ids_: &[i64], _v: &Row) -> Result<()> {
    for id in ids_ {
        let c = rec(env, "delivery.carrier", *id)?;
        let a = ids(&c, "must_have_tag_ids"); let b = ids(&c, "excluded_tag_ids");
        if a.iter().any(|t| b.contains(t)) { return user_error(format!("Carrier {} cannot have the same tag in both Must Have Tags and Excluded Tags.", text(&c, "name").unwrap_or_default())); }
        if num(&c, "margin") < -1.0 { return user_error("Margin cannot be lower than -100%"); }
        let si = c.get("shipping_insurance").and_then(|v| v.as_i64()).unwrap_or(0);
        if !(0..=100).contains(&si) { return user_error("The shipping insurance must be a percentage between 0 and 100."); }
    }
    Ok(())
}
trait CloneCtx { fn clone_ctx(&self) -> Env<'_>; }
impl CloneCtx for Env<'_> { fn clone_ctx(&self) -> Env<'_> { self.with_ctx("order_weight", Value::Null) } }

/// `_set_product_fixed_price` inverse: writing `fixed_price` sets the delivery product's list price.
fn fixed_price_inverse(env: &Env, ids_: &[i64], v: &Row) -> Result<()> {
    if let Some(fp) = v.get("fixed_price").and_then(|f| f.as_f64()) {
        for id in ids_ { if let Some(p) = opt_id(&rec(env, "delivery.carrier", *id)?, "product_id") { if let Some(t) = opt_id(&rec(env, "product.product", p)?, "product_tmpl_id") { orm::write(&env.sudo(), "product.template", &[t], row(&[("list_price", fp.into())]))?; } } }
        orm::recompute_ids(env, "delivery.carrier", ids_)?;
    }
    Ok(())
}
