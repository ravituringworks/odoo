//! sale: order/line computes, state transitions, invoicing (incl. down payments) and pricing.
//! Faithful ports of addons/sale/models/{sale_order,sale_order_line,account_move}.py and
//! addons/sale/wizard/{sale_make_invoice_advance,sale_order_cancel,mass_cancel_orders,sale_order_discount}.py.
//!
//! Registered through `patch()` which supersedes the minimal rules of `sale.rs` for the entries re-implemented here.
use crate::pricelist_methods as pl;
use crate::sale_util::*;
use crate::util::*;
use crate::{stock, tax};
use odoo_core::orm::{self, Env};
use odoo_core::{OdooError, Result, Row, Rules, Value};
use std::collections::{BTreeMap, BTreeSet};

pub const NOTHING_TO_INVOICE: &str = "Cannot create an invoice. No items are available to invoice.\n\nTo resolve this issue, please ensure that:\n   \u{2022} The products have been delivered before attempting to invoice them.\n   \u{2022} The invoicing policy of the product is configured correctly.\n\nIf you want to invoice based on ordered quantities instead:\n   \u{2022} For consumable or storable products, open the product, go to the 'General Information' tab and change the 'Invoicing Policy' from 'Delivered Quantities' to 'Ordered Quantities'.\n   \u{2022} For services (and other products), change the 'Invoicing Policy' to 'Prepaid/Fixed Price'.\n";

fn is_real(l: &Row) -> bool { text(l, "display_type").map_or(true, |d| d.is_empty()) }
fn ordered_lines(env: &Env, order: i64) -> Result<Vec<Row>> {
    let mut v = children(env, "sale.order.line", "order_id", order)?;
    v.sort_by_key(|l| (l.get("sequence").and_then(|s| s.as_i64()).unwrap_or(10), l["id"].as_i64().unwrap_or(0)));
    Ok(v)
}
fn rid(r: &Row) -> i64 { r["id"].as_i64().unwrap_or(0) }

// ===================================================================== line calculations

fn line_taxes(env: &Env, l: &Row) -> Result<Vec<tax::Tax>> { load_taxes(env, &ids(l, "tax_id")) }
fn line_amounts(env: &Env, l: &Row) -> Result<(f64, f64, f64)> { Ok(tax::compute(num(l, "product_uom_qty"), num(l, "price_unit"), num(l, "discount"), &line_taxes(env, l)?)) }
fn line_order(env: &Env, l: &Row) -> Result<Row> { rec(env, "sale.order", opt_id(l, "order_id").ok_or_else(|| OdooError::Required("sale.order.line".into(), "order_id".into()))?) }
fn line_in_sale(env: &Env, l: &Row) -> Result<bool> { Ok(text(&line_order(env, l)?, "state").as_deref() == Some("sale")) }
fn line_invoice_policy(env: &Env, l: &Row) -> Option<String> { opt_id(l, "product_id").and_then(|p| tmpl_of(env, p).ok()).and_then(|t| text(&t, "invoice_policy")) }
fn line_product_type(env: &Env, l: &Row) -> Option<String> { opt_id(l, "product_id").and_then(|p| tmpl_of(env, p).ok()).and_then(|t| text(&t, "type")) }

/// Invoice (account.move.line, account.move) pairs linked through `invoice_lines` (`_get_invoice_lines`).
fn invoice_lines_of(env: &Env, l: &Row) -> Result<Vec<(Row, Row)>> {
    let e = env.sudo();
    let mut out = vec![];
    for id in ids(l, "invoice_lines") {
        let aml = rec(&e, "account.move.line", id)?;
        if let Some(m) = opt_id(&aml, "move_id") { out.push((aml, rec(&e, "account.move", m)?)); }
    }
    Ok(out)
}
fn mv_state(m: &Row) -> String { text(m, "state").unwrap_or_default() }
fn legacy(m: &Row) -> bool { text(m, "payment_state").as_deref() == Some("invoicing_legacy") }
fn mv_date(m: &Row) -> String { text(m, "invoice_date").filter(|d| !d.is_empty()).unwrap_or_else(orm::today) }
/// +1 for customer invoices, -1 for credit notes (the sign `_compute_qty_invoiced` applies).
fn inv_sign(m: &Row) -> f64 { match text(m, "move_type").as_deref() { Some("out_invoice") => 1.0, Some("out_refund") => -1.0, _ => 0.0 } }

pub fn calc_qty_invoiced(env: &Env, l: &Row) -> Result<f64> {
    let invs = invoice_lines_of(env, l)?;
    if flag(l, "is_downpayment") {
        // Odoo sums `balance`; the signed untaxed amount is the same quantity before posting.
        let s: f64 = invs.iter().filter(|(_, m)| mv_state(m) != "cancel").map(|(a, m)| inv_sign(m) * num(a, "price_subtotal")).sum();
        return Ok(if cur_is_zero(env, opt_id(l, "currency_id"), s) { 0.0 } else { 1.0 });
    }
    let uom = opt_id(l, "product_uom");
    let mut q = 0.0;
    for (a, m) in &invs {
        if mv_state(m) != "cancel" || legacy(m) {
            let qq = uom_compute_quantity(env, num(a, "quantity"), opt_id(a, "product_uom_id"), uom, false, "UP", true)?;
            q += inv_sign(m) * qq;
        }
    }
    Ok(q)
}
pub fn calc_qty_invoiced_posted(env: &Env, l: &Row) -> Result<f64> {
    let uom = opt_id(l, "product_uom");
    let mut q = 0.0;
    for (a, m) in invoice_lines_of(env, l)? {
        if mv_state(&m) == "posted" || legacy(&m) {
            q += inv_sign(&m) * uom_compute_quantity(env, num(&a, "quantity"), opt_id(&a, "product_uom_id"), uom, true, "UP", true)?;
        }
    }
    Ok(q)
}
pub fn calc_qty_to_invoice(env: &Env, l: &Row) -> Result<f64> {
    if !line_in_sale(env, l)? || !is_real(l) { return Ok(0.0); }
    // combo products (`type == 'combo'`) are not modelled: their qty_to_invoice stays 0
    if line_product_type(env, l).as_deref() == Some("combo") { return Ok(0.0); }
    let invoiced = calc_qty_invoiced(env, l)?;
    Ok(if line_invoice_policy(env, l).as_deref() == Some("order") { num(l, "product_uom_qty") - invoiced } else { num(l, "qty_delivered") - invoiced })
}
pub fn calc_untaxed_amount_invoiced(env: &Env, l: &Row) -> Result<f64> {
    let cur = opt_id(l, "currency_id");
    let mut amt = 0.0;
    for (a, m) in invoice_lines_of(env, l)? {
        if mv_state(&m) == "posted" || legacy(&m) {
            amt += inv_sign(&m) * convert(env, num(&a, "price_subtotal"), opt_id(&a, "currency_id"), cur, &mv_date(&m), true);
        }
    }
    Ok(amt)
}
pub fn calc_untaxed_amount_to_invoice(env: &Env, l: &Row) -> Result<f64> {
    if !line_in_sale(env, l)? { return Ok(0.0); }
    let cur = opt_id(l, "currency_id");
    let qty = if line_invoice_policy(env, l).as_deref() == Some("delivery") { num(l, "qty_delivered") } else { num(l, "product_uom_qty") };
    let price_reduce = num(l, "price_unit") * (1.0 - num(l, "discount") / 100.0);
    let taxes = line_taxes(env, l)?;
    let price_subtotal = if taxes.iter().any(|t| t.price_include) { tax::compute(qty, price_reduce, 0.0, &taxes).0 } else { price_reduce * qty };
    let invs = invoice_lines_of(env, l)?;
    if invs.iter().any(|(a, _)| num(a, "discount") != num(l, "discount")) {
        // re-invoicing with a different discount: recompute what has already been billed from the invoice lines
        let mut amount = 0.0;
        for (a, m) in &invs {
            let ataxes = load_taxes(env, &ids(a, "tax_ids"))?;
            let unit = convert(env, num(a, "price_unit"), opt_id(a, "currency_id"), cur, &text(a, "date").unwrap_or_else(|| mv_date(m)), false) * num(a, "quantity");
            amount += if ataxes.iter().any(|t| t.price_include) { tax::compute(num(a, "quantity"), convert(env, num(a, "price_unit"), opt_id(a, "currency_id"), cur, &mv_date(m), false), 0.0, &ataxes).0 } else { unit };
        }
        Ok((price_subtotal - amount).max(0.0))
    } else {
        Ok(price_subtotal - calc_untaxed_amount_invoiced(env, l)?)
    }
}
pub fn calc_amount_invoiced(env: &Env, l: &Row) -> Result<f64> {
    let cur = opt_id(l, "currency_id");
    let mut amt = 0.0;
    for (a, m) in invoice_lines_of(env, l)? {
        if mv_state(&m) == "posted" || legacy(&m) {
            amt += inv_sign(&m) * convert(env, num(&a, "price_total"), opt_id(&a, "currency_id"), cur, &mv_date(&m), true);
        }
    }
    Ok(amt)
}
pub fn calc_amount_to_invoice(env: &Env, l: &Row) -> Result<f64> {
    let q = num(l, "product_uom_qty");
    if q == 0.0 { return Ok(0.0); }
    let (_, _, total) = line_amounts(env, l)?;
    Ok(total / q * (q - calc_qty_invoiced_posted(env, l)?))
}
pub fn calc_invoice_status(env: &Env, l: &Row) -> Result<String> {
    if !line_in_sale(env, l)? { return Ok("no".into()); }
    let to_inv = calc_qty_to_invoice(env, l)?;
    if flag(l, "is_downpayment") && calc_untaxed_amount_to_invoice(env, l)? == 0.0 { return Ok("invoiced".into()); }
    if !float_is_zero(to_inv, 10f64.powi(-UOM_DIGITS)) { return Ok("to invoice".into()); }
    let ordered = num(l, "product_uom_qty");
    if line_invoice_policy(env, l).as_deref() == Some("order") && ordered >= 0.0 && float_compare(num(l, "qty_delivered"), ordered, UOM_DIGITS) == 1 { return Ok("upselling".into()); }
    if float_compare(calc_qty_invoiced(env, l)?, ordered, UOM_DIGITS) >= 0 { return Ok("invoiced".into()); }
    Ok("no".into())
}
/// `_can_be_invoiced_alone`: not the discount product, and (delivery) not a delivery line.
fn can_be_invoiced_alone(env: &Env, l: &Row) -> bool {
    if flag(l, "is_delivery") { return false; }
    let company = opt_id(l, "company_id").and_then(|c| rec(env, "res.company", c).ok());
    let disc = company.and_then(|c| opt_id(&c, "sale_discount_product_id"));
    disc.is_none() || opt_id(l, "product_id") != disc
}

fn dp_description(env: &Env, l: &Row) -> Result<String> {
    if !is_real(l) { return Ok("Down Payments".into()); }
    let invs = invoice_lines_of(env, l)?;
    let fmt = |d: &str| -> String { let p: Vec<&str> = d.get(..10).unwrap_or(d).split('-').collect(); if p.len() == 3 { format!("{}/{}/{}", p[1], p[2], p[0]) } else { d.to_string() } };
    if invs.iter().all(|(_, m)| mv_state(m) == "draft") {
        let created = text(l, "create_date").unwrap_or_else(orm::now);
        return Ok(format!("Down Payment: {} (Draft)", fmt(&created)));
    }
    if invs.iter().all(|(_, m)| mv_state(m) == "cancel") { return Ok("Down Payment (Cancelled)".into()); }
    let mut moves: BTreeMap<i64, &Row> = BTreeMap::new();
    for (a, m) in &invs { if num(a, "quantity") >= 0.0 && text(m, "move_type").as_deref() == Some("out_invoice") { moves.insert(rid(m), m); } }
    if moves.len() == 1 {
        let m = moves.values().next().unwrap();
        if let (Some(r), Some(d)) = (text(m, "payment_reference").filter(|s| !s.is_empty()), text(m, "invoice_date").filter(|s| !s.is_empty())) { return Ok(format!("Down Payment (ref: {r} on {})", fmt(&d))); }
    }
    Ok("Down Payment".into())
}

// ===================================================================== order calculations

fn order_sum(env: &Env, order: i64) -> Result<(f64, f64)> {
    let (mut u, mut t) = (0.0, 0.0);
    for l in children(env, "sale.order.line", "order_id", order)?.iter().filter(|l| is_real(l)) { let (a, b, _) = line_amounts(env, l)?; u += a; t += b; }
    Ok((r2(u), r2(t)))
}

fn order_invoice_ids(env: &Env, order: i64) -> Result<Vec<i64>> {
    let mut out = BTreeSet::new();
    for l in children(env, "sale.order.line", "order_id", order)? { for (_, m) in invoice_lines_of(env, &l)? { if matches!(text(&m, "move_type").as_deref(), Some("out_invoice" | "out_refund")) { out.insert(rid(&m)); } } }
    Ok(out.into_iter().collect())
}
/// `_compute_amount_paid`: sum of authorized/done payment transactions.
fn order_amount_paid(env: &Env, order: i64) -> Result<f64> {
    if !has_model(env, "payment.transaction") || !has_field(env, "sale.order", "transaction_ids") { return Ok(0.0); }
    let o = rec(env, "sale.order", order)?;
    let mut t = 0.0;
    for id in ids(&o, "transaction_ids") { let tx = rec(env, "payment.transaction", id)?; if matches!(text(&tx, "state").as_deref(), Some("authorized" | "done")) { t += num(&tx, "amount"); } }
    Ok(t)
}
fn calc_order_invoice_status(env: &Env, o: &Row) -> Result<String> {
    if text(o, "state").as_deref() != Some("sale") { return Ok("no".into()); }
    let lines: Vec<Row> = children(env, "sale.order.line", "order_id", rid(o))?.into_iter().filter(|l| !flag(l, "is_downpayment") && is_real(l)).collect();
    let st: Vec<String> = lines.iter().map(|l| text(l, "invoice_status").unwrap_or_else(|| "no".into())).collect();
    Ok(if st.iter().any(|s| s == "to invoice") {
        if st.iter().any(|s| s == "no") {
            let invoiceable: Vec<&Row> = lines.iter().filter(|l| text(l, "invoice_status").as_deref() == Some("to invoice")).collect();
            let special = invoiceable.iter().filter(|l| !can_be_invoiced_alone(env, l)).count();
            if special == invoiceable.len() { "no" } else { "to invoice" }
        } else { "to invoice" }
    } else if !st.is_empty() && st.iter().all(|s| s == "invoiced") { "invoiced" } else if !st.is_empty() && st.iter().all(|s| s == "invoiced" || s == "upselling") { "upselling" } else { "no" }.into())
}

// ===================================================================== pricing

pub struct OrderPricing { pub pricelist: Option<i64>, pub currency: Option<i64>, pub company: Option<i64>, pub fpos: Option<i64>, pub date: String }
pub fn default_pricing(env: &Env) -> OrderPricing { OrderPricing { pricelist: None, currency: pl::env_company_currency(env), company: pl::env_company(env), fpos: None, date: orm::now() } }
pub fn order_pricing(env: &Env, o: &Row) -> OrderPricing {
    OrderPricing { pricelist: opt_id(o, "pricelist_id"), currency: opt_id(o, "currency_id"), company: opt_id(o, "company_id"), fpos: opt_id(o, "fiscal_position_id"), date: text(o, "date_order").filter(|d| !d.is_empty()).unwrap_or_else(|| { let _ = env; orm::now() }) }
}

/// Unrounded untaxed / taxed amounts of one unit at `price` (helper for `_get_tax_included_unit_price_from_price`).
fn excl_of(price: f64, taxes: &[tax::Tax]) -> f64 {
    let fixed: f64 = taxes.iter().filter(|t| t.price_include && t.kind == "fixed").map(|t| t.amount).sum();
    let pct: f64 = taxes.iter().filter(|t| t.price_include && t.kind == "percent").map(|t| t.amount).sum();
    if fixed != 0.0 || pct != 0.0 { (price - fixed) / (1.0 + pct / 100.0) } else { price }
}
fn incl_of(excl: f64, taxes: &[tax::Tax]) -> f64 {
    excl + taxes.iter().map(|t| match t.kind.as_str() { "fixed" => t.amount, "division" => excl / (1.0 - t.amount / 100.0) - excl, _ => excl * t.amount / 100.0 }).sum::<f64>()
}
/// `product.product._get_tax_included_unit_price_from_price`: re-express a tax-included price after a fiscal-position tax change.
pub fn tax_included_unit_price(env: &Env, price: f64, product_taxes: &[i64], fpos: Option<i64>) -> Result<f64> {
    if product_taxes.is_empty() || fpos.is_none() { return Ok(price); }
    let after = map_tax(env, fpos, product_taxes);
    let before_t = load_taxes(env, product_taxes)?;
    let mut a: Vec<i64> = after.clone(); a.sort(); let mut b: Vec<i64> = product_taxes.to_vec(); b.sort();
    if a != b && !before_t.is_empty() && before_t.iter().all(|t| t.price_include) {
        let excl = excl_of(price, &before_t);
        let after_t = load_taxes(env, &after)?;
        return Ok(if after_t.iter().any(|t| t.price_include) { incl_of(excl, &after_t) } else { excl });
    }
    Ok(price)
}

pub struct LinePrice { pub price_unit: f64, pub discount: f64, pub item: Option<i64> }
/// `_compute_pricelist_item_id` + `_get_display_price` + `_reset_price_unit` + `_compute_discount` for one product line.
pub fn price_line(env: &Env, op: &OrderPricing, product: i64, qty: f64, uom: Option<i64>) -> Result<LinePrice> {
    let q = if qty == 0.0 { 1.0 } else { qty };
    let item_id = match op.pricelist { Some(p) => pl::get_product_rule(env, p, product, q, uom, &op.date)?, None => None };
    let item = item_id.map(|i| rec(env, "product.pricelist.item", i)).transpose()?;
    let pricelist_price = pl::item_compute_price(env, item.as_ref(), product, q, uom, &op.date, op.currency)?;
    let show = pl::item_show_discount(env, item.as_ref());
    let display = if !show { pricelist_price } else { pricelist_price.max(pl::item_price_before_discount(env, item.as_ref(), product, q, uom, &op.date, op.currency)?) };
    let tmpl = tmpl_of(env, product)?;
    let ptaxes = filter_taxes_by_company(env, &ids(&tmpl, "taxes_id"), op.company);
    let price_unit = tax_included_unit_price(env, display, &ptaxes, op.fpos)?;
    let mut discount = 0.0;
    if op.pricelist.is_some() && pl::discount_feature_enabled(env) && show {
        let base = pl::item_price_before_discount(env, item.as_ref(), product, q, uom, &op.date, op.currency)?;
        if base != 0.0 { let d = (base - pricelist_price) / base * 100.0; if (d > 0.0 && base > 0.0) || (d < 0.0 && base < 0.0) { discount = d; } }
    }
    Ok(LinePrice { price_unit, discount, item: item_id })
}

pub fn product_line_name(env: &Env, product: i64) -> Result<String> {
    let t = tmpl_of(env, product)?;
    let mut name = product_display_name(env, product)?;
    if let Some(d) = text(&t, "description_sale").filter(|d| !d.is_empty()) { name = format!("{name}\n{d}"); }
    Ok(name)
}
fn line_tax_ids(env: &Env, product: i64, company: Option<i64>, fpos: Option<i64>) -> Result<Vec<i64>> {
    let t = tmpl_of(env, product)?;
    let taxes = filter_taxes_by_company(env, &ids(&t, "taxes_id"), company);
    Ok(map_tax(env, fpos, &taxes))
}

/// Fill the defaults a new/changed product line gets from `_compute_*` (product_uom, name, tax_id, price_unit, discount ...).
fn apply_line_defaults(env: &Env, v: &mut Row, order: Option<&Row>, explicit: bool) -> Result<()> {
    let display = text(v, "display_type").map_or(false, |d| !d.is_empty());
    if display { v.insert("product_uom_qty".into(), 0.0.into()); return Ok(()); }
    let product = v.get("product_id").and_then(|p| match p { Value::List(l) => l.first().and_then(|x| x.as_i64()), o => o.as_i64() }).filter(|p| *p != 0);
    if let Some(pid) = product {
        let t = tmpl_of(env, pid)?;
        if explicit || v.get("product_uom").map_or(true, |u| u.is_null()) { if let Some(u) = opt_id(&t, "uom_id") { v.insert("product_uom".into(), u.into()); } }
        if has_field(env, "sale.order.line", "product_template_id") { if let Some(tid) = opt_id(&rec(env, "product.product", pid)?, "product_tmpl_id") { v.insert("product_template_id".into(), tid.into()); } }
        if explicit || v.get("name").and_then(|n| n.as_str()).map_or(true, |n| n.is_empty()) {
            let tname = order.and_then(|o| crate::sale_mgmt_methods::template_line_name_if_installed(env, o, pid));
            v.insert("name".into(), match tname { Some(n) => n, None => product_line_name(env, pid)? }.into());
        }
        let (company, fpos) = order.map(|o| (opt_id(o, "company_id"), opt_id(o, "fiscal_position_id"))).unwrap_or((None, None));
        if explicit || !v.contains_key("tax_id") { v.insert("tax_id".into(), set6(&line_tax_ids(env, pid, company, fpos)?)); }
        let qty = v.get("product_uom_qty").and_then(|q| q.as_f64());
        let uom = v.get("product_uom").and_then(|u| u.as_i64());
        if explicit || !v.contains_key("price_unit") || !v.contains_key("discount") {
            let op = match order { Some(o) => order_pricing(env, o), None => OrderPricing { pricelist: None, currency: pl::env_company_currency(env), company: pl::env_company(env), fpos: None, date: orm::now() } };
            let lp = price_line(env, &op, pid, qty.unwrap_or(1.0), uom)?;
            if explicit || !v.contains_key("price_unit") { v.insert("price_unit".into(), lp.price_unit.into()); }
            if explicit || !v.contains_key("discount") { v.insert("discount".into(), lp.discount.into()); }
            if has_field(env, "sale.order.line", "pricelist_item_id") { v.insert("pricelist_item_id".into(), idv(lp.item)); }
        }
    } else if flag(v, "is_downpayment") && v.get("name").and_then(|n| n.as_str()).map_or(true, |n| n.is_empty()) {
        v.insert("name".into(), "Down Payment: ".to_string().into());   // refined after creation (needs create_date)
    }
    v.entry("product_uom_qty".into()).or_insert(1.0.into());
    v.entry("name".into()).or_insert("".into());
    if has_field(env, "sale.order.line", "technical_price_unit") && !v.contains_key("technical_price_unit") { if let Some(p) = v.get("price_unit").cloned() { v.insert("technical_price_unit".into(), p); } }
    Ok(())
}

// ===================================================================== refresh plumbing

fn lines_of_amls(env: &Env, amls: &[i64]) -> Vec<i64> {
    let mut out = BTreeSet::new();
    for a in amls { if let Ok(r) = rec(env, "account.move.line", *a) { for l in ids(&r, "sale_line_ids") { out.insert(l); } } }
    out.into_iter().collect()
}
/// Recompute sale lines (invoice-derived quantities/amounts) and their orders.
pub fn refresh_sale_lines(env: &Env, lines: &[i64]) -> Result<()> {
    let e = env.sudo();
    let lines: Vec<i64> = lines.iter().copied().filter(|l| rec(&e, "sale.order.line", *l).is_ok()).collect();
    if lines.is_empty() { return Ok(()); }
    orm::recompute_ids(&e, "sale.order.line", &lines)?;
    let orders: BTreeSet<i64> = lines.iter().filter_map(|l| rec(&e, "sale.order.line", *l).ok().and_then(|r| opt_id(&r, "order_id"))).collect();
    orm::recompute_ids(&e, "sale.order", &orders.into_iter().collect::<Vec<_>>())
}

fn protected_label(f: &str) -> &str { match f { "product_id" => "Product", "name" => "Description", "price_unit" => "Unit Price", "product_uom" => "Unit of Measure", "product_uom_qty" => "Quantity", "tax_id" => "Taxes", "analytic_distribution" => "Analytic Distribution", o => o } }

// ===================================================================== invoicing

fn prepare_invoice(env: &Env, o: &Row) -> Row {
    let m = "account.move";
    let mut v = row(&[("ref", text(o, "client_order_ref").unwrap_or_default().into()), ("move_type", "out_invoice".into())]);
    put(env, m, &mut v, "narration", text(o, "note").map(Value::Text).unwrap_or(Value::Null));
    put(env, m, &mut v, "currency_id", idv(opt_id(o, "currency_id")));
    for k in ["campaign_id", "medium_id", "source_id", "team_id"] { put(env, m, &mut v, k, idv(opt_id(o, k))); }
    put(env, m, &mut v, "partner_id", idv(opt_id(o, "partner_invoice_id").or_else(|| opt_id(o, "partner_id"))));
    put(env, m, &mut v, "partner_shipping_id", idv(opt_id(o, "partner_shipping_id")));
    // `self.fiscal_position_id or self.fiscal_position_id._get_fiscal_position(self.partner_invoice_id)`
    let inv_partner = opt_id(o, "partner_invoice_id").or_else(|| opt_id(o, "partner_id"));
    let fpos = opt_id(o, "fiscal_position_id").or_else(|| inv_partner.and_then(|p| fiscal_position_for(env, p, None).ok().flatten()));
    put(env, m, &mut v, "fiscal_position_id", idv(fpos));
    put(env, m, &mut v, "invoice_origin", text(o, "name").map(Value::Text).unwrap_or(Value::Null));
    put(env, m, &mut v, "invoice_payment_term_id", idv(opt_id(o, "payment_term_id")));
    put(env, m, &mut v, "invoice_user_id", idv(opt_id(o, "user_id")));
    put(env, m, &mut v, "payment_reference", text(o, "reference").filter(|r| !r.is_empty()).map(Value::Text).unwrap_or(Value::Null));
    put(env, m, &mut v, "company_id", idv(opt_id(o, "company_id")));
    put(env, m, &mut v, "user_id", idv(opt_id(o, "user_id")));
    put(env, m, &mut v, "journal_id", idv(opt_id(o, "journal_id")));
    v
}

/// `_prepare_invoice_line` for a sale line (aml values, without `move_id`).
fn prepare_invoice_line(env: &Env, l: &Row, optional: &Row) -> Result<Row> {
    let display = text(l, "display_type").filter(|d| !d.is_empty());
    let qty_to_invoice = calc_qty_to_invoice(env, l)?;
    let name = text(l, "name").unwrap_or_default();
    let pdisp = match opt_id(l, "product_id") { Some(p) => Some(product_display_name(env, p)?), None => None };
    let full = match &pdisp { Some(d) if !d.is_empty() && !name.contains(d.as_str()) => format!("{d}\n{name}"), _ => name };
    let mut res = row(&[("display_type", display.clone().unwrap_or_else(|| "product".into()).into()), ("sequence", l.get("sequence").cloned().unwrap_or(10.into())), ("name", full.into()), ("quantity", qty_to_invoice.into()),
        ("discount", num(l, "discount").into()), ("price_unit", num(l, "price_unit").into()), ("sale_line_ids", link4(rid(l)))]);
    if let Some(p) = opt_id(l, "product_id") { res.insert("product_id".into(), p.into()); }
    if let Some(u) = opt_id(l, "product_uom") { res.insert("product_uom_id".into(), u.into()); }
    let tx = ids(l, "tax_id"); res.insert("tax_ids".into(), set6(&tx));
    res.insert("is_downpayment".into(), flag(l, "is_downpayment").into());
    if flag(l, "is_downpayment") {
        // reuse the account of the first down payment invoice line
        for (a, _) in invoice_lines_of(env, l)? { if flag(&a, "is_downpayment") { if let Some(acc) = opt_id(&a, "account_id") { res.insert("account_id".into(), acc.into()); break; } } }
    }
    for (k, v) in optional { res.insert(k.clone(), v.clone()); }
    if display.is_some() { res.remove("account_id"); res.insert("quantity".into(), 0.0.into()); res.remove("tax_ids"); }
    Ok(res)
}

/// `_get_invoiceable_lines`.
fn invoiceable_lines(env: &Env, order: i64, final_: bool) -> Result<Vec<Row>> {
    let mut dp = vec![]; let mut inv = vec![]; let mut pending: Option<Row> = None;
    let digits = 10f64.powi(-UOM_DIGITS);
    for l in ordered_lines(env, order)? {
        let dt = text(&l, "display_type").unwrap_or_default();
        if dt == "line_section" { pending = Some(l); continue; }
        let q = calc_qty_to_invoice(env, &l)?;
        if dt != "line_note" && float_is_zero(q, digits) { continue; }
        if q > 0.0 || (q < 0.0 && final_) || dt == "line_note" {
            if flag(&l, "is_downpayment") { dp.push(l); continue; }
            if let Some(s) = pending.take() { inv.push(s); }
            inv.push(l);
        }
    }
    inv.extend(dp);
    Ok(inv)
}

fn section_line_vals(sequence: i64) -> Row {
    row(&[("display_type", "line_section".into()), ("name", "Down Payments".into()), ("quantity", 0.0.into()), ("discount", 0.0.into()), ("price_unit", 0.0.into()), ("sequence", sequence.into())])
}

/// Create an account.move with its invoice lines from prepared values.
fn create_move(env: &Env, mut mv: Row, lines: Vec<Row>) -> Result<i64> {
    let e = env.sudo();
    mv.remove("invoice_line_ids");
    let mid = orm::create(&e, "account.move", mv)?;
    for mut l in lines {
        l.insert("move_id".into(), mid.into());
        // only set what the registry knows (is_downpayment / sale_line_ids come from sale itself)
        l.retain(|k, _| has_field(&e, "account.move.line", k));
        orm::create(&e, "account.move.line", l)?;
    }
    orm::recompute_ids(&e, "account.move", &[mid])?;
    Ok(mid)
}

/// `sale.order._create_invoices(grouped, final)`.
pub fn create_invoices(env: &Env, orders: &[i64], grouped: bool, final_: bool, raise_if_nothing: bool) -> Result<Vec<i64>> {
    let e = env.sudo();
    let mut vals_list: Vec<(Row, Vec<Row>, Vec<i64>)> = vec![];   // (invoice vals, line vals, order ids)
    let mut seq = 0i64;
    for oid in orders {
        let o = rec(&e, "sale.order", *oid)?;
        let invoice_vals = prepare_invoice(&e, &o);
        let lines = invoiceable_lines(&e, *oid, final_)?;
        if !lines.iter().any(is_real) { continue; }
        let mut out = vec![]; let mut dp_section = false;
        for l in &lines {
            if !dp_section && flag(l, "is_downpayment") { out.push(section_line_vals(seq)); dp_section = true; seq += 1; }
            let mut lv = prepare_invoice_line(&e, l, &Row::new())?;
            lv.insert("sequence".into(), seq.into()); seq += 1;
            out.push(lv);
        }
        vals_list.push((invoice_vals, out, vec![*oid]));
    }
    if vals_list.is_empty() {
        if raise_if_nothing { return user_error(NOTHING_TO_INVOICE); }
        return Ok(vec![]);
    }
    if !grouped {
        let keys = ["company_id", "partner_id", "currency_id", "fiscal_position_id"];
        let key = |v: &Row| -> Vec<String> { keys.iter().map(|k| v.get(*k).map(|x| x.to_json().to_string()).unwrap_or_default()).collect() };
        vals_list.sort_by_key(|(v, _, _)| key(v));
        let mut merged: Vec<(Row, Vec<Row>, Vec<i64>)> = vec![];
        let mut cur_key: Option<Vec<String>> = None;
        let (mut origins, mut prefs, mut refs): (BTreeSet<String>, BTreeSet<String>, BTreeSet<String>) = Default::default();
        let flush = |merged: &mut Vec<(Row, Vec<Row>, Vec<i64>)>, origins: &mut BTreeSet<String>, prefs: &mut BTreeSet<String>, refs: &mut BTreeSet<String>| {
            if let Some(last) = merged.last_mut() {
                let r: Vec<String> = refs.iter().cloned().collect::<Vec<_>>();
                last.0.insert("ref".into(), r.join(", ").chars().take(2000).collect::<String>().into());
                last.0.insert("invoice_origin".into(), origins.iter().cloned().collect::<Vec<_>>().join(", ").into());
                last.0.insert("payment_reference".into(), if prefs.len() == 1 && !prefs.iter().next().unwrap().is_empty() { prefs.iter().next().unwrap().clone().into() } else { Value::Null });
            }
            origins.clear(); prefs.clear(); refs.clear();
        };
        for (v, l, o) in vals_list {
            let k = key(&v);
            if cur_key.as_ref() != Some(&k) { flush(&mut merged, &mut origins, &mut prefs, &mut refs); merged.push((v.clone(), l, o)); cur_key = Some(k); }
            else { let last = merged.last_mut().unwrap(); last.1.extend(l); last.2.extend(o); }
            origins.insert(text(&v, "invoice_origin").unwrap_or_default());
            prefs.insert(text(&v, "payment_reference").unwrap_or_default());
            refs.insert(text(&v, "ref").unwrap_or_default());
        }
        flush(&mut merged, &mut origins, &mut prefs, &mut refs);
        vals_list = merged;
    }
    // resequence invoice lines when several orders may share one invoice
    if vals_list.len() < orders.len() { for (_, lines, _) in vals_list.iter_mut() { for (i, l) in lines.iter_mut().enumerate() { l.insert("sequence".into(), ((i + 1) as i64).into()); } } }
    let mut moves = vec![];
    for (v, lines, _) in vals_list { moves.push(create_move(&e, v, lines)?); }
    // credit notes: with `final`, a negative total turns the invoice into a refund
    if final_ {
        for m in &moves {
            let mv = rec(&e, "account.move", *m)?;
            if num(&mv, "amount_total") < 0.0 {
                orm::write(&e, "account.move", &[*m], row(&[("move_type", "out_refund".into())]))?;
                for l in children(&e, "account.move.line", "move_id", *m)?.iter().filter(|l| text(l, "display_type").as_deref() == Some("product")) { orm::write(&e, "account.move.line", &[rid(l)], row(&[("quantity", (-num(l, "quantity")).into())]))?; }
                orm::recompute_ids(&e, "account.move", &[*m])?;
            }
        }
    }
    Ok(moves)
}

fn action_view_invoice_value(env: &Env, invoices: &[i64], orders: &[Row]) -> Value {
    let mut a = Row::new();
    if invoices.is_empty() { a.insert("type".into(), "ir.actions.act_window_close".into()); return Value::Map(a.into_iter().collect()); }
    a.insert("type".into(), "ir.actions.act_window".into());
    a.insert("res_model".into(), "account.move".into());
    if invoices.len() > 1 { a.insert("domain".into(), Value::List(vec![Value::List(vec!["id".into(), "in".into(), Value::List(invoices.iter().map(|i| Value::Int(*i)).collect())])])); a.insert("view_mode".into(), "list,form".into()); }
    else { a.insert("res_id".into(), invoices[0].into()); a.insert("view_mode".into(), "form".into()); }
    let mut ctx = row(&[("default_move_type", "out_invoice".into())]);
    if orders.len() == 1 {
        let o = &orders[0];
        ctx.insert("default_partner_id".into(), idv(opt_id(o, "partner_id")));
        ctx.insert("default_partner_shipping_id".into(), idv(opt_id(o, "partner_shipping_id")));
        let term = opt_id(o, "payment_term_id").or_else(|| opt_id(o, "partner_id").and_then(|p| rec(env, "res.partner", p).ok()).and_then(|p| opt_id(&p, "property_payment_term_id")));
        ctx.insert("default_invoice_payment_term_id".into(), idv(term));
    }
    a.insert("context".into(), Value::Map(ctx.into_iter().collect()));
    Value::Map(a.into_iter().collect())
}

// ===================================================================== state transitions

fn confirmation_error(env: &Env, o: &Row) -> Result<Option<String>> {
    if !matches!(text(o, "state").as_deref(), Some("draft" | "sent")) { return Ok(Some("Some orders are not in a state requiring confirmation.".into())); }
    for l in children(env, "sale.order.line", "order_id", rid(o))? {
        if is_real(&l) && !flag(&l, "is_downpayment") && opt_id(&l, "product_id").is_none() { return Ok(Some("A line on these orders missing a product, you cannot confirm it.".into())); }
    }
    Ok(None)
}
fn should_be_locked(env: &Env, o: &Row) -> bool { opt_id(o, "create_uid").map_or(false, |u| user_has_group(env, u, "sale.group_auto_done_setting")) }

fn action_cancel_impl(env: &Env, ids_: &[i64]) -> Result<Value> {
    let e = env.sudo();
    for id in ids_ {
        let drafts: Vec<i64> = order_invoice_ids(&e, *id)?.into_iter().filter(|m| rec(&e, "account.move", *m).map_or(false, |r| mv_state(&r) == "draft")).collect();
        if !drafts.is_empty() { orm::call(&e, "account.move", "button_cancel", &drafts, &Row::new())?; }
    }
    orm::write(&e, "sale.order", ids_, row(&[("state", "cancel".into())]))?;
    Ok(Value::Bool(true))
}
fn truthy_arg(a: &Row, k: &str) -> bool { a.get(k).map_or(false, |v| v.truthy()) }

// ===================================================================== sale.advance.payment.inv

fn wizard_orders(env: &Env, w: &Row) -> Vec<i64> {
    let o = ids(w, "sale_order_ids");
    if !o.is_empty() { return o; }
    match env.ctx.get("active_ids") { Some(Value::List(l)) => l.iter().filter_map(|v| v.as_i64()).collect(), _ => vec![] }
}
fn product_income_account(env: &Env, product: i64, downpayment: bool) -> Option<i64> {
    let t = tmpl_of(env, product).ok()?;
    let categ = opt_id(&t, "categ_id").and_then(|c| rec(env, "product.category", c).ok());
    let cf = |f: &str| categ.as_ref().and_then(|c| opt_id(c, f));
    (if downpayment { cf("property_account_downpayment_categ_id") } else { None }).or_else(|| opt_id(&t, "property_account_income_id")).or_else(|| cf("property_account_income_categ_id"))
}

fn create_down_payment(env: &Env, w: &Row, order: i64) -> Result<i64> {
    let e = env.sudo();
    let o = rec(&e, "sale.order", order)?;
    let all = ordered_lines(&e, order)?;
    let next_seq = all.last().map(|l| l.get("sequence").and_then(|s| s.as_i64()).unwrap_or(10) + 1).unwrap_or(10);
    // down payment section
    if !all.iter().any(|l| !is_real(l) && flag(l, "is_downpayment")) {
        orm::create(&e.with_ctx("sale_no_log_for_new_lines", Value::Bool(true)), "sale.order.line", row(&[("product_uom_qty", 0.0.into()), ("order_id", order.into()), ("display_type", "line_section".into()), ("is_downpayment", true.into()), ("sequence", next_seq.into()), ("name", "Down Payments".into())]))?;
    }
    // one down payment line per tax combination and account
    let cur = opt_id(&o, "currency_id");
    let mut groups: Vec<((Vec<i64>, Option<i64>), f64)> = vec![];
    let mut fixed_taxes_amount = 0.0;
    for l in all.iter().filter(|l| is_real(l) && !flag(l, "is_downpayment")) {
        let (sub, _, _) = line_amounts(&e, l)?;
        let account = opt_id(l, "product_id").and_then(|p| product_income_account(&e, p, true));
        let mut tx: Vec<i64> = vec![];
        for t in ids(l, "tax_id") {
            let tr = rec(&e, "account.tax", t)?;
            if text(&tr, "amount_type").as_deref() == Some("fixed") { if text(&tr, "price_include_override").as_deref() != Some("tax_included") { fixed_taxes_amount += num(&tr, "amount") * num(l, "product_uom_qty"); } } else { tx.push(t); }
        }
        tx.sort();
        match groups.iter_mut().find(|(k, _)| k.0 == tx && k.1 == account) { Some(g) => g.1 += sub, None => groups.push(((tx, account), sub)) }
    }
    let advance = text(w, "advance_payment_method").unwrap_or_else(|| "percentage".into());
    let ratio = if advance == "percentage" { num(w, "amount") / 100.0 } else { let denom = num(&o, "amount_total") - fixed_taxes_amount; if denom != 0.0 { num(w, "fixed_amount") / denom } else { 1.0 } };
    let mut dp_lines: Vec<(i64, Option<i64>)> = vec![];
    for ((tx, account), subtotal) in groups {
        if cur_is_zero(&e, cur, subtotal) { continue; }
        let price = cur_round(&e, cur, subtotal * ratio);
        let id = orm::create(&e.with_ctx("sale_no_log_for_new_lines", Value::Bool(true)), "sale.order.line", row(&[("product_uom_qty", 0.0.into()), ("order_id", order.into()), ("discount", 0.0.into()), ("is_downpayment", true.into()), ("sequence", next_seq.into()), ("tax_id", set6(&tx)), ("price_unit", price.into())]))?;
        dp_lines.push((id, account));
    }
    let name = if advance == "percentage" { format!("Down payment of {:.2}%", num(w, "amount")) } else { "Down Payment".into() };
    let mut inv_lines = vec![];
    for (id, account) in &dp_lines {
        let l = rec(&e, "sale.order.line", *id)?;
        let mut opt = row(&[("name", name.clone().into()), ("quantity", 1.0.into())]);
        if let Some(a) = account { opt.insert("account_id".into(), (*a).into()); }
        inv_lines.push(prepare_invoice_line(&e, &l, &opt)?);
    }
    let mid = create_move(&e, prepare_invoice(&e, &o), inv_lines)?;
    // dp lines now have invoice lines: refresh their quantities/status
    refresh_sale_lines(&e, &dp_lines.iter().map(|d| d.0).collect::<Vec<_>>())?;
    Ok(mid)
}

fn wizard_create_invoices(env: &Env, w: &Row, orders: &[i64]) -> Result<Vec<i64>> {
    let method = text(w, "advance_payment_method").unwrap_or_else(|| "delivered".into());
    if method == "delivered" { return create_invoices(env, orders, !flag(w, "consolidated_billing"), flag(w, "deduct_down_payments"), true); }
    if orders.len() != 1 { return user_error("Expected singleton: sale.order"); }
    Ok(vec![create_down_payment(env, w, orders[0])?])
}

// ===================================================================== address helpers

/// `res.partner.address_get(['invoice' | 'delivery'])`.
pub fn address_get(env: &Env, partner: i64, prefs: &[&str]) -> Result<BTreeMap<String, i64>> {
    let mut want: BTreeSet<String> = prefs.iter().map(|s| s.to_string()).collect(); want.insert("contact".into());
    let mut result: BTreeMap<String, i64> = BTreeMap::new();
    let mut visited: BTreeSet<i64> = BTreeSet::new();
    let mut current = Some(partner);
    while let Some(cp) = current {
        let mut to_scan = vec![cp];
        while !to_scan.is_empty() {
            let r = to_scan.remove(0);
            visited.insert(r);
            let rr = rec(env, "res.partner", r)?;
            let t = text(&rr, "type").unwrap_or_else(|| "contact".into());
            if want.contains(&t) && !result.contains_key(&t) { result.insert(t, r); }
            if result.len() == want.len() { return Ok(result); }
            let mut kids: Vec<i64> = vec![];
            for c in ids(&rr, "child_ids") { if !visited.contains(&c) && !flag(&rec(env, "res.partner", c)?, "is_company") { kids.push(c); } }
            kids.extend(to_scan); to_scan = kids;
        }
        let cr = rec(env, "res.partner", cp)?;
        if flag(&cr, "is_company") || opt_id(&cr, "parent_id").is_none() { break; }
        current = opt_id(&cr, "parent_id");
    }
    let default = result.get("contact").copied().unwrap_or(partner);
    for t in want { result.entry(t).or_insert(default); }
    Ok(result)
}
/// `account.fiscal.position._get_fiscal_position`: the partner's manual position, else the first matching auto-apply one.
pub fn fiscal_position_for(env: &Env, partner: i64, delivery: Option<i64>) -> Result<Option<i64>> {
    if !has_field(env, "res.partner", "property_account_position_id") { return Ok(None); }
    let p = rec(env, "res.partner", partner)?;
    let d = delivery.map(|d| rec(env, "res.partner", d)).transpose()?;
    if let Some(f) = d.as_ref().and_then(|d| opt_id(d, "property_account_position_id")).or_else(|| opt_id(&p, "property_account_position_id")) { return Ok(Some(f)); }
    let Some(country) = opt_id(d.as_ref().unwrap_or(&p), "country_id").or_else(|| opt_id(&p, "country_id")) else { return Ok(None) };
    if !has_model(env, "account.fiscal.position") { return Ok(None); }
    let e = env.sudo();
    let cands = orm::search(&e, "account.fiscal.position", &term("auto_apply", "=", true), Some("sequence, id"), None, 0)?;
    for c in cands {
        let f = rec(&e, "account.fiscal.position", c)?;
        if flag(&f, "vat_required") && text(&p, "vat").map_or(true, |v| v.is_empty()) { continue; }
        let by_country = opt_id(&f, "country_id").map_or(true, |x| x == country);
        let by_group = opt_id(&f, "country_group_id").map_or(true, |g| rec(&e, "res.country.group", g).map_or(false, |g| ids(&g, "country_ids").contains(&country)));
        if by_country && by_group && (opt_id(&f, "country_id").is_some() || opt_id(&f, "country_group_id").is_some()) { return Ok(Some(c)); }
    }
    Ok(None)
}

// ===================================================================== rules

/// Supersede the minimal sale rules with the faithful ports in this file.
pub fn patch(mut base: Rules) -> Rules {
    base.before_create.remove("sale.order.line");
    base.onchange.remove("sale.order.line");
    base.merge(rules())
}

pub fn rules() -> Rules {
    Rules::default()
        // ---- order creation defaults (`_compute_*` with readonly=False, precomputed on create)
        .before_create("sale.order", |env, mut v| {
            let explicit: BTreeSet<String> = v.keys().cloned().collect();
            let company = v.get("company_id").and_then(|c| c.as_i64()).or_else(|| pl::env_company(env));
            if let Some(c) = company { v.entry("company_id".into()).or_insert(c.into()); }
            let crow = company.and_then(|c| rec(env, "res.company", c).ok());
            if let Some(pid) = v.get("partner_id").and_then(|p| p.as_i64()).filter(|p| *p != 0) {
                let partner = rec(env, "res.partner", pid)?;
                let addr = address_get(env, pid, &["invoice", "delivery"])?;
                v.entry("partner_invoice_id".into()).or_insert(addr["invoice"].into());
                v.entry("partner_shipping_id".into()).or_insert(addr["delivery"].into());
                if !v.contains_key("pricelist_id") { if let Some(p) = pl::partner_pricelist(env, pid)? { v.insert("pricelist_id".into(), p.into()); } }
                if !v.contains_key("payment_term_id") { if let Some(t) = opt_id(&partner, "property_payment_term_id") { v.insert("payment_term_id".into(), t.into()); } }
                if !v.contains_key("fiscal_position_id") { let ship = v.get("partner_shipping_id").and_then(|x| x.as_i64()); if let Some(f) = fiscal_position_for(env, pid, ship)? { v.insert("fiscal_position_id".into(), f.into()); } }
                if !v.contains_key("user_id") {
                    let cp = opt_id(&partner, "commercial_partner_id").and_then(|c| rec(env, "res.partner", c).ok());
                    let u = opt_id(&partner, "user_id").or_else(|| cp.and_then(|c| opt_id(&c, "user_id"))).or_else(|| if user_has_group(env, env.uid, "sales_team.group_sale_salesman") { Some(env.uid) } else { None });
                    if let Some(u) = u { v.insert("user_id".into(), u.into()); }
                }
            }
            if !v.contains_key("currency_id") {
                let c = v.get("pricelist_id").and_then(|p| p.as_i64()).and_then(|p| rec(env, "product.pricelist", p).ok()).and_then(|p| opt_id(&p, "currency_id")).or_else(|| company_currency(env, company));
                if let Some(c) = c { v.insert("currency_id".into(), c.into()); }
            }
            if !v.contains_key("currency_rate") && has_field(env, "sale.order", "currency_rate") {
                let cc = company_currency(env, company); let oc = v.get("currency_id").and_then(|c| c.as_i64());
                let date = text(&v, "date_order").unwrap_or_else(orm::now);
                let r = match (cc, oc) { (Some(a), Some(b)) if a != b => cur_rate(env, b, &date) / cur_rate(env, a, &date), _ => 1.0 };
                v.insert("currency_rate".into(), r.into());
            }
            if let Some(c) = &crow {
                if !v.contains_key("validity_date") { let d = c.get("quotation_validity_days").and_then(|d| d.as_i64()).unwrap_or(0); if d > 0 { v.insert("validity_date".into(), orm::shift_date(&orm::today(), d).into()); } }
                if !v.contains_key("require_signature") { v.insert("require_signature".into(), flag(c, "portal_confirmation_sign").into()); }
                if !v.contains_key("require_payment") { v.insert("require_payment".into(), flag(c, "portal_confirmation_pay").into()); }
                if !v.contains_key("prepayment_percent") { v.insert("prepayment_percent".into(), c.get("prepayment_percent").cloned().unwrap_or(1.0.into())); }
            }
            v.entry("locked".into()).or_insert(false.into());
            crate::sale_mgmt_methods::template_defaults(env, &mut v, &explicit)?;
            Ok(v)
        })
        // ---- line creation / form-time defaults (`_compute_product_uom`, `_compute_name`, `_compute_tax_id`, `_compute_price_unit`, `_compute_discount`)
        .before_create("sale.order.line", |env, mut v| {
            let order = v.get("order_id").and_then(|o| o.as_i64()).map(|o| rec(env, "sale.order", o)).transpose()?;
            apply_line_defaults(env, &mut v, order.as_ref(), false)?;
            Ok(v)
        })
        .after_create("sale.order.line", |env, ids_, _| {
            // dp lines: description depends on create_date/invoice state
            let e = env.sudo();
            for id in ids_ { let l = rec(&e, "sale.order.line", *id)?; if flag(&l, "is_downpayment") && opt_id(&l, "product_id").is_none() { let d = dp_description(&e, &l)?; orm::write(&e, "sale.order.line", &[*id], row(&[("name", d.into())]))?; } }
            Ok(())
        })
        .onchange("sale.order.line", |env, mut v| {
            let order = v.get("order_id").and_then(|o| match o { Value::List(l) => l.first().and_then(|x| x.as_i64()), o => o.as_i64() }).map(|o| rec(env, "sale.order", o)).transpose()?;
            let changed_product = v.get("product_id").map_or(false, |p| !p.is_null());
            apply_line_defaults(env, &mut v, order.as_ref(), changed_product)?;
            let (u, tx, tot) = line_amounts(env, &v)?;
            v.insert("price_subtotal".into(), u.into()); v.insert("price_tax".into(), tx.into()); v.insert("price_total".into(), tot.into());
            Ok(v)
        })
        // ---- amounts (price_include aware: supersedes the simplified computes of sale.rs)
        .compute("sale.order.line", "price_subtotal", |env, r| Ok(line_amounts(env, r)?.0.into()))
        .compute("sale.order.line", "price_tax", |env, r| Ok(line_amounts(env, r)?.1.into()))
        .compute("sale.order.line", "price_total", |env, r| Ok(line_amounts(env, r)?.2.into()))
        .compute("sale.order", "amount_untaxed", |env, r| Ok(order_sum(env, rid(r))?.0.into()))
        .compute("sale.order", "amount_tax", |env, r| Ok(order_sum(env, rid(r))?.1.into()))
        .compute("sale.order", "amount_total", |env, r| { let (u, t) = order_sum(env, rid(r))?; Ok(r2(u + t).into()) })
        // ---- line computes
        .compute("sale.order.line", "product_template_id", |env, r| Ok(match opt_id(r, "product_id") { Some(p) => idv(opt_id(&rec(env, "product.product", p)?, "product_tmpl_id")), None => r.get("product_template_id").cloned().unwrap_or(Value::Null) }))
        .compute("sale.order.line", "qty_invoiced", |env, r| Ok(calc_qty_invoiced(env, r)?.into()))
        .compute("sale.order.line", "qty_invoiced_posted", |env, r| Ok(calc_qty_invoiced_posted(env, r)?.into()))
        .compute("sale.order.line", "qty_to_invoice", |env, r| Ok(calc_qty_to_invoice(env, r)?.into()))
        .compute("sale.order.line", "invoice_status", |env, r| Ok(calc_invoice_status(env, r)?.into()))
        .compute("sale.order.line", "untaxed_amount_invoiced", |env, r| Ok(calc_untaxed_amount_invoiced(env, r)?.into()))
        .compute("sale.order.line", "untaxed_amount_to_invoice", |env, r| Ok(calc_untaxed_amount_to_invoice(env, r)?.into()))
        .compute("sale.order.line", "amount_invoiced", |env, r| Ok(calc_amount_invoiced(env, r)?.into()))
        .compute("sale.order.line", "amount_to_invoice", |env, r| Ok(calc_amount_to_invoice(env, r)?.into()))
        .compute("sale.order.line", "price_reduce_taxexcl", |env, r| { let q = num(r, "product_uom_qty"); Ok(if q != 0.0 { (line_amounts(env, r)?.0 / q).into() } else { 0.0.into() }) })
        .compute("sale.order.line", "price_reduce_taxinc", |env, r| { let q = num(r, "product_uom_qty"); Ok(if q != 0.0 { (line_amounts(env, r)?.2 / q).into() } else { 0.0.into() }) })
        .compute("sale.order.line", "is_product_archived", |env, r| Ok(opt_id(r, "product_id").map_or(false, |p| rec(&env.sudo().with_ctx("active_test", Value::Bool(false)), "product.product", p).map_or(false, |p| !p.get("active").map_or(true, |a| a.truthy()))).into()))
        // ---- order computes
        .compute("sale.order", "invoice_status", |env, r| Ok(calc_order_invoice_status(env, r)?.into()))
        .compute("sale.order", "invoice_count", |env, r| Ok((order_invoice_ids(env, rid(r))?.len() as i64).into()))
        .compute("sale.order", "amount_to_invoice", |env, r| { let mut s = 0.0; for l in children(env, "sale.order.line", "order_id", rid(r))? { s += calc_amount_to_invoice(env, &l)?; } Ok(s.into()) })
        .compute("sale.order", "amount_invoiced", |env, r| { let mut s = 0.0; for l in children(env, "sale.order.line", "order_id", rid(r))? { s += calc_amount_invoiced(env, &l)?; } Ok(s.into()) })
        .compute("sale.order", "amount_undiscounted", |env, r| {
            let mut t = 0.0;
            for l in children(env, "sale.order.line", "order_id", rid(r))? { let (sub, _, _) = line_amounts(env, &l)?; t += if num(&l, "discount") != 100.0 { sub * 100.0 / (100.0 - num(&l, "discount")) } else { num(&l, "price_unit") * num(&l, "product_uom_qty") }; }
            Ok(t.into())
        })
        .compute("sale.order", "amount_paid", |env, r| Ok(order_amount_paid(env, rid(r))?.into()))
        .compute("sale.order", "type_name", |_, r| Ok(if matches!(text(r, "state").as_deref(), Some("draft" | "sent" | "cancel")) { "Quotation" } else { "Sales Order" }.into()))
        .compute("sale.order", "is_expired", |_, r| Ok((matches!(text(r, "state").as_deref(), Some("draft" | "sent")) && text(r, "validity_date").filter(|d| !d.is_empty()).map_or(false, |d| date_part(&d) < orm::today())).into()))
        .compute("sale.order", "currency_id", |env, r| {
            let c = opt_id(r, "pricelist_id").and_then(|p| rec(env, "product.pricelist", p).ok()).and_then(|p| opt_id(&p, "currency_id")).or_else(|| company_currency(env, opt_id(r, "company_id"))).or_else(|| opt_id(r, "currency_id"));
            Ok(idv(c))
        })
        .compute("sale.order", "currency_rate", |env, r| {
            let cc = company_currency(env, opt_id(r, "company_id")); let oc = opt_id(r, "currency_id");
            let date = text(r, "date_order").unwrap_or_else(orm::now);
            Ok(match (cc, oc) { (Some(a), Some(b)) if a != b => (cur_rate(env, b, &date) / cur_rate(env, a, &date)).into(), _ => 1.0.into() })
        })
        .compute("sale.order", "has_active_pricelist", |env, r| {
            if !has_model(env, "product.pricelist") { return Ok(false.into()); }
            let c = opt_id(r, "company_id");
            let all = orm::search(&env.sudo(), "product.pricelist", &term("active", "=", true), None, None, 0)?;
            Ok(all.into_iter().any(|p| rec(env, "product.pricelist", p).map_or(false, |p| opt_id(&p, "company_id").map_or(true, |x| Some(x) == c))).into())
        })
        // ---- keep invoice-derived values in sync when account documents change
        .after_create("account.move.line", |env, ids_, vals| { if vals.contains_key("sale_line_ids") { refresh_sale_lines(env, &lines_of_amls(env, ids_))?; } Ok(()) })
        .after_write("account.move.line", |env, ids_, _| refresh_sale_lines(env, &lines_of_amls(env, ids_)))
        .on_unlink("account.move.line", |env, ids_| {
            let lines = lines_of_amls(env, ids_);
            let orders: BTreeSet<i64> = lines.iter().filter_map(|l| rec(env, "sale.order.line", *l).ok().and_then(|r| opt_id(&r, "order_id"))).collect();
            Ok(vec![("sale.order.line".into(), lines), ("sale.order".into(), orders.into_iter().collect())])
        })
        .after_write("account.move", |env, ids_, vals| {
            let e = env.sudo();
            let mut amls = vec![]; for m in ids_ { for l in children(&e, "account.move.line", "move_id", *m)? { amls.push(rid(&l)); } }
            let lines = lines_of_amls(&e, &amls);
            if lines.is_empty() { return Ok(()); }
            refresh_sale_lines(&e, &lines)?;
            // down payment lines follow the state of their invoice (`account_move.py` action_post / button_draft / button_cancel)
            if let Some(st) = vals.get("state").and_then(|s| s.as_str()) {
                for l in &lines {
                    let sl = rec(&e, "sale.order.line", *l)?;
                    if !flag(&sl, "is_downpayment") || !is_real(&sl) { continue; }
                    let mut w = row(&[("name", dp_description(&e, &sl)?.into())]);
                    let locked = flag(&line_order(&e, &sl)?, "locked");
                    if matches!(st, "posted" | "cancel") && !locked {
                        let order = opt_id(&sl, "order_id").unwrap_or(0);
                        let others: BTreeSet<i64> = ordered_lines(&e, order)?.iter().filter(|x| rid(x) != *l && !(flag(x, "is_downpayment"))).flat_map(|x| invoice_lines_of(&e, x).unwrap_or_default().into_iter().map(|(_, m)| rid(&m)).collect::<Vec<_>>()).collect();
                        let price: f64 = invoice_lines_of(&e, &sl)?.iter().filter(|(_, m)| mv_state(m) == "posted" && !others.contains(&rid(m))).map(|(a, m)| if text(m, "move_type").as_deref() == Some("out_invoice") { num(a, "price_unit") } else { -num(a, "price_unit") }).sum();
                        w.insert("price_unit".into(), price.into());
                        if st == "posted" { if let Some((a, _)) = invoice_lines_of(&e, &sl)?.first() { w.insert("tax_id".into(), set6(&ids(a, "tax_ids"))); } }
                    }
                    orm::write(&e, "sale.order.line", &[*l], w)?;
                }
            }
            Ok(())
        })
        // deleting a draft down-payment invoice removes its down-payment lines (account_move.py unlink)
        .on_unlink("account.move", |env, ids_| {
            let e = env.sudo();
            let mut amls = vec![]; for m in ids_ { for l in children(&e, "account.move.line", "move_id", *m)? { amls.push(rid(&l)); } }
            for sl in lines_of_amls(&e, &amls) {
                let r = rec(&e, "sale.order.line", sl)?;
                if !flag(&r, "is_downpayment") || !ids(&r, "invoice_lines").iter().all(|a| amls.contains(a)) { continue; }
                orm::write(&e, "sale.order.line", &[sl], row(&[("invoice_lines", Value::List(vec![Value::List(vec![5.into()])]))]))?;
                let order = opt_id(&r, "order_id");
                orm::unlink(&e, "sale.order.line", &[sl])?;
                if let Some(o) = order { orm::recompute_ids(&e, "sale.order", &[o])?; }
            }
            Ok(vec![])
        })
        // ---- order hooks
        .after_write("sale.order", |env, ids_, vals| {
            let e = env.sudo();
            if vals.get("pricelist_id").is_some() {
                for id in ids_ { if text(&rec(&e, "sale.order", *id)?, "state").as_deref() == Some("sale") { return user_error("You cannot change the pricelist of a confirmed order !"); } }
            }
            if let Some(st) = vals.get("state") {
                for id in ids_ {
                    let lines: Vec<i64> = children(&e, "sale.order.line", "order_id", *id)?.iter().map(rid).collect();
                    if !lines.is_empty() { orm::write(&e, "sale.order.line", &lines, row(&[("state", st.clone())]))?; orm::recompute_ids(&e, "sale.order.line", &lines)?; }
                }
                orm::recompute_ids(&e, "sale.order", ids_)?;
            }
            Ok(())
        })
        .on_unlink("sale.order", |env, ids_| {
            for id in ids_ { if !matches!(text(&rec(env, "sale.order", *id)?, "state").as_deref(), Some("draft" | "cancel")) { return user_error("You can not delete a sent quotation or a confirmed sales order. You must first cancel it."); } }
            Ok(vec![])
        })
        // ---- line hooks
        .on_unlink("sale.order.line", |env, ids_| {
            let mut orders = BTreeSet::new();
            for i in ids_ {
                let l = rec(env, "sale.order.line", *i)?;
                let o = line_order(env, &l)?;
                // `_check_line_unlink` (+ delivery lines are always deletable)
                if text(&o, "state").as_deref() == Some("sale") && (!ids(&l, "invoice_lines").is_empty() || !flag(&l, "is_downpayment")) && is_real(&l) && !flag(&l, "is_delivery") {
                    return user_error("Once a sales order is confirmed, you can't remove one of its lines (we need to track if something gets invoiced or delivered).\n                Set the quantity to 0 instead.");
                }
                orders.insert(rid(&o));
            }
            Ok(vec![("sale.order".into(), orders.into_iter().collect())])
        })
        .after_write("sale.order.line", |env, ids_, vals| {
            const PROTECTED: [&str; 7] = ["product_id", "name", "price_unit", "product_uom", "product_uom_qty", "tax_id", "analytic_distribution"];
            let touched: Vec<&str> = PROTECTED.iter().copied().filter(|f| vals.contains_key(*f)).collect();
            if touched.is_empty() { return Ok(()); }
            let mut locked = false; let mut all_dp = true;
            for id in ids_ { let l = rec(env, "sale.order.line", *id)?; if flag(&line_order(env, &l)?, "locked") { locked = true; } if !flag(&l, "is_downpayment") { all_dp = false; } }
            if !locked { return Ok(()); }
            let fields: Vec<&str> = touched.into_iter().filter(|f| !(*f == "name" && all_dp)).collect();
            if fields.is_empty() { return Ok(()); }
            user_error(format!("It is forbidden to modify the following fields in a locked order:\n{}", fields.iter().map(|f| protected_label(f)).collect::<Vec<_>>().join("\n")))
        })
        // ---- order actions
        .action("sale.order", "action_confirm", |env, ids_, args| {
            let e = env.sudo();
            for id in ids_ { if let Some(m) = confirmation_error(&e, &rec(&e, "sale.order", *id)?)? { return user_error(m); } }
            orm::write(&e, "sale.order", ids_, row(&[("state", "sale".into()), ("date_order", orm::now().into())]))?;
            // sale_stock `_action_confirm`: create the delivery
            if env.reg.model("stock.picking").is_ok() && env.reg.field("stock.move", "sale_line_id").is_ok() { for id in ids_ { stock::deliver_sale_order(&e, *id)?; } }
            for id in ids_ { if should_be_locked(&e, &rec(&e, "sale.order", *id)?) { orm::write(&e, "sale.order", &[*id], row(&[("locked", true.into())]))?; } }
            let _ = args;
            Ok(Value::Bool(true))
        })
        .action("sale.order", "_confirmation_error_message", |env, ids_, _| { let o = rec(env, "sale.order", *ids_.first().ok_or_else(|| OdooError::User("Expected singleton".into()))?)?; Ok(confirmation_error(env, &o)?.map(Value::Text).unwrap_or(Value::Bool(false))) })
        .action("sale.order", "action_draft", |env, ids_, _| {
            let e = env.sudo();
            let todo: Vec<i64> = ids_.iter().copied().filter(|i| rec(&e, "sale.order", *i).map_or(false, |o| matches!(text(&o, "state").as_deref(), Some("cancel" | "sent")))).collect();
            let mut w = row(&[("state", "draft".into())]);
            for k in ["signature", "signed_by", "signed_on"] { if has_field(&e, "sale.order", k) { w.insert(k.into(), Value::Null); } }
            orm::write(&e, "sale.order", &todo, w)?;
            Ok(Value::Bool(true))
        })
        .action("sale.order", "action_quotation_sent", |env, ids_, _| {
            let e = env.sudo();
            for id in ids_ { if text(&rec(&e, "sale.order", *id)?, "state").as_deref() != Some("draft") { return user_error("Only draft orders can be marked as sent directly."); } }
            orm::write(&e, "sale.order", ids_, row(&[("state", "sent".into())]))?; Ok(Value::Bool(true))
        })
        .action("sale.order", "action_lock", |env, ids_, _| { orm::write(&env.sudo(), "sale.order", ids_, row(&[("locked", true.into())]))?; Ok(Value::Bool(true)) })
        .action("sale.order", "action_unlock", |env, ids_, _| { orm::write(&env.sudo(), "sale.order", ids_, row(&[("locked", false.into())]))?; Ok(Value::Bool(true)) })
        .action("sale.order", "_show_cancel_wizard", |env, ids_, _| {
            if env.ctx.get("disable_cancel_warning").map_or(false, |v| v.truthy()) { return Ok(false.into()); }
            for id in ids_ { if text(&rec(env, "sale.order", *id)?, "state").as_deref() != Some("draft") { return Ok(true.into()); } }
            Ok(false.into())
        })
        .action("sale.order", "action_cancel", |env, ids_, args| {
            let e = env.sudo();
            for id in ids_ { if flag(&rec(&e, "sale.order", *id)?, "locked") { return user_error("You cannot cancel a locked order. Please unlock it first."); } }
            let skip = env.ctx.get("disable_cancel_warning").map_or(false, |v| v.truthy()) || truthy_arg(args, "disable_cancel_warning");
            let show = !skip && { let mut any = false; for id in ids_ { if text(&rec(&e, "sale.order", *id)?, "state").as_deref() != Some("draft") { any = true; } } any };
            if show {
                if ids_.len() != 1 { return user_error("Expected singleton: sale.order"); }
                let o = rec(&e, "sale.order", ids_[0])?;
                let type_name = if matches!(text(&o, "state").as_deref(), Some("draft" | "sent" | "cancel")) { "Quotation" } else { "Sales Order" };
                return Ok(Value::Map(row(&[("name", format!("Cancel {type_name}").into()), ("view_mode", "form".into()), ("res_model", "sale.order.cancel".into()), ("type", "ir.actions.act_window".into()), ("target", "new".into()),
                    ("context", Value::Map(row(&[("default_order_id", ids_[0].into()), ("mark_so_as_canceled", true.into())]).into_iter().collect()))]).into_iter().collect()));
            }
            action_cancel_impl(env, ids_)
        })
        .action("sale.order", "_action_cancel", |env, ids_, _| action_cancel_impl(env, ids_))
        .action("sale.order", "action_view_invoice", |env, ids_, args| {
            let mut inv: Vec<i64> = match args.get("invoices") { Some(Value::List(l)) => l.iter().filter_map(|v| v.as_i64()).collect(), _ => vec![] };
            let mut orders = vec![];
            for id in ids_ { orders.push(rec(env, "sale.order", *id)?); }
            if inv.is_empty() { let mut s = BTreeSet::new(); for id in ids_ { s.extend(order_invoice_ids(env, *id)?); } inv = s.into_iter().collect(); }
            Ok(action_view_invoice_value(env, &inv, &orders))
        })
        .action("sale.order", "_prepare_invoice", |env, ids_, _| { let o = rec(env, "sale.order", *ids_.first().ok_or_else(|| OdooError::User("Expected singleton: sale.order".into()))?)?; Ok(Value::Map(prepare_invoice(env, &o).into_iter().collect())) })
        .action("sale.order", "_get_invoiceable_lines", |env, ids_, a| { let l = invoiceable_lines(env, *ids_.first().ok_or_else(|| OdooError::User("Expected singleton: sale.order".into()))?, truthy_arg(a, "final"))?; Ok(Value::List(l.iter().map(|r| Value::Int(rid(r))).collect())) })
        .action("sale.order", "_nothing_to_invoice_error_message", |_, _, _| Ok(NOTHING_TO_INVOICE.into()))
        .action("sale.order", "_create_invoices", |env, ids_, a| {
            let raise = env.ctx.get("raise_if_nothing_to_invoice").map_or(true, |v| v.truthy());
            Ok(Value::List(create_invoices(env, ids_, truthy_arg(a, "grouped"), truthy_arg(a, "final"), raise)?.into_iter().map(Value::Int).collect()))
        })
        .action("sale.order", "_get_prepayment_required_amount", |env, ids_, _| {
            let o = rec(env, "sale.order", *ids_.first().ok_or_else(|| OdooError::User("Expected singleton: sale.order".into()))?)?;
            Ok(if num(&o, "prepayment_percent") == 1.0 || !flag(&o, "require_payment") { num(&o, "amount_total") } else { cur_round(env, opt_id(&o, "currency_id"), num(&o, "amount_total") * num(&o, "prepayment_percent")) }.into())
        })
        .action("sale.order", "_is_confirmation_amount_reached", |env, ids_, _| {
            let o = rec(env, "sale.order", *ids_.first().ok_or_else(|| OdooError::User("Expected singleton: sale.order".into()))?)?;
            let req = if num(&o, "prepayment_percent") == 1.0 || !flag(&o, "require_payment") { num(&o, "amount_total") } else { cur_round(env, opt_id(&o, "currency_id"), num(&o, "amount_total") * num(&o, "prepayment_percent")) };
            Ok((cur_round(env, opt_id(&o, "currency_id"), req) <= cur_round(env, opt_id(&o, "currency_id"), order_amount_paid(env, rid(&o))?) + cur_rounding(env, opt_id(&o, "currency_id")) / 2.0).into())
        })
        .action("sale.order", "_generate_downpayment_invoices", |env, ids_, a| {
            let mut out = vec![];
            for id in ids_ {
                let o = rec(env, "sale.order", *id)?;
                let amount = env.ctx.get("downpayment_fixed_amount").and_then(|v| v.as_f64()).or_else(|| a.get("downpayment_fixed_amount").and_then(|v| v.as_f64())).unwrap_or_else(|| num(&o, "amount_paid"));
                let w = row(&[("advance_payment_method", "fixed".into()), ("fixed_amount", amount.into()), ("sale_order_ids", set6(&[*id]))]);
                out.extend(wizard_create_invoices(env, &w, &[*id])?);
            }
            Ok(Value::List(out.into_iter().map(Value::Int).collect()))
        })
        .action("sale.order", "action_update_taxes", |env, ids_, _| {
            let e = env.sudo();
            for id in ids_ {
                let o = rec(&e, "sale.order", *id)?;
                for l in children(&e, "sale.order.line", "order_id", *id)?.iter().filter(|l| is_real(l)) {
                    if let Some(p) = opt_id(l, "product_id") { orm::write(&e, "sale.order.line", &[rid(l)], row(&[("tax_id", set6(&line_tax_ids(&e, p, opt_id(&o, "company_id"), opt_id(&o, "fiscal_position_id"))?))]))?; }
                    else { orm::write(&e, "sale.order.line", &[rid(l)], row(&[("tax_id", set6(&[]))]))?; }
                }
                if has_field(&e, "sale.order", "show_update_fpos") { let _ = orm::write(&e, "sale.order", &[*id], row(&[("show_update_fpos", false.into())])); }
            }
            Ok(Value::Bool(true))
        })
        .action("sale.order", "action_update_prices", |env, ids_, _| {
            let e = env.sudo();
            for id in ids_ {
                let o = rec(&e, "sale.order", *id)?;
                let op = order_pricing(&e, &o);
                for l in children(&e, "sale.order.line", "order_id", *id)?.iter().filter(|l| is_real(l) && !flag(l, "is_delivery")) {
                    let Some(p) = opt_id(l, "product_id") else { continue };
                    let mut w = Row::new();
                    // `_compute_price_unit` leaves already-invoiced lines alone
                    if calc_qty_invoiced(&e, l)? <= 0.0 && opt_id(l, "product_uom").is_some() {
                        let lp = price_line(&e, &op, p, num(l, "product_uom_qty"), opt_id(l, "product_uom"))?;
                        w.insert("price_unit".into(), lp.price_unit.into());
                        if has_field(&e, "sale.order.line", "technical_price_unit") { w.insert("technical_price_unit".into(), lp.price_unit.into()); }
                        w.insert("discount".into(), lp.discount.into());
                        if has_field(&e, "sale.order.line", "pricelist_item_id") { w.insert("pricelist_item_id".into(), idv(lp.item)); }
                    } else {
                        let lp = price_line(&e, &op, p, num(l, "product_uom_qty"), opt_id(l, "product_uom"))?;
                        w.insert("discount".into(), lp.discount.into());
                    }
                    orm::write(&e, "sale.order.line", &[rid(l)], w)?;
                }
                crate::sale_mgmt_methods::recompute_option_prices(&e, *id)?;
                if has_field(&e, "sale.order", "show_update_pricelist") { let _ = orm::write(&e, "sale.order", &[*id], row(&[("show_update_pricelist", false.into())])); }
            }
            Ok(Value::Bool(true))
        })
        // ---- line actions / helpers callable from the UI
        .action("sale.order.line", "_get_invoice_line_sequence", |_, _, a| { let n = a.get("new").and_then(|v| v.as_i64()).unwrap_or(0); let o = a.get("old").and_then(|v| v.as_i64()).unwrap_or(0); Ok((if n != 0 { n } else { o }).into()) })
        .action("sale.order.line", "_prepare_invoice_line", |env, ids_, a| { let l = rec(env, "sale.order.line", *ids_.first().ok_or_else(|| OdooError::User("Expected singleton: sale.order.line".into()))?)?; Ok(Value::Map(prepare_invoice_line(env, &l, a)?.into_iter().collect())) })
        .action("sale.order.line", "_can_be_invoiced_alone", |env, ids_, _| { let l = rec(env, "sale.order.line", *ids_.first().ok_or_else(|| OdooError::User("Expected singleton".into()))?)?; Ok(can_be_invoiced_alone(env, &l).into()) })
        // ---- wizards
        .action("sale.order.cancel", "action_cancel", |env, ids_, _| {
            let e = env.sudo();
            for id in ids_ { let w = rec(&e, "sale.order.cancel", *id)?; if let Some(o) = opt_id(&w, "order_id") { orm::call(&e.with_ctx("disable_cancel_warning", Value::Bool(true)), "sale.order", "action_cancel", &[o], &Row::new())?; } }
            Ok(Value::Bool(true))
        })
        .action("sale.mass.cancel.orders", "action_mass_cancel", |env, ids_, _| {
            let e = env.sudo();
            for id in ids_ { let w = rec(&e, "sale.mass.cancel.orders", *id)?; let os = wizard_orders(&e, &w); action_cancel_impl(&e, &os)?; }
            Ok(Value::Bool(true))
        })
        .action("sale.advance.payment.inv", "create_invoices", |env, ids_, _| {
            let e = env.sudo();
            let w = rec(&e, "sale.advance.payment.inv", *ids_.first().ok_or_else(|| OdooError::User("Expected singleton".into()))?)?;
            let method = text(&w, "advance_payment_method").unwrap_or_else(|| "delivered".into());
            if (method == "percentage" && num(&w, "amount") <= 0.0) || (method == "fixed" && num(&w, "fixed_amount") <= 0.0) { return user_error("The value of the down payment amount must be positive."); }
            let orders = wizard_orders(&e, &w);
            let moves = wizard_create_invoices(&e, &w, &orders)?;
            let mut rows = vec![]; for o in &orders { rows.push(rec(&e, "sale.order", *o)?); }
            Ok(action_view_invoice_value(&e, &moves, &rows))
        })
        .action("sale.advance.payment.inv", "_create_invoices", |env, ids_, a| {
            let e = env.sudo();
            let w = rec(&e, "sale.advance.payment.inv", *ids_.first().ok_or_else(|| OdooError::User("Expected singleton".into()))?)?;
            let orders: Vec<i64> = match a.get("sale_order_ids") { Some(Value::List(l)) => l.iter().filter_map(|v| v.as_i64()).collect(), _ => wizard_orders(&e, &w) };
            Ok(Value::List(wizard_create_invoices(&e, &w, &orders)?.into_iter().map(Value::Int).collect()))
        })
        .compute("sale.advance.payment.inv", "count", |env, r| Ok((wizard_orders(env, r).len() as i64).into()))
        .compute("sale.advance.payment.inv", "has_down_payments", |env, r| { for o in wizard_orders(env, r) { if children(env, "sale.order.line", "order_id", o)?.iter().any(|l| flag(l, "is_downpayment")) { return Ok(true.into()); } } Ok(false.into()) })
        .action("sale.order.discount", "action_apply_discount", |env, ids_, _| apply_discount(env, ids_))
}

// ===================================================================== sale.order.discount wizard

fn apply_discount(env: &Env, ids_: &[i64]) -> Result<Value> {
    let e = env.sudo();
    for wid in ids_ {
        let w = rec(&e, "sale.order.discount", *wid)?;
        let order = opt_id(&w, "sale_order_id").or_else(|| env.ctx.get("active_id").and_then(|v| v.as_i64())).ok_or_else(|| OdooError::Required("sale.order.discount".into(), "sale_order_id".into()))?;
        let dtype = text(&w, "discount_type").unwrap_or_else(|| "sol_discount".into());
        if matches!(dtype.as_str(), "sol_discount" | "so_discount") && num(&w, "discount_percentage") > 1.0 { return Err(OdooError::Validation("Invalid discount amount".into())); }
        if dtype == "sol_discount" {
            let lines: Vec<i64> = children(&e, "sale.order.line", "order_id", order)?.iter().map(rid).collect();
            orm::write(&e, "sale.order.line", &lines, row(&[("discount", (num(&w, "discount_percentage") * 100.0).into())]))?;
            continue;
        }
        create_discount_lines(&e, &w, order, &dtype)?;
    }
    Ok(Value::Bool(true))
}

fn create_discount_lines(e: &Env, w: &Row, order: i64, dtype: &str) -> Result<()> {
    let o = rec(e, "sale.order", order)?;
    let company = opt_id(&o, "company_id");
    let crow = company.map(|c| rec(e, "res.company", c)).transpose()?.unwrap_or_default();
    let product = match opt_id(&crow, "sale_discount_product_id") {
        Some(p) => p,
        None => {
            // `_get_discount_product`: create the company's discount service product on first use
            let p = orm::create(e, "product.product", row(&[("name", "Discount".into()), ("type", "service".into()), ("invoice_policy", "order".into()), ("list_price", 0.0.into())]))?;
            if let Some(c) = company { orm::write(e, "res.company", &[c], row(&[("sale_discount_product_id", p.into())]))?; }
            p
        }
    };
    let lines = ordered_lines(e, order)?;
    let discount_percentage = if dtype == "amount" {
        let mut so_amount = num(&o, "amount_total");
        if so_amount == 0.0 { return Ok(()); }
        for l in &lines { for t in ids(l, "tax_id") { let tr = rec(e, "account.tax", t)?; if text(&tr, "amount_type").as_deref() == Some("fixed") { so_amount -= num(&tr, "amount") * num(l, "product_uom_qty"); } } }
        num(w, "discount_amount") / so_amount
    } else { num(w, "discount_percentage") };
    let mut groups: Vec<(Vec<i64>, f64)> = vec![];
    for l in &lines {
        if num(l, "product_uom_qty") == 0.0 || num(l, "price_unit") == 0.0 { continue; }
        let mut tx: Vec<i64> = vec![];
        for t in ids(l, "tax_id") { if text(&rec(e, "account.tax", t)?, "amount_type").as_deref() != Some("fixed") { tx.push(t); } }
        tx.sort();
        let amt = num(l, "price_unit") * (1.0 - num(l, "discount") / 100.0) * num(l, "product_uom_qty");
        match groups.iter_mut().find(|g| g.0 == tx) { Some(g) => g.1 += amt, None => groups.push((tx, amt)) }
    }
    if groups.is_empty() { return Ok(()); }
    let repr = |v: f64| format!("{:.2}", v);
    let n = groups.len();
    for (taxes, subtotal) in groups {
        let names: Vec<String> = taxes.iter().filter_map(|t| rec(e, "account.tax", *t).ok().and_then(|r| text(&r, "name"))).collect();
        let desc = if n == 1 { format!("Discount {}%", repr(discount_percentage * 100.0)) }
            else if dtype != "amount" { format!("Discount {}%- On products with the following taxes {}", repr(discount_percentage * 100.0), names.join(", ")) }
            else { format!("Discount- On products with the following taxes {}", names.join(", ")) };
        let tax_ids = taxes;
        orm::create(e, "sale.order.line", row(&[("order_id", order.into()), ("product_id", product.into()), ("sequence", 999.into()), ("price_unit", (-(subtotal * discount_percentage)).into()), ("technical_price_unit", 0.0.into()), ("tax_id", set6(&tax_ids)), ("name", desc.into())]))?;
    }
    Ok(())
}
