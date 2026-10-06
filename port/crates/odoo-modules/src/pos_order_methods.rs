//! pos.order / pos.order.line behaviour ported from `addons/point_of_sale/models/pos_order.py`: computes, price/payment
//! bookkeeping, `sync_from_ui` and its helpers, paying, invoicing, cancelling and refunding.
//!
//! Not ported (needs engines this port does not have): anglo-saxon cost computation (`_compute_total_cost_*`), delayed
//! shipping procurements (`shipping_date`), the reversal entry for orders invoiced after their session closed, mail receipts,
//! and the "paid orders cannot go back to draft" write guard (the ORM write hook cannot see the stored state).
//! The terminal runtime in `pos.rs` (`create_from_ui`, `refund`) is a separate, complementary entry point.
use crate::pos_methods::{flag, has_model, id_list, invalid, m2o, many, set6, user_err, xmlid};
use crate::{account, pos_post, tax, util::*};
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, OdooError, Result, Row, Rules, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};

fn oid_of(ids_: &[i64]) -> Result<i64> { ids_.first().copied().ok_or_else(|| OdooError::User("Select an order".into())) }
pub(crate) fn new_uuid() -> String {
    static N: AtomicU64 = AtomicU64::new(0);
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0) as u64;
    let a = t ^ (N.fetch_add(1, Ordering::Relaxed).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let b = a.rotate_left(17).wrapping_mul(0xBF58_476D_1CE4_E5B9) ^ t.rotate_right(7);
    format!("{:08x}-{:04x}-4{:03x}-a{:03x}-{:012x}", (a >> 32) as u32, (a >> 16) as u16, (a & 0xfff) as u16, (b >> 52) as u16 & 0xfff, b & 0xffff_ffff_ffff)
}

// ---- currency helpers -----------------------------------------------------------------------------------------------
fn order_cfg(env: &Env, o: &Row) -> Result<Row> {
    let s = id_of(o, "session_id").ok_or_else(|| OdooError::User("The order has no session".into()))?;
    rec(env, "pos.config", id_of(&rec(env, "pos.session", s)?, "config_id").ok_or_else(|| OdooError::User("The session has no point of sale".into()))?)
}
fn order_currency(env: &Env, o: &Row) -> Option<i64> { order_cfg(env, o).ok().and_then(|c| crate::pos_methods::config_currency_id(env, c["id"].as_i64().unwrap())) }
fn rounding_of(env: &Env, cur: Option<i64>) -> f64 { cur.and_then(|c| rec(env, "res.currency", c).ok()).map(|c| num(&c, "rounding")).filter(|r| *r > 0.0).unwrap_or(0.01) }
fn clean(x: f64) -> f64 { (x * 1e9).round() / 1e9 }
/// `float_round` for the three methods of account.cash.rounding.
fn float_round(x: f64, rounding: f64, method: &str) -> f64 {
    let q = x / rounding;
    let n = match method { "UP" => if q >= 0.0 { (q - 1e-9).ceil() } else { (q + 1e-9).floor() }, "DOWN" => if q >= 0.0 { (q + 1e-9).floor() } else { (q - 1e-9).ceil() }, _ => if q >= 0.0 { (q + 0.5 + 1e-9).floor() } else { (q - 0.5 - 1e-9).ceil() } };
    clean(n * rounding)
}
fn is_zero(x: f64, rounding: f64) -> bool { x.abs() < rounding / 2.0 }
/// Units of `to` per unit of `from` on `date` (company currency has rate 1; a missing rate means 1).
fn currency_rate_at(env: &Env, cur: i64, date: &str) -> f64 {
    if !has_model(env, "res.currency.rate") { return 1.0; }
    let d = if date.len() >= 10 { &date[..10] } else { date };
    let dom = Domain::And(vec![term("currency_id", "=", cur), term("name", "<=", d)]);
    orm::search(env, "res.currency.rate", &dom, Some("name desc"), Some(1), 0).ok().and_then(|v| v.first().copied()).and_then(|r| rec(env, "res.currency.rate", r).ok()).map(|r| num(&r, "rate")).filter(|r| *r > 0.0).unwrap_or(1.0)
}

// ---- fiscal position / tax helpers -------------------------------------------------------------------------------------
/// `account.fiscal.position.map_tax`.
pub(crate) fn map_tax(env: &Env, fpos: Option<i64>, taxes: &[i64]) -> Result<Vec<i64>> {
    let Some(fp) = fpos else { return Ok(taxes.to_vec()) };
    if !has_model(env, "account.fiscal.position.tax") { return Ok(taxes.to_vec()); }
    let maps = children(env, "account.fiscal.position.tax", "position_id", fp)?;
    let mut out: Vec<i64> = vec![];
    for t in taxes {
        let mine: Vec<&Row> = maps.iter().filter(|m| id_of(m, "tax_src_id") == Some(*t)).collect();
        let dest: Vec<i64> = if mine.is_empty() { vec![*t] } else { mine.iter().filter_map(|m| id_of(m, "tax_dest_id")).filter(|d| rec(env, "account.tax", *d).map_or(false, |x| x.get("active").map_or(true, |a| a.truthy()))).collect() };
        for d in dest { if !out.contains(&d) { out.push(d); } }
    }
    Ok(out)
}
fn line_taxes_after_fpos(env: &Env, l: &Row) -> Result<Vec<i64>> {
    let fp = id_of(l, "order_id").and_then(|o| rec(env, "pos.order", o).ok()).and_then(|o| id_of(&o, "fiscal_position_id"));
    map_tax(env, fp, &ids(l, "tax_ids"))
}
/// `_compute_amount_line_all`: (price_subtotal, price_subtotal_incl).
fn line_amounts(env: &Env, l: &Row) -> Result<(f64, f64)> {
    let taxes = tax::load(env, &line_taxes_after_fpos(env, l)?)?;
    let (excl, _, incl) = tax::compute(num(l, "qty"), num(l, "price_unit"), num(l, "discount"), &taxes);
    Ok((excl, incl))
}
/// `_get_discount_amount`.
pub(crate) fn line_discount_amount(env: &Env, l: &Row) -> Result<f64> {
    let taxes = tax::load(env, &line_taxes_after_fpos(env, l)?)?;
    let (_, _, original) = tax::compute(num(l, "qty"), num(l, "price_unit"), 0.0, &taxes);
    Ok(clean(original - num(l, "price_subtotal_incl")))
}

// ---- computes ---------------------------------------------------------------------------------------------------------
fn order_lines(env: &Env, r: &Row) -> Result<Vec<Row>> { many(env, "pos.order.line", &ids(r, "lines")) }
fn refund_lines_of(env: &Env, l: &Row) -> Result<Vec<Row>> { children(env, "pos.order.line", "refunded_orderline_id", l["id"].as_i64().unwrap_or(0)) }
fn line_refunded_qty(env: &Env, l: &Row) -> Result<f64> {
    let mut q = 0.0;
    for rl in refund_lines_of(env, l)? { if id_of(&rl, "order_id").and_then(|o| rec(env, "pos.order", o).ok()).and_then(|o| text(&o, "state")).as_deref() != Some("cancel") { q += num(&rl, "qty"); } }
    Ok(-q)
}
fn refund_orders(env: &Env, r: &Row) -> Result<Vec<i64>> {
    let mut out = vec![];
    for l in order_lines(env, r)? { for rl in refund_lines_of(env, &l)? { if let Some(o) = id_of(&rl, "order_id") { if !out.contains(&o) { out.push(o); } } } }
    Ok(out)
}
fn refunded_order(env: &Env, r: &Row) -> Result<Option<i64>> {
    for l in order_lines(env, r)? { if let Some(o) = id_of(&l, "refunded_orderline_id").and_then(|x| rec(env, "pos.order.line", x).ok()).and_then(|x| id_of(&x, "order_id")) { return Ok(Some(o)); } }
    Ok(None)
}
fn total_cost_computed(env: &Env, r: &Row) -> Result<bool> { Ok(order_lines(env, r)?.iter().all(|l| flag(l, "is_total_cost_computed"))) }
fn order_margin(env: &Env, r: &Row) -> Result<(f64, f64)> {
    if !total_cost_computed(env, r)? { return Ok((0.0, 0.0)); }
    let lines = order_lines(env, r)?;
    let margin: f64 = lines.iter().map(|l| line_margin(env, l).map(|m| m.0).unwrap_or(0.0)).sum();
    let rounding = rounding_of(env, order_currency(env, r));
    let untaxed = float_round(lines.iter().map(|l| num(l, "price_subtotal")).sum(), rounding, "HALF-UP");
    Ok((margin, if is_zero(untaxed, rounding) { 0.0 } else { margin / untaxed }))
}
fn line_margin(env: &Env, l: &Row) -> Result<(f64, f64)> {
    let combo = id_of(l, "product_id").and_then(|p| rec(env, "product.product", p).ok()).and_then(|p| id_of(&p, "product_tmpl_id")).and_then(|t| rec(env, "product.template", t).ok()).and_then(|t| text(&t, "type")).as_deref() == Some("combo");
    if combo { return Ok((0.0, 0.0)); }
    let m = num(l, "price_subtotal") - num(l, "total_cost");
    let rounding = rounding_of(env, id_of(l, "order_id").and_then(|o| rec(env, "pos.order", o).ok()).and_then(|o| order_currency(env, &o)));
    Ok((m, if is_zero(num(l, "price_subtotal"), rounding) { 0.0 } else { m / num(l, "price_subtotal") }))
}
fn order_pickings(env: &Env, r: &Row) -> Result<Vec<Row>> { many(env, "stock.picking", &ids(r, "picking_ids")) }

fn compute_rules() -> Rules {
    Rules::default()
        .compute("pos.order", "tracking_number", |_, r| {
            // (session % 10) * 100 + sequence % 100, zero-padded to 3
            let n = (id_of(r, "session_id").unwrap_or(0) % 10) * 100 + r.get("sequence_number").and_then(|v| v.as_i64()).unwrap_or(0) % 100;
            Ok(Value::Text(format!("{n:03}")))
        })
        .compute("pos.order", "refund_orders_count", |env, r| Ok((refund_orders(env, r)?.len() as i64).into()))
        .compute("pos.order", "refunded_order_id", |env, r| Ok(m2o(env, "pos.order", refunded_order(env, r)?)))
        .compute("pos.order", "has_refundable_lines", |env, r| {
            for l in order_lines(env, r)? { if num(&l, "qty") - line_refunded_qty(env, &l)? > 0.0005 { return Ok(true.into()); } }
            Ok(false.into())
        })
        .compute("pos.order", "is_invoiced", |_, r| Ok(id_of(r, "account_move").is_some().into()))
        .compute("pos.order", "picking_count", |_, r| Ok((ids(r, "picking_ids").len() as i64).into()))
        .compute("pos.order", "failed_pickings", |env, r| Ok(order_pickings(env, r)?.iter().any(|p| text(p, "state").as_deref() != Some("done")).into()))
        .compute("pos.order", "is_total_cost_computed", |env, r| Ok(total_cost_computed(env, r)?.into()))
        .compute("pos.order", "margin", |env, r| Ok(order_margin(env, r)?.0.into()))
        .compute("pos.order", "margin_percent", |env, r| Ok(order_margin(env, r)?.1.into()))
        .compute("pos.order", "is_edited", |env, r| Ok((flag(r, "has_deleted_line") || order_lines(env, r)?.iter().any(|l| flag(l, "is_edited"))).into()))
        // stored: the rate of the order currency at the order date; a rate given at creation (the terminal's) is kept
        .compute("pos.order", "currency_rate", |env, r| {
            if num(r, "currency_rate") > 0.0 { return Ok(num(r, "currency_rate").into()); }
            let comp = id_of(r, "company_id").and_then(|c| rec(env, "res.company", c).ok()).and_then(|c| id_of(&c, "currency_id"));
            let cur = order_currency(env, r);
            let date = text(r, "date_order").unwrap_or_else(orm::now);
            Ok(match (comp, cur) { (Some(a), Some(b)) if a != b => clean(currency_rate_at(env, b, &date) / currency_rate_at(env, a, &date)), _ => 1.0 }.into())
        })
        // stored, editable: filled from the partner while empty
        .compute("pos.order", "email", |env, r| {
            if flag(r, "email") { return Ok(r["email"].clone()); }
            Ok(Value::Text(id_of(r, "partner_id").and_then(|p| rec(env, "res.partner", p).ok()).and_then(|p| text(&p, "email")).unwrap_or_default()))
        })
        .compute("pos.order", "mobile", |env, r| {
            if flag(r, "mobile") { return Ok(r["mobile"].clone()); }
            let p = id_of(r, "partner_id").and_then(|p| rec(env, "res.partner", p).ok()).unwrap_or_default();
            Ok(Value::Text(text(&p, "mobile").filter(|m| !m.is_empty()).or_else(|| text(&p, "phone")).unwrap_or_default()))
        })
        .compute("pos.order.line", "refunded_qty", |env, r| Ok(line_refunded_qty(env, r)?.into()))
        .compute("pos.order.line", "margin", |env, r| Ok(line_margin(env, r)?.0.into()))
        .compute("pos.order.line", "margin_percent", |env, r| Ok(line_margin(env, r)?.1.into()))
        .compute("pos.order.line", "tax_ids_after_fiscal_position", |env, r| Ok(id_list(&line_taxes_after_fpos(env, r)?)))
}

// ---- pricing / payments -----------------------------------------------------------------------------------------------
/// `_compute_prices`: paid/returned amounts from the payments, taxes and total from the lines (cash rounding included).
fn compute_prices(env: &Env, oid: i64) -> Result<()> {
    let e = env.sudo(); let o = rec(&e, "pos.order", oid)?;
    let cfg = order_cfg(&e, &o)?;
    let cur = order_currency(&e, &o); if cur.is_none() { return user_err("You can't: create a pos order from the backend interface, or unset the pricelist, or create a pos.order in a python test with Form tool, or edit the form view in studio if no PoS order exist"); }
    let rounding = rounding_of(&e, cur);
    let pays = children(&e, "pos.payment", "pos_order_id", oid)?;
    let paid = float_round(pays.iter().map(|p| num(p, "amount")).sum(), rounding, "HALF-UP");
    let ret = float_round(-pays.iter().map(|p| num(p, "amount")).filter(|a| *a < 0.0).sum::<f64>(), rounding, "HALF-UP");
    let (mut excl, mut incl) = (0.0, 0.0);
    for l in children(&e, "pos.order.line", "order_id", oid)? { excl += num(&l, "price_subtotal"); incl += num(&l, "price_subtotal_incl"); }
    let mut total = incl;
    if flag(&cfg, "cash_rounding") && !flag(&cfg, "only_round_cash_method") {
        if let Some(rm) = id_of(&cfg, "rounding_method").and_then(|m| rec(&e, "account.cash.rounding", m).ok()) { total = float_round(total, num(&rm, "rounding").max(1e-9), &text(&rm, "rounding_method").unwrap_or_else(|| "HALF-UP".into())); }
    }
    let tax_amount = incl - excl;
    orm::write(&e, "pos.order", &[oid], row(&[("amount_paid", paid.into()), ("amount_return", ret.into()), ("amount_tax", float_round(tax_amount, rounding, "HALF-UP").into()), ("amount_total", float_round(total, rounding, "HALF-UP").into()), ("amount_difference", clean(paid - float_round(total, rounding, "HALF-UP")).into())]))
}
fn uses_rounding(env: &Env, o: &Row, cfg: &Row, force: bool) -> Result<bool> {
    if !flag(cfg, "cash_rounding") { return Ok(false); }
    if force || !flag(cfg, "only_round_cash_method") { return Ok(true); }
    for p in children(env, "pos.payment", "pos_order_id", o["id"].as_i64().unwrap())? { if id_of(&p, "payment_method_id").and_then(|m| rec(env, "pos.payment.method", m).ok()).map_or(false, |m| flag(&m, "is_cash_count")) { return Ok(true); } }
    Ok(false)
}
/// `_get_rounded_amount`.
fn rounded_amount(env: &Env, o: &Row, amount: f64, force: bool) -> Result<f64> {
    let cfg = order_cfg(env, o)?; let mut a = amount;
    if uses_rounding(env, o, &cfg, force)? {
        if let Some(rm) = id_of(&cfg, "rounding_method").and_then(|m| rec(env, "account.cash.rounding", m).ok()) { a = float_round(a, num(&rm, "rounding").max(1e-9), &text(&rm, "rounding_method").unwrap_or_else(|| "HALF-UP".into())); }
    }
    Ok(float_round(a, rounding_of(env, order_currency(env, o)), "HALF-UP"))
}
/// `_is_pos_order_paid`.
fn is_paid(env: &Env, o: &Row) -> Result<bool> {
    let rounding = rounding_of(env, order_currency(env, o));
    let mut total = num(o, "amount_total");
    if let Some(r) = refunded_order(env, o)?.and_then(|r| rec(env, "pos.order", r).ok()) { if is_zero(num(&r, "amount_total") + total, rounding) { total = -num(&r, "amount_paid"); } }
    Ok(is_zero(rounded_amount(env, o, total, false)? - num(o, "amount_paid"), rounding))
}
fn add_payment(env: &Env, oid: i64, mut data: Row) -> Result<i64> {
    data.insert("pos_order_id".into(), oid.into());
    data.entry("payment_date".into()).or_insert(orm::now().into());
    let pid = orm::create(env, "pos.payment", data)?;
    let paid: f64 = children(env, "pos.payment", "pos_order_id", oid)?.iter().map(|p| num(p, "amount")).sum();
    orm::write(env, "pos.order", &[oid], row(&[("amount_paid", paid.into())]))?;
    Ok(pid)
}
/// `action_pos_order_paid`: the order must be fully paid (within the cash rounding tolerance).
fn order_paid(env: &Env, oid: i64) -> Result<()> {
    let e = env.sudo(); let o = rec(&e, "pos.order", oid)?; let cfg = order_cfg(&e, &o)?;
    let cur = order_currency(&e, &o); let rounding = rounding_of(&e, cur);
    let total = if !flag(&cfg, "cash_rounding") || (flag(&cfg, "only_round_cash_method") && !uses_rounding(&e, &o, &cfg, false)?) { num(&o, "amount_total") } else { rounded_amount(&e, &o, num(&o, "amount_total"), true)? };
    let name = text(&o, "name").unwrap_or_default();
    if !is_zero(total - num(&o, "amount_paid"), rounding) {
        if !flag(&cfg, "cash_rounding") { return user_err(format!("Order {name} is not fully paid.")); }
        let rm = id_of(&cfg, "rounding_method").and_then(|m| rec(&e, "account.cash.rounding", m).ok()).unwrap_or_default();
        let step = num(&rm, "rounding"); let max_diff = float_round(if text(&rm, "rounding_method").as_deref() == Some("HALF-UP") { step / 2.0 } else { step }, rounding, "HALF-UP");
        let diff = float_round(num(&o, "amount_total") - num(&o, "amount_paid"), rounding, "HALF-UP");
        if diff.abs() > max_diff + 1e-9 { return user_err(format!("Order {name} is not fully paid.")); }
    }
    orm::write(&e, "pos.order", &[oid], row(&[("state", "paid".into())]))
}

/// The per-config sequence that names orders and lines (Python creates them with the config; here on first use).
fn config_sequence(env: &Env, cfg: &Row, lines: bool) -> Result<String> {
    let e = env.sudo(); let cid = cfg["id"].as_i64().unwrap(); let field = if lines { "sequence_line_id" } else { "sequence_id" };
    let code = format!("{}.config.{cid}", if lines { "pos.order.line" } else { "pos.order" });
    let seq = find_one(&e, "ir.sequence", term("code", "=", code.as_str()))?;
    let name = text(cfg, "name").unwrap_or_default();
    let n = next_seq(&e, &code, &format!("{name}/"), 4)?;
    if id_of(cfg, field).is_none() {
        if let Some(s) = find_one(&e, "ir.sequence", term("code", "=", code.as_str()))? { if seq.is_some() || true { orm::write(&e, "pos.config", &[cid], row(&[(field, s.into())]))?; } }
    }
    Ok(n)
}
/// `_compute_order_name`.
fn order_name(env: &Env, o: &Row) -> Result<String> {
    if let Some(r) = refunded_order(env, o)?.and_then(|r| rec(env, "pos.order", r).ok()) { return Ok(format!("{} REFUND", text(&r, "name").unwrap_or_default())); }
    config_sequence(env, &order_cfg(env, o)?, false)
}

// ---- invoicing --------------------------------------------------------------------------------------------------------
fn prepare_invoice_vals(env: &Env, o: &Row) -> Result<Row> {
    let oid = o["id"].as_i64().unwrap(); let cfg = order_cfg(env, o)?; let rounding = rounding_of(env, order_currency(env, o));
    let sess = rec(env, "pos.session", id_of(o, "session_id").unwrap_or(0))?;
    let date = if text(&sess, "state").as_deref() == Some("closed") { orm::today() } else { text(o, "date_order").map_or_else(orm::today, |d| d.chars().take(10).collect()) };
    let mut refunded_invoices: Vec<i64> = vec![];
    for l in many(env, "pos.order.line", &ids(o, "lines"))? {
        if let Some(m) = id_of(&l, "refunded_orderline_id").and_then(|x| rec(env, "pos.order.line", x).ok()).and_then(|x| id_of(&x, "order_id")).and_then(|x| rec(env, "pos.order", x).ok()).and_then(|x| id_of(&x, "account_move")) { if !refunded_invoices.contains(&m) { refunded_invoices.push(m); } }
    }
    let refund = clean(num(o, "amount_total")) < 0.0 && !is_zero(num(o, "amount_total"), rounding);
    let mut v = row(&[("invoice_origin", text(o, "name").unwrap_or_default().into()), ("move_type", (if refund { "out_refund" } else { "out_invoice" }).into()), ("ref", text(o, "name").unwrap_or_default().into()), ("invoice_date", date.into()), ("invoice_payment_term_id", Value::Null)]);
    if let Some(j) = id_of(&cfg, "invoice_journal_id") { v.insert("journal_id".into(), j.into()); }
    if let Some(p) = id_of(o, "partner_id") { v.insert("partner_id".into(), p.into()); v.insert("partner_shipping_id".into(), p.into()); }
    if let Some(c) = order_currency(env, o) { v.insert("currency_id".into(), c.into()); }
    if let Some(u) = id_of(o, "user_id") { v.insert("invoice_user_id".into(), u.into()); }
    if let Some(f) = id_of(o, "fiscal_position_id") { v.insert("fiscal_position_id".into(), f.into()); }
    if has_model(env, "account.move") && env.reg.field("account.move", "pos_order_ids").is_ok() { v.insert("pos_order_ids".into(), set6(&[oid])); }
    if env.reg.field("account.move", "pos_refunded_invoice_ids").is_ok() && !refunded_invoices.is_empty() { v.insert("pos_refunded_invoice_ids".into(), set6(&refunded_invoices)); }
    if let Some(rm) = refunded_order(env, o)?.and_then(|r| rec(env, "pos.order", r).ok()).and_then(|r| id_of(&r, "account_move")) {
        let rmv = rec(env, "account.move", rm)?;
        v.insert("ref".into(), format!("Reversal of: {}", text(&rmv, "name").unwrap_or_default()).into()); v.insert("reversed_entry_id".into(), rm.into());
    }
    if uses_rounding(env, o, &cfg, false)? { if let Some(rm) = id_of(&cfg, "rounding_method") { v.insert("invoice_cash_rounding_id".into(), rm.into()); } }
    if let Some(n) = text(o, "floating_order_name").filter(|n| !n.is_empty()) { v.insert("narration".into(), n.into()); }
    // pos_sale: the sales team and the addresses/terms of the first sale order being settled
    if env.reg.field("account.move", "team_id").is_ok() && env.reg.field("pos.order", "crm_team_id").is_ok() { if let Some(t) = id_of(o, "crm_team_id") { v.insert("team_id".into(), t.into()); } }
    if let Some(so) = crate::pos_sale_methods::origin_orders(env, o)?.first().and_then(|s| rec(env, "sale.order", *s).ok()) {
        if let Some(ship) = id_of(&so, "partner_shipping_id") { if id_of(&so, "partner_invoice_id") != Some(ship) { v.insert("partner_shipping_id".into(), ship.into()); } }
        let term_ok = id_of(&so, "payment_term_id").and_then(|t| rec(env, "account.payment.term", t).ok()).filter(|t| !flag(t, "early_discount"));
        v.insert("invoice_payment_term_id".into(), term_ok.and_then(|t| t["id"].as_i64()).map_or(Value::Bool(false), Value::Int));
        if let Some(inv) = id_of(&so, "partner_invoice_id") { if Some(inv) != id_of(&so, "partner_id") { v.insert("partner_id".into(), inv.into()); } }
    }
    Ok(v)
}
/// `_prepare_invoice_lines`: one product line per order line (refund invoices carry positive quantities).
fn prepare_invoice_lines(env: &Env, o: &Row) -> Result<Vec<Row>> {
    let mut out = vec![];
    for l in many(env, "pos.order.line", &ids(o, "lines"))? {
        let mut name = text(&l, "full_product_name").filter(|n| !n.is_empty()).or_else(|| text(&l, "name")).unwrap_or_default();
        if id_of(&l, "sale_order_origin_id").is_some() { if let Some(sl) = id_of(&l, "sale_order_line_id").and_then(|s| rec(env, "sale.order.line", s).ok()) { if let Some(n) = text(&sl, "name") { name = n; } } }   // pos_sale: the sale line's description
        let mut v = row(&[("name", name.into()), ("quantity", num(&l, "qty").abs().into()), ("price_unit", num(&l, "price_unit").into()), ("discount", num(&l, "discount").into()), ("display_type", "product".into())]);
        if let Some(p) = id_of(&l, "product_id") { v.insert("product_id".into(), p.into()); }
        let tx = line_taxes_after_fpos(env, &l)?; if !tx.is_empty() { v.insert("tax_ids".into(), set6(&tx)); }
        out.push(v);
    }
    Ok(out)
}
fn create_invoice(env: &Env, oid: i64, mut move_vals: Row, lines: Vec<Row>) -> Result<i64> {
    move_vals.retain(|k, _| env.reg.field("account.move", k).is_ok());
    let mid = orm::create(env, "account.move", move_vals)?;
    for mut l in lines { l.insert("move_id".into(), mid.into()); orm::create(env, "account.move.line", l)?; }
    orm::recompute_ids(env, "account.move", &[mid])?;
    orm::write(env, "pos.order", &[oid], row(&[("account_move", mid.into())]))?;
    Ok(mid)
}
/// `_create_payment_moves` + the reconciliation of `_apply_invoice_payments`: each real payment is booked against the
/// customer's receivable and the invoice residual drops by what was paid.
fn apply_invoice_payments(env: &Env, oid: i64) -> Result<Vec<i64>> {
    let o = rec(env, "pos.order", oid)?; let cfg = order_cfg(env, &o)?; let rounding = rounding_of(env, order_currency(env, &o));
    let mut pays = children(env, "pos.payment", "pos_order_id", oid)?;
    pays.sort_by_key(|p| p["id"].as_i64());
    let kind = |p: &Row| id_of(p, "payment_method_id").and_then(|m| rec(env, "pos.payment.method", m).ok()).map(|m| crate::pos_methods::pm_type(env, &m)).unwrap_or_default();
    // the change given back is netted against the first cash payment
    let change: f64 = pays.iter().filter(|p| flag(p, "is_change") && kind(p) == "cash").map(|p| num(p, "amount")).sum();
    let first_cash = pays.iter().find(|p| !flag(p, "is_change") && kind(p) == "cash").and_then(|p| p["id"].as_i64());
    let has_change = pays.iter().any(|p| flag(p, "is_change") && kind(p) == "cash") && first_cash.is_some();
    let journal = match id_of(&cfg, "journal_id") { Some(j) => j, None => account::ensure_journal(env, "general", "POSS", "Point of Sale")? };
    let recv_partner = account::ensure_account(env, "asset_receivable", "Account Receivable")?;
    let recv_pos = recv_partner;
    let mut moves = vec![]; let mut credited = 0.0;
    for p in &pays {
        if has_change && flag(p, "is_change") && kind(p) == "cash" { continue; }
        if kind(p) == "pay_later" { continue; }
        let amount = if has_change && p["id"].as_i64() == first_cash { num(p, "amount") + change } else { num(p, "amount") };
        if is_zero(amount, rounding) { continue; }
        let pm = id_of(p, "payment_method_id").and_then(|m| rec(env, "pos.payment.method", m).ok()).unwrap_or_default();
        let reference = format!("Invoice payment for {} ({}) using {}", text(&o, "name").unwrap_or_default(), id_of(&o, "account_move").and_then(|m| rec(env, "account.move", m).ok()).and_then(|m| text(&m, "name")).unwrap_or_default(), text(&pm, "name").unwrap_or_default());
        let mv = orm::create(env, "account.move", row(&[("move_type", "entry".into()), ("journal_id", journal.into()), ("ref", reference.as_str().into()), ("date", text(&o, "date_order").map_or_else(orm::today, |d| d.chars().take(10).collect::<String>()).into())]))?;
        let mut line = |acct: i64, bal: f64, partner: Option<i64>| -> Result<()> {
            let (dr, cr) = if bal >= 0.0 { (bal, 0.0) } else { (0.0, -bal) };
            let mut v = row(&[("move_id", mv.into()), ("name", reference.as_str().into()), ("account_id", acct.into()), ("debit", dr.into()), ("credit", cr.into()), ("balance", (dr - cr).into())]);
            if let Some(pp) = partner { v.insert("partner_id".into(), pp.into()); }
            orm::create(env, "account.move.line", v).map(|_| ())
        };
        line(recv_partner, -amount, id_of(&o, "partner_id"))?; line(recv_pos, amount, None)?;
        account::post(env, mv)?;
        orm::write(env, "pos.payment", &[p["id"].as_i64().unwrap()], row(&[("account_move_id", mv.into())]))?;
        credited += amount; moves.push(mv);
    }
    if let Some(inv) = id_of(&o, "account_move") {
        let m = rec(env, "account.move", inv)?;
        let residual = clean((num(&m, "amount_total") - credited.abs()).max(0.0));
        orm::write(env, "account.move", &[inv], row(&[("amount_residual", residual.into())]))?;
    }
    Ok(moves)
}
/// `_generate_pos_order_invoice`: the ids of the invoices (created or already there).
fn generate_invoices(env: &Env, ids_: &[i64]) -> Result<Vec<i64>> {
    let e = env.sudo(); let mut moves = vec![];
    for oid in ids_ {
        let o = rec(&e, "pos.order", *oid)?;
        if let Some(m) = id_of(&o, "account_move") { moves.push(m); continue; }
        if id_of(&o, "partner_id").is_none() { return user_err("Please provide a partner for the sale."); }
        let vals = prepare_invoice_vals(&e, &o)?; let lines = prepare_invoice_lines(&e, &o)?;
        let mid = create_invoice(&e, *oid, vals, lines)?;
        orm::write(&e, "pos.order", &[*oid], row(&[("state", "invoiced".into())]))?;
        account::post(&e, mid)?;
        apply_invoice_payments(&e, *oid)?;
        moves.push(mid);
    }
    Ok(moves)
}

// ---- stock -----------------------------------------------------------------------------------------------------------
/// `_should_create_picking_real_time`.
fn real_time_picking(env: &Env, o: &Row) -> Result<bool> {
    let sess = rec(env, "pos.session", id_of(o, "session_id").unwrap_or(0))?;
    let anglo = id_of(o, "company_id").and_then(|c| rec(env, "res.company", c).ok()).map_or(false, |c| flag(&c, "anglo_saxon_accounting"));
    let from_sale = !crate::pos_sale_methods::origin_orders(env, o)?.is_empty();   // pos_sale: settling a sale order ships right away
    Ok(!flag(&sess, "update_stock_at_closing") || (anglo && flag(o, "to_invoice")) || from_sale)
}
fn create_order_picking(env: &Env, oid: i64) -> Result<()> {
    let e = env.sudo(); let o = rec(&e, "pos.order", oid)?;
    if !ids(&o, "picking_ids").is_empty() || !real_time_picking(&e, &o)? || !has_model(&e, "stock.picking") || flag(&o, "shipping_date") { return Ok(()); }
    let cfg = order_cfg(&e, &o)?; let name = text(&o, "name").unwrap_or_default();
    let src = id_of(&cfg, "picking_type_id").and_then(|t| rec(&e, "stock.picking.type", t).ok()).and_then(|t| id_of(&t, "default_location_src_id"));
    pos_post::move_stock(&e, oid, &name, id_of(&o, "partner_id"), src)?;
    if e.reg.field("stock.picking", "pos_order_id").is_ok() {
        let sid = id_of(&o, "session_id");
        for p in orm::search(&e, "stock.picking", &Domain::And(vec![term("origin", "=", name.as_str()), term("pos_order_id", "=", Value::Bool(false))]), None, None, 0)? {
            let mut w = row(&[("pos_order_id", oid.into())]); if let Some(s) = sid { if e.reg.field("stock.picking", "pos_session_id").is_ok() { w.insert("pos_session_id".into(), s.into()); } }
            orm::write(&e, "stock.picking", &[p], w)?;
        }
    }
    // pos_sale: delivered quantities follow the pickings
    if e.reg.field("pos.order.line", "qty_delivered").is_ok() { orm::recompute_ids(&e, "pos.order.line", &ids(&rec(&e, "pos.order", oid)?, "lines"))?; }
    Ok(())
}

// ---- refunds ---------------------------------------------------------------------------------------------------------
fn refund_orders_from(env: &Env, ids_: &[i64]) -> Result<Vec<i64>> {
    let e = env.sudo(); let mut out = vec![];
    for oid in ids_ {
        let o = rec(&e, "pos.order", *oid)?; let cfg = order_cfg(&e, &o)?;
        let current = crate::pos_methods::current_session(&e, cfg["id"].as_i64().unwrap())?.and_then(|s| s["id"].as_i64());
        let Some(cs) = current else { return user_err(format!("To return product(s), you need to open a session in the POS {}", text(&cfg, "name").unwrap_or_default())); };
        let mut v = row(&[("name", format!("{} REFUND", text(&o, "name").unwrap_or_default()).into()), ("session_id", cs.into()), ("date_order", orm::now().into()), ("pos_reference", o.get("pos_reference").cloned().unwrap_or(Value::Null)),
            ("amount_tax", (-num(&o, "amount_tax")).into()), ("amount_total", (-num(&o, "amount_total")).into()), ("amount_paid", 0.0.into()), ("amount_return", 0.0.into()), ("uuid", new_uuid().into()), ("state", "draft".into())]);
        for k in ["partner_id", "pricelist_id", "fiscal_position_id", "company_id", "user_id", "floating_order_name"] { if let Some(x) = o.get(k) { if !x.is_null() { v.insert(k.into(), x.clone()); } } }
        let ro = orm::create(&e, "pos.order", v)?;
        for l in children(&e, "pos.order.line", "order_id", *oid)? {
            let lid = l["id"].as_i64().unwrap();
            let mut nl = row(&[("name", format!("{} REFUND", text(&l, "name").unwrap_or_default()).into()), ("order_id", ro.into()), ("qty", (-(num(&l, "qty") - line_refunded_qty(&e, &l)?)).into()), ("price_unit", num(&l, "price_unit").into()), ("discount", num(&l, "discount").into()),
                ("price_subtotal", (-num(&l, "price_subtotal")).into()), ("price_subtotal_incl", (-num(&l, "price_subtotal_incl")).into()), ("is_total_cost_computed", false.into()), ("refunded_orderline_id", lid.into()), ("uuid", new_uuid().into())]);
            for k in ["product_id", "full_product_name", "customer_note", "total_cost"] { if let Some(x) = l.get(k) { if !x.is_null() { nl.insert(k.into(), x.clone()); } } }
            let tx = ids(&l, "tax_ids"); if !tx.is_empty() { nl.insert("tax_ids".into(), set6(&tx)); }
            let nid = orm::create(&e, "pos.order.line", nl)?;
            if has_model(&e, "pos.pack.operation.lot") {
                for lot in children(&e, "pos.pack.operation.lot", "pos_order_line_id", lid)? {
                    let mut lv = row(&[("pos_order_line_id", nid.into())]);
                    for k in ["lot_name", "product_id", "order_id"] { if let Some(x) = lot.get(k) { if !x.is_null() { lv.insert(k.into(), x.clone()); } } }
                    lv.insert("order_id".into(), ro.into());
                    lv.retain(|k, _| e.reg.field("pos.pack.operation.lot", k).is_ok());
                    orm::create(&e, "pos.pack.operation.lot", lv)?;
                }
            }
            // _onchange_amount_line_all
            let nr = rec(&e, "pos.order.line", nid)?; let (excl, incl) = line_amounts(&e, &nr)?;
            orm::write(&e, "pos.order.line", &[nid], row(&[("price_subtotal", excl.into()), ("price_subtotal_incl", incl.into())]))?;
        }
        compute_prices(&e, ro)?;
        out.push(ro);
    }
    Ok(out)
}

// ---- sync_from_ui and friends --------------------------------------------------------------------------------------------
fn as_map(v: &Value) -> Row { if let Value::Map(m) = v { m.clone() } else { Row::new() } }
fn commands(order: &Row, field: &str) -> Vec<Value> { match order.get(field) { Some(Value::List(l)) => l.clone(), _ => vec![] } }
fn line_cmd_vals(c: &Value) -> Option<(i64, Row)> { if let Value::List(l) = c { let code = l.first()?.as_i64()?; if (code == 0 || code == 1) && l.len() > 2 { return Some((code, as_map(&l[2]))); } } None }

/// `_get_valid_session`: an open session of the same config for an order that belongs to a session already closing.
fn valid_session(env: &Env, order: &Row) -> Result<i64> {
    let closed = rec(env, "pos.session", order.get("session_id").and_then(|v| v.as_i64()).unwrap_or(0))?;
    let cfg = id_of(&closed, "config_id").unwrap_or(0);
    let open = orm::search(env, "pos.session", &Domain::And(vec![term("state", "not in", Value::List(vec!["closed".into(), "closing_control".into()])), term("config_id", "=", cfg)]), None, Some(1), 0)?;
    open.first().copied().ok_or_else(|| OdooError::User("No open session available. Please open a new session to capture the order.".into()))
}
/// `_prepare_combo_line_uuids`: parent line uuid -> the uuids of its combo children; the client's own links are cleared.
fn prepare_combo_uuids(order: &mut Row) -> BTreeMap<String, Vec<String>> {
    let mut acc: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut lines = commands(order, "lines");
    let all: Vec<Row> = lines.iter().filter_map(|c| line_cmd_vals(c).map(|x| x.1)).collect();
    for c in lines.iter_mut() {
        let Value::List(l) = c else { continue };
        if l.len() < 3 || !matches!(l[0].as_i64(), Some(0 | 1)) { continue; }
        let Value::Map(vals) = &mut l[2] else { continue };
        if let Some(Value::List(child_ids)) = vals.get("combo_line_ids").cloned() {
            if !child_ids.is_empty() {
                let wanted: Vec<i64> = child_ids.iter().filter_map(|v| v.as_i64()).collect();
                let kids: Vec<String> = all.iter().filter(|x| x.get("id").and_then(|i| i.as_i64()).map_or(false, |i| wanted.contains(&i))).filter_map(|x| text(x, "uuid")).collect();
                if let Some(u) = text(vals, "uuid") { acc.insert(u, kids); }
            }
        }
        vals.insert("combo_line_ids".into(), Value::Bool(false)); vals.insert("combo_parent_id".into(), Value::Bool(false));
        vals.remove("id");   // the terminal's temporary id is not a column
    }
    order.insert("lines".into(), Value::List(lines));
    acc
}
fn check_combo_available(env: &Env, order: &Row, combos: &BTreeMap<String, Vec<String>>) -> Result<()> {
    if combos.is_empty() { return Ok(()); }
    let mut product_by_uuid: BTreeMap<String, i64> = BTreeMap::new();
    for c in commands(order, "lines") { if let Some((_, v)) = line_cmd_vals(&c) { if let (Some(u), Some(p)) = (text(&v, "uuid"), v.get("product_id").and_then(|p| p.as_i64())) { product_by_uuid.insert(u, p); } } }
    for (parent, kids) in combos {
        let mut available: BTreeSet<i64> = BTreeSet::new();
        if let Some(t) = product_by_uuid.get(parent).and_then(|p| rec(env, "product.product", *p).ok()).and_then(|p| id_of(&p, "product_tmpl_id")).and_then(|t| rec(env, "product.template", t).ok()) {
            for c in ids(&t, "combo_ids") { for it in children(env, "product.combo.item", "combo_id", c)? { if let Some(p) = id_of(&it, "product_id") { available.insert(p); } } }
        }
        for k in kids {
            let pid = product_by_uuid.get(k).copied();
            if pid.map_or(true, |p| !available.contains(&p)) {
                let name = pid.and_then(|p| orm::display_names(env, "product.product", &[p]).ok()).and_then(|m| m.into_values().next()).unwrap_or_default();
                return user_err(format!("The combo choice '{name}' is no longer available in this combo. Please reload your data."));
            }
        }
    }
    Ok(())
}
fn link_combo_items(env: &Env, oid: i64, combos: &BTreeMap<String, Vec<String>>) -> Result<()> {
    let lines = children(env, "pos.order.line", "order_id", oid)?;
    for (parent, kids) in combos {
        let Some(p) = lines.iter().find(|l| text(l, "uuid").as_deref() == Some(parent.as_str())) else { continue };
        let child_ids: Vec<i64> = lines.iter().filter(|l| text(l, "uuid").map_or(false, |u| kids.contains(&u))).filter_map(|l| l["id"].as_i64()).collect();
        orm::write(env, "pos.order.line", &child_ids, row(&[("combo_parent_id", p["id"].clone())]))?;
    }
    Ok(())
}
fn known_fields(env: &Env, model: &str, mut v: Row) -> Row { v.retain(|k, _| env.reg.field(model, k).is_ok()); v }

/// `_process_payment_lines`: amount paid is recomputed server-side and the change is booked as a negative cash payment.
fn process_payment_lines(env: &Env, oid: i64, order: &Row, session: i64, draft: bool) -> Result<()> {
    let e = env.sudo();
    let paid: f64 = children(&e, "pos.payment", "pos_order_id", oid)?.iter().map(|p| num(p, "amount")).sum();
    orm::write(&e.with_ctx("backend_recomputation", Value::Bool(true)), "pos.order", &[oid], row(&[("amount_paid", paid.into())]))?;
    let ret = num(order, "amount_return");
    let cur = order_currency(&e, &rec(&e, "pos.order", oid)?);
    if !draft && !is_zero(ret, rounding_of(&e, cur)) {
        let s = rec(&e, "pos.session", session)?; let cfg = rec(&e, "pos.config", id_of(&s, "config_id").unwrap_or(0))?;
        let mut pms = many(&e, "pos.payment.method", &ids(&cfg, "payment_method_ids"))?; pms.sort_by_key(|p| (p.get("sequence").and_then(|v| v.as_i64()).unwrap_or(0), p["id"].as_i64().unwrap_or(0)));
        let Some(cash) = pms.iter().find(|p| flag(p, "is_cash_count")) else { return user_err("No cash statement found for this session. Unable to record returned cash.") };
        add_payment(&e, oid, row(&[("name", "return".into()), ("amount", (-ret).into()), ("payment_date", orm::now().into()), ("payment_method_id", cash["id"].clone()), ("is_change", true.into())]))?;
        compute_prices(&e, oid)?;
    }
    Ok(())
}
/// `_process_saved_order`: pay, ship and invoice a freshly synced order. A payment problem leaves the order as a draft.
fn process_saved_order(env: &Env, oid: i64, draft: bool) -> Result<i64> {
    let e = env.sudo(); let o = rec(&e, "pos.order", oid)?;
    if !draft && text(&o, "state").as_deref() != Some("cancel") {
        match order_paid(&e, oid) { Ok(()) => {}, Err(OdooError::Storage(m)) => return Err(OdooError::Storage(m)), Err(_) => {} }
        create_order_picking(&e, oid)?;
    }
    let o = rec(&e, "pos.order", oid)?;
    if flag(&o, "to_invoice") && text(&o, "state").as_deref() == Some("paid") { generate_invoices(&e, &[oid])?; }
    Ok(oid)
}
/// `_process_order`: create or update the pos.order a terminal sent.
fn process_order(env: &Env, mut order: Row, existing: Option<i64>) -> Result<i64> {
    let e = env.sudo();
    let draft = text(&order, "state").as_deref() == Some("draft"); let paid = text(&order, "state").as_deref() == Some("paid");
    if order.contains_key("state") && paid { order.insert("state".into(), "draft".into()); }
    let sess = rec(&e, "pos.session", order.get("session_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("Missing POS session".into()))?)?;
    let orig_session = sess["id"].as_i64().unwrap();
    if matches!(text(&sess, "state").as_deref(), Some("closing_control" | "closed")) { order.insert("session_id".into(), valid_session(&e, &order)?.into()); }
    if let Some(p) = order.get("partner_id").and_then(|v| v.as_i64()) { if rec(&e, "res.partner", p).is_err() { order.insert("partner_id".into(), Value::Bool(false)); order.insert("to_invoice".into(), Value::Bool(false)); } }
    let combos = prepare_combo_uuids(&mut order); check_combo_available(&e, &order, &combos)?;
    let oid = match existing {
        None => {
            let mut v = order.clone(); v.remove("name"); v.insert("pos_reference".into(), order.get("name").cloned().unwrap_or(Value::Bool(false)));
            v.remove("access_token");
            orm::create(&e, "pos.order", known_fields(&e, "pos.order", v))?
        }
        Some(oid) => {
            let cur = rec(&e, "pos.order", oid)?;
            if let Some(s) = order.get("session_id").and_then(|v| v.as_i64()) { if Some(s) != id_of(&cur, "session_id") { orm::write(&e, "pos.order", &[oid], row(&[("session_id", s.into())]))?; } }
            // lines and payments first, so a deleted line cannot break the state change to paid
            for (field, model) in [("lines", "pos.order.line"), ("payment_ids", "pos.payment")] {
                let cmds = commands(&order, field); if cmds.is_empty() { continue; }
                let keep: Vec<Value> = cmds.into_iter().filter(|c| match c { Value::List(l) => { let code = l.first().and_then(|v| v.as_i64()).unwrap_or(-1); let target = l.get(1).and_then(|v| v.as_i64()).unwrap_or(0); !matches!(code, 1 | 2 | 3 | 4) || rec(&e, model, target).is_ok() } _ => true }).collect();
                orm::write(&e, "pos.order", &[oid], row(&[(field, Value::List(keep))]))?;
                order.insert(field.into(), Value::List(vec![]));
            }
            order.remove("uuid"); order.remove("access_token");
            orm::write(&e, "pos.order", &[oid], known_fields(&e, "pos.order", order.clone()))?;
            oid
        }
    };
    link_combo_items(&e, oid, &combos)?;
    for p in children(&e, "pos.payment", "pos_order_id", oid)? { crate::pos_methods::check_payment_method_allowed(&e, &p)?; }
    process_payment_lines(&e, oid, &order, orig_session, draft)?;
    process_saved_order(&e, oid, draft)
}
fn refunded_orders_of(env: &Env, order: &Row) -> Vec<i64> {
    let mut out: Vec<i64> = vec![];
    for c in commands(order, "lines") {
        if let Some((_, v)) = line_cmd_vals(&c) {
            if let Some(o) = v.get("refunded_orderline_id").and_then(|x| x.as_i64()).and_then(|x| rec(env, "pos.order.line", x).ok()).and_then(|x| id_of(&x, "order_id")) { if !out.contains(&o) { out.push(o); } }
        }
    }
    out
}
fn read_pos_data(env: &Env, oids: &[i64]) -> Result<Value> {
    let mut o = vec![]; let mut pays = vec![]; let mut lines = vec![];
    for oid in oids {
        o.push(Value::Map(rec(env, "pos.order", *oid)?));
        for p in children(env, "pos.payment", "pos_order_id", *oid)? { pays.push(Value::Map(p)); }
        for l in children(env, "pos.order.line", "order_id", *oid)? { lines.push(Value::Map(l)); }
    }
    Ok(Value::Map(row(&[("pos.order", Value::List(o)), ("pos.payment", Value::List(pays)), ("pos.order.line", Value::List(lines)), ("pos.session", Value::List(vec![]))])))
}

fn act_window(name: &str, model: &str, mode: &str, res_id: Option<i64>, domain: Option<Vec<i64>>) -> Value {
    let mut m = row(&[("name", name.into()), ("view_mode", mode.into()), ("res_model", model.into()), ("type", "ir.actions.act_window".into())]);
    if let Some(i) = res_id { m.insert("res_id".into(), i.into()); }
    if let Some(d) = domain { m.insert("domain".into(), Value::List(vec![Value::List(vec!["id".into(), "in".into(), id_list(&d)])])); }
    Value::Map(m)
}

pub fn rules() -> Rules {
    compute_rules()
        // create: the session decides the pricelist, fiscal position and company that were not sent
        .before_create("pos.order", |env, mut v| {
            if let Some(s) = v.get("session_id").and_then(|s| s.as_i64()).and_then(|s| rec(env, "pos.session", s).ok()) {
                if let Some(cfg) = id_of(&s, "config_id").and_then(|c| rec(env, "pos.config", c).ok()) {
                    for (k, src) in [("pricelist_id", "pricelist_id"), ("fiscal_position_id", "default_fiscal_position_id"), ("company_id", "company_id")] {
                        if !v.get(k).map_or(false, |x| x.truthy()) { if let Some(x) = id_of(&cfg, src) { v.insert(k.into(), x.into()); } }
                    }
                }
            }
            Ok(v)
        })
        .after_write("pos.order", |env, ids_, vals| {
            let e = env.sudo();
            for id in ids_ {
                let o = rec(&e, "pos.order", *id)?;
                if text(vals, "state").as_deref() == Some("paid") && text(&o, "name").map_or(true, |n| n == "/") {
                    orm::write(&e, "pos.order", &[*id], row(&[("name", order_name(&e, &o)?.into())]))?;
                }
                if vals.contains_key("payment_ids") {
                    compute_prices(&e, *id)?;
                    let o = rec(&e, "pos.order", *id)?; let rounding = rounding_of(&e, order_currency(&e, &o));
                    let diff = num(&o, "amount_paid") - rounded_amount(&e, &o, num(&o, "amount_total"), false)?;
                    if diff < -rounding / 2.0 && matches!(text(&o, "state").as_deref(), Some("paid" | "done" | "invoiced")) { return user_err("The paid amount is different from the total amount of the order."); }
                    if num(&o, "nb_print") > 0.0 { return user_err("You cannot change the payment of a printed order."); }
                }
            }
            Ok(())
        })
        .on_unlink("pos.order", |env, ids_| {
            for i in ids_ { if !matches!(text(&rec(env, "pos.order", *i)?, "state").as_deref(), Some("draft" | "cancel")) { return user_err("In order to delete a sale, it must be new or cancelled."); } }
            Ok(vec![])
        })
        .before_create("pos.order.line", |env, mut v| {
            if !v.get("name").map_or(false, |n| n.truthy()) {
                let cfg = v.get("order_id").and_then(|o| o.as_i64()).and_then(|o| rec(env, "pos.order", o).ok()).and_then(|o| order_cfg(env, &o).ok());
                v.insert("name".into(), match cfg { Some(c) => config_sequence(env, &c, true)?, None => next_seq(&env.sudo(), "pos.order.line", "", 0)? }.into());
            }
            Ok(v)
        })
        .on_unlink("pos.order.line", |env, ids_| {
            for i in ids_ {
                let l = rec(env, "pos.order.line", *i)?;
                let Some(o) = id_of(&l, "order_id").and_then(|o| rec(env, "pos.order", o).ok()) else { continue };
                if !matches!(text(&o, "state").as_deref(), Some("draft" | "cancel")) { return user_err("You can only unlink PoS order lines that are related to orders in new or cancelled state."); }
                if order_cfg(env, &o).map_or(false, |c| flag(&c, "order_edit_tracking")) { orm::write(&env.sudo(), "pos.order", &[o["id"].as_i64().unwrap()], row(&[("has_deleted_line", true.into())]))?; }
            }
            Ok(vec![])
        })
        // ---- actions ----
        .action("pos.order", "_compute_prices", |env, ids_, _| { for i in ids_ { compute_prices(env, *i)?; } Ok(Value::Bool(true)) })
        .action("pos.order", "_onchange_amount_all", |env, ids_, _| { for i in ids_ { compute_prices(env, *i)?; } Ok(Value::Bool(true)) })
        .action("pos.order", "add_payment", |env, ids_, kw| { let oid = oid_of(ids_)?; add_payment(&env.sudo(), oid, kw.clone())?; Ok(Value::Null) })
        .action("pos.order", "_is_pos_order_paid", |env, ids_, _| Ok(is_paid(env, &rec(env, "pos.order", oid_of(ids_)?)?)?.into()))
        .action("pos.order", "_get_rounded_amount", |env, ids_, kw| Ok(rounded_amount(env, &rec(env, "pos.order", oid_of(ids_)?)?, num(kw, "amount"), flag(kw, "force_round"))?.into()))
        .action("pos.order", "action_pos_order_paid", |env, ids_, _| { order_paid(env, oid_of(ids_)?)?; Ok(Value::Bool(true)) })
        .action("pos.order", "_compute_order_name", |env, ids_, _| Ok(Value::Text(order_name(env, &rec(env, "pos.order", oid_of(ids_)?)?)?)))
        .action("pos.order", "action_pos_order_cancel", |env, ids_, _| {
            let e = env.sudo(); let mut done = vec![];
            for i in ids_ { if text(&rec(&e, "pos.order", *i)?, "state").as_deref() == Some("draft") { orm::write(&e, "pos.order", &[*i], row(&[("state", "cancel".into())]))?; done.push(*i); } }
            Ok(Value::Map(row(&[("pos.order", Value::List(done.iter().filter_map(|i| rec(&e, "pos.order", *i).ok()).map(Value::Map).collect()))])))
        })
        .action("pos.order", "remove_from_ui", |env, _, kw| {
            let e = env.sudo(); let want: Vec<i64> = match kw.get("server_ids") { Some(Value::List(l)) => l.iter().filter_map(|v| v.as_i64()).collect(), _ => vec![] };
            let drafts = orm::search(&e, "pos.order", &Domain::And(vec![term("id", "in", id_list(&want)), term("state", "=", "draft")]), None, None, 0)?;
            for o in &drafts {
                orm::write(&e, "pos.order", &[*o], row(&[("state", "cancel".into())]))?;
                for p in children(&e, "pos.payment", "pos_order_id", *o)? { orm::unlink(&e, "pos.payment", &[p["id"].as_i64().unwrap()])?; }
                orm::unlink(&e, "pos.order", &[*o])?;
            }
            Ok(id_list(&drafts))
        })
        .action("pos.order", "_clean_payment_lines", |env, ids_, _| { let e = env.sudo(); let o = oid_of(ids_)?; for p in children(&e, "pos.payment", "pos_order_id", o)? { orm::unlink(&e, "pos.payment", &[p["id"].as_i64().unwrap()])?; } Ok(Value::Null) })
        .action("pos.order", "_prepare_invoice_vals", |env, ids_, _| Ok(Value::Map(prepare_invoice_vals(env, &rec(env, "pos.order", oid_of(ids_)?)?)?)))
        .action("pos.order", "_prepare_invoice_lines", |env, ids_, _| Ok(Value::List(prepare_invoice_lines(env, &rec(env, "pos.order", oid_of(ids_)?)?)?.into_iter().map(Value::Map).collect())))
        .action("pos.order", "_generate_pos_order_invoice", |env, ids_, _| {
            let moves = generate_invoices(env, ids_)?;
            Ok(match moves.first() { Some(m) => Value::Map(row(&[("name", "Customer Invoice".into()), ("view_mode", "form".into()), ("res_model", "account.move".into()), ("type", "ir.actions.act_window".into()), ("target", "current".into()), ("res_id", (*m).into())])), None => Value::Map(Row::new()) })
        })
        .action("pos.order", "action_pos_order_invoice", |env, ids_, _| {
            let e = env.sudo();
            let companies: BTreeSet<i64> = ids_.iter().filter_map(|i| rec(&e, "pos.order", *i).ok().and_then(|o| id_of(&o, "company_id"))).collect();
            if companies.len() > 1 { return user_err("You cannot invoice orders belonging to different companies."); }
            orm::write(&e, "pos.order", ids_, row(&[("to_invoice", true.into())]))?;
            for i in ids_ {
                let o = rec(&e, "pos.order", *i)?;
                if ids(&o, "picking_ids").is_empty() && real_time_picking(&e, &o)? && text(&rec(&e, "pos.session", id_of(&o, "session_id").unwrap_or(0))?, "state").as_deref() != Some("closed") { create_order_picking(&e, *i)?; }
            }
            let moves = generate_invoices(&e, ids_)?;
            Ok(match moves.first() { Some(m) => Value::Map(row(&[("name", "Customer Invoice".into()), ("view_mode", "form".into()), ("res_model", "account.move".into()), ("type", "ir.actions.act_window".into()), ("target", "current".into()), ("res_id", (*m).into())])), None => Value::Map(Row::new()) })
        })
        .action("pos.order", "_apply_invoice_payments", |env, ids_, _| Ok(id_list(&apply_invoice_payments(&env.sudo(), oid_of(ids_)?)?)))
        .action("pos.order", "_create_order_picking", |env, ids_, _| { create_order_picking(env, oid_of(ids_)?)?; Ok(Value::Null) })
        .action("pos.order", "_should_create_picking_real_time", |env, ids_, _| Ok(real_time_picking(env, &rec(env, "pos.order", oid_of(ids_)?)?)?.into()))
        .action("pos.order", "_refund", |env, ids_, _| Ok(id_list(&refund_orders_from(env, ids_)?)))
        .action("pos.order", "_prepare_refund_values", |env, ids_, kw| {
            let o = rec(env, "pos.order", oid_of(ids_)?)?;
            Ok(Value::Map(row(&[("name", format!("{} REFUND", text(&o, "name").unwrap_or_default()).into()), ("session_id", kw.get("current_session").cloned().unwrap_or(Value::Null)), ("date_order", orm::now().into()), ("pos_reference", o.get("pos_reference").cloned().unwrap_or(Value::Null)), ("amount_tax", (-num(&o, "amount_tax")).into()), ("amount_total", (-num(&o, "amount_total")).into()), ("amount_paid", 0.into()), ("is_total_cost_computed", false.into()), ("uuid", new_uuid().into())])))
        })
        .action("pos.order", "_get_valid_session", |env, _, kw| Ok(Value::Int(valid_session(env, &as_map(kw.get("order").unwrap_or(&Value::Null)))?)))
        .action("pos.order", "_get_open_order", |env, _, kw| Ok(crate::pos_restaurant_methods::open_order(env, &as_map(kw.get("order").unwrap_or(&Value::Null)))?.map_or(Value::Bool(false), Value::Int)))
        .action("pos.order", "_get_refunded_orders", |env, _, kw| Ok(id_list(&refunded_orders_of(&env.sudo(), &as_map(kw.get("order").unwrap_or(&Value::Null))))))
        .action("pos.order", "_prepare_combo_line_uuids", |_, _, kw| {
            let mut o = as_map(kw.get("order").unwrap_or(&Value::Null)); let acc = prepare_combo_uuids(&mut o);
            Ok(Value::Map(acc.into_iter().map(|(k, v)| (k, Value::List(v.into_iter().map(Value::Text).collect()))).collect()))
        })
        .action("pos.order", "_check_combo_item_available", |env, _, kw| {
            let mut o = as_map(kw.get("order").unwrap_or(&Value::Null)); let combos = prepare_combo_uuids(&mut o);
            check_combo_available(&env.sudo(), &as_map(kw.get("order").unwrap_or(&Value::Null)), &combos)?; Ok(Value::Null)
        })
        .action("pos.order", "_process_payment_lines", |env, _, kw| {
            let oid = kw.get("pos_order_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("pos_order_id required".into()))?;
            process_payment_lines(env, oid, &as_map(kw.get("order").unwrap_or(&Value::Null)), kw.get("session_id").and_then(|v| v.as_i64()).unwrap_or(0), flag(kw, "draft"))?; Ok(Value::Null)
        })
        .action("pos.order", "_process_saved_order", |env, ids_, kw| Ok(Value::Int(process_saved_order(env, oid_of(ids_)?, flag(kw, "draft"))?)))
        .action("pos.order", "_process_order", |env, _, kw| {
            let order = as_map(kw.get("order").unwrap_or(&Value::Null));
            Ok(Value::Int(process_order(env, order, kw.get("existing_order").and_then(|v| v.as_i64()))?))
        })
        .action("pos.order", "sync_from_ui", |env, _, kw| {
            let e = env.sudo(); let mut done = vec![];
            let orders = match kw.get("orders") { Some(Value::List(l)) => l.clone(), _ => vec![] };
            for o in &orders {
                let order = as_map(o);
                if refunded_orders_of(&e, &order).len() > 1 { return invalid("You can only refund products from the same order."); }
                let existing = match text(&order, "uuid") { Some(u) if !u.is_empty() => crate::pos_restaurant_methods::open_order(&e, &order)?, _ => None };
                match existing {
                    Some(x) if text(&rec(&e, "pos.order", x)?, "state").as_deref() == Some("draft") => done.push(process_order(&e, order, Some(x))?),
                    None => done.push(process_order(&e, order, None)?),
                    Some(x) => done.push(x),   // already settled: the sync is ignored
                }
            }
            crate::pos_sale_methods::after_sync(&e, &done)?;
            let mut data = read_pos_data(&e, &done)?;
            // pos_restaurant: the terminal also gets the other open orders of the tables it works on
            if let Some(Value::List(tables)) = env.ctx.get("table_ids") {
                let tids: Vec<i64> = tables.iter().filter_map(|v| v.as_i64()).collect();
                let extra = crate::pos_restaurant_methods::table_draft_orders(&e, &tids, &done)?;
                if !extra.is_empty() {
                    if let (Value::Map(d), Value::Map(more)) = (&mut data, read_pos_data(&e, &extra)?) { for (k, v) in more { if let (Some(Value::List(a)), Value::List(b)) = (d.get_mut(&k), v) { a.extend(b); } } }
                }
            }
            Ok(data)
        })
        .action("pos.order", "action_view_invoice", |env, ids_, _| { let o = rec(env, "pos.order", oid_of(ids_)?)?; Ok(act_window("Customer Invoice", "account.move", "form", id_of(&o, "account_move"), None)) })
        .action("pos.order", "action_view_refunded_order", |env, ids_, _| { let o = rec(env, "pos.order", oid_of(ids_)?)?; Ok(act_window("Refunded Order", "pos.order", "form", refunded_order(env, &o)?, None)) })
        .action("pos.order", "action_view_refund_orders", |env, ids_, _| { let o = rec(env, "pos.order", oid_of(ids_)?)?; Ok(act_window("Refund Orders", "pos.order", "list,form", None, Some(refund_orders(env, &o)?))) })
        .action("pos.order", "action_stock_picking", |env, ids_, _| { let o = rec(env, "pos.order", oid_of(ids_)?)?; Ok(act_window("Pickings", "stock.picking", "list,form", None, Some(ids(&o, "picking_ids")))) })
        .action("pos.order", "_get_stock_moves", |env, ids_, _| {
            let o = rec(env, "pos.order", oid_of(ids_)?)?; let mut mv = vec![];
            for p in order_pickings(env, &o)? { mv.extend(ids(&p, "move_ids")); }
            Ok(id_list(&mv))
        })
        // ---- order lines ----
        .action("pos.order.line", "_compute_amount_line_all", |env, ids_, _| {
            let (excl, incl) = line_amounts(env, &rec(env, "pos.order.line", oid_of(ids_)?)?)?;
            Ok(Value::Map(row(&[("price_subtotal_incl", incl.into()), ("price_subtotal", excl.into())])))
        })
        .action("pos.order.line", "_onchange_amount_line_all", |env, ids_, _| {
            for i in ids_ { let (excl, incl) = line_amounts(env, &rec(env, "pos.order.line", *i)?)?; orm::write(&env.sudo(), "pos.order.line", &[*i], row(&[("price_subtotal", excl.into()), ("price_subtotal_incl", incl.into())]))?; }
            Ok(Value::Bool(true))
        })
        .action("pos.order.line", "_get_tax_ids_after_fiscal_position", |env, ids_, _| Ok(id_list(&line_taxes_after_fpos(env, &rec(env, "pos.order.line", oid_of(ids_)?)?)?)))
        .action("pos.order.line", "_get_discount_amount", |env, ids_, _| Ok(line_discount_amount(env, &rec(env, "pos.order.line", oid_of(ids_)?)?)?.into()))
        .action("pos.order.line", "isRefund", |env, ids_, _| { let l = rec(env, "pos.order.line", oid_of(ids_)?)?; let _ = env; Ok((num(&l, "qty") * num(&l, "price_unit") < 0.0 && !flag(&l, "is_reward_line")).into()) })
        .action("pos.order.line", "_prepare_refund_data", |env, ids_, kw| {
            let l = rec(env, "pos.order.line", oid_of(ids_)?)?;
            Ok(Value::Map(row(&[("name", format!("{} REFUND", text(&l, "name").unwrap_or_default()).into()), ("qty", (-(num(&l, "qty") - line_refunded_qty(env, &l)?)).into()), ("order_id", kw.get("refund_order_id").cloned().unwrap_or(Value::Null)), ("price_subtotal", (-num(&l, "price_subtotal")).into()), ("price_subtotal_incl", (-num(&l, "price_subtotal_incl")).into()), ("is_total_cost_computed", false.into()), ("refunded_orderline_id", l["id"].clone()), ("uuid", new_uuid().into())])))
        })
        .action("pos.order.line", "_onchange_product_id", |env, ids_, _| {
            // the line's price and taxes follow its product (list price of the product; pricelist rules are not applied)
            let l = rec(env, "pos.order.line", oid_of(ids_)?)?; let Some(p) = id_of(&l, "product_id").and_then(|p| rec(env, "product.product", p).ok()) else { return Ok(Value::Map(Row::new())) };
            let t = id_of(&p, "product_tmpl_id").and_then(|t| rec(env, "product.template", t).ok()).unwrap_or_default();
            let company = id_of(&l, "order_id").and_then(|o| rec(env, "pos.order", o).ok()).and_then(|o| id_of(&o, "company_id"));
            let taxes: Vec<i64> = ids(&t, "taxes_id").into_iter().filter(|x| rec(env, "account.tax", *x).ok().map_or(false, |x| id_of(&x, "company_id") == company)).collect();
            let fp = id_of(&l, "order_id").and_then(|o| rec(env, "pos.order", o).ok()).and_then(|o| id_of(&o, "fiscal_position_id"));
            let mapped = map_tax(env, fp, &taxes)?;
            let price = num(&t, "list_price");
            let (_, _, incl_src) = tax::compute(1.0, price, 0.0, &tax::load(env, &taxes)?);
            let _ = incl_src;
            Ok(Value::Map(row(&[("tax_ids", set6(&taxes)), ("price_unit", price.into()), ("tax_ids_after_fiscal_position", id_list(&mapped))])))
        })
}
