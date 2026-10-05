//! pos_sale: sale orders settled at the till (`addons/pos_sale/models`). Rules are keyed by model and only act when the
//! pos_sale fields exist in the registry.
//!
//! Not ported: re-quantifying the waiting deliveries of a sale order after a POS down payment/settlement (needs stock reservation),
//! qty_delivered/qty_invoiced contributions of POS lines on sale lines, and online-payment confirmation.
use crate::pos_methods::{flag, has_model, id_list};
use crate::util::*;
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, OdooError, Result, Row, Rules, Value};
use std::collections::BTreeSet;

fn has_field(env: &Env, m: &str, f: &str) -> bool { env.reg.field(m, f).is_ok() }
fn first(ids_: &[i64]) -> Result<i64> { ids_.first().copied().ok_or_else(|| OdooError::User("Select a record".into())) }
fn act(name: &str, model: &str, domain: Vec<i64>) -> Value {
    Value::Map(row(&[("type", "ir.actions.act_window".into()), ("name", name.into()), ("res_model", model.into()), ("view_mode", "list,form".into()), ("domain", Value::List(vec![Value::List(vec!["id".into(), "in".into(), id_list(&domain)])]))]))
}
/// Distinct sale orders the lines of a POS order come from.
pub(crate) fn origin_orders(env: &Env, order: &Row) -> Result<Vec<i64>> {
    if !has_field(env, "pos.order.line", "sale_order_origin_id") { return Ok(vec![]); }
    let mut out = vec![];
    for l in children(env, "pos.order.line", "order_id", order["id"].as_i64().unwrap())? { if let Some(s) = id_of(&l, "sale_order_origin_id") { if !out.contains(&s) { out.push(s); } } }
    Ok(out)
}
fn pos_orders_of_sale(env: &Env, so: i64) -> Result<Vec<i64>> {
    if !has_field(env, "pos.order.line", "sale_order_origin_id") { return Ok(vec![]); }
    let mut out = vec![];
    for l in children(env, "pos.order.line", "sale_order_origin_id", so)? { if let Some(o) = id_of(&l, "order_id") { if !out.contains(&o) { out.push(o); } } }
    Ok(out)
}

/// `amount_unpaid`: what is left of the order after invoices and POS payments.
fn amount_unpaid(env: &Env, so: &Row) -> Result<f64> {
    let e = env.sudo(); let mut invoiced = 0.0; let mut pos_paid = 0.0;
    for l in children(&e, "sale.order.line", "order_id", so["id"].as_i64().unwrap())? {
        if flag(&l, "display_type") { continue; }
        for il in ids(&l, "invoice_lines") {
            let Ok(ml) = rec(&e, "account.move.line", il) else { continue };
            let Some(mv) = id_of(&ml, "move_id").and_then(|m| rec(&e, "account.move", m).ok()) else { continue };
            if text(&mv, "state").as_deref() == Some("cancel") { continue; }
            // credit notes give money back: the sign follows the document type
            let sign = if text(&mv, "move_type").as_deref() == Some("out_refund") { -1.0 } else { 1.0 };
            invoiced += sign * num(&ml, "price_total").abs();
        }
        if has_field(&e, "pos.order.line", "sale_order_line_id") { for pl in children(&e, "pos.order.line", "sale_order_line_id", l["id"].as_i64().unwrap())? { pos_paid += num(&pl, "price_subtotal_incl"); } }
    }
    Ok(clean((num(so, "amount_total") - invoiced - pos_paid - num(so, "amount_paid")).max(0.0)))
}
fn clean(x: f64) -> f64 { (x * 1e6).round() / 1e6 }

/// `pos.order.line.qty_delivered`: quantity actually shipped for lines of settled orders.
fn line_qty_delivered(env: &Env, l: &Row) -> Result<f64> {
    let Some(order) = id_of(l, "order_id").and_then(|o| rec(env, "pos.order", o).ok()) else { return Ok(0.0) };
    if !matches!(text(&order, "state").as_deref(), Some("paid" | "done" | "invoiced")) { return Ok(num(l, "qty_delivered")); }
    let mut moves: Vec<Row> = vec![]; let mut any_outgoing = false;
    for p in crate::pos_methods::many(env, "stock.picking", &ids(&order, "picking_ids"))? {
        let code = id_of(&p, "picking_type_id").and_then(|t| rec(env, "stock.picking.type", t).ok()).and_then(|t| text(&t, "code"));
        if text(&p, "state").as_deref() == Some("done") && code.as_deref() == Some("outgoing") { any_outgoing = true; moves.extend(children(env, "stock.move", "picking_id", p["id"].as_i64().unwrap())?); }
    }
    if !any_outgoing { return Ok(0.0); }
    if flag(&order, "shipping_date") {
        let shipped: f64 = moves.iter().filter(|m| text(m, "state").as_deref() == Some("done") && id_of(m, "product_id") == id_of(l, "product_id")).map(|m| num(m, "quantity")).sum();
        return Ok(num(l, "qty").min(shipped));
    }
    Ok(num(l, "qty"))   // not delivered later and settled: fully delivered
}

pub fn rules() -> Rules {
    Rules::default()
        .compute("crm.team", "pos_sessions_open_count", |env, r| {
            let e = env.sudo(); let mut n = 0;
            for cfg in children(&e, "pos.config", "crm_team_id", r["id"].as_i64().unwrap())? { n += orm::search_count(&e, "pos.session", &Domain::And(vec![term("config_id", "=", cfg["id"].as_i64().unwrap()), term("state", "=", "opened")]))?; }
            Ok(n.into())
        })
        .compute("crm.team", "pos_order_amount_total", |env, r| {
            // the report's price_total: line totals in company currency, for orders of the team's open sessions
            let e = env.sudo(); let mut total = 0.0;
            for cfg in children(&e, "pos.config", "crm_team_id", r["id"].as_i64().unwrap())? {
                for s in orm::search(&e, "pos.session", &Domain::And(vec![term("config_id", "=", cfg["id"].as_i64().unwrap()), term("state", "=", "opened")]), None, None, 0)? {
                    for o in children(&e, "pos.order", "session_id", s)? {
                        let rate = crate::pos::rate_of(&o);
                        for l in children(&e, "pos.order.line", "order_id", o["id"].as_i64().unwrap())? { total += num(&l, "price_subtotal_incl") / rate; }
                    }
                }
            }
            Ok(((total * 100.0).round() / 100.0).into())
        })
        .compute("pos.order", "sale_order_count", |env, r| Ok((origin_orders(&env.sudo(), r)?.len() as i64).into()))
        .compute("sale.order", "pos_order_count", |env, r| Ok((pos_orders_of_sale(&env.sudo(), r["id"].as_i64().unwrap())?.len() as i64).into()))
        .compute("sale.order", "amount_unpaid", |env, r| Ok(amount_unpaid(env, r)?.into()))
        .compute("pos.order.line", "qty_delivered", |env, r| Ok(line_qty_delivered(&env.sudo(), r)?.into()))
        // POS lines changing refresh the stored amounts that depend on them
        .after_create("pos.order.line", |env, ids_, _| refresh_linked(env, ids_))
        .after_write("pos.order.line", |env, ids_, _| refresh_linked(env, ids_))
        .after_write("pos.order", |env, ids_, vals| {
            let e = env.sudo();
            if has_field(&e, "pos.order", "crm_team_id") && vals.contains_key("crm_team_id") && !flag(vals, "crm_team_id") {
                // an emptied team falls back to the session's
                for i in ids_ {
                    let o = rec(&e, "pos.order", *i)?;
                    let team = id_of(&o, "session_id").and_then(|s| rec(&e, "pos.session", s).ok()).and_then(|s| id_of(&s, "config_id")).and_then(|c| rec(&e, "pos.config", c).ok()).and_then(|c| id_of(&c, "crm_team_id"));
                    if let Some(t) = team { orm::write(&e, "pos.order", &[*i], row(&[("crm_team_id", t.into())]))?; }
                }
            }
            if has_field(&e, "pos.order.line", "qty_delivered") {
                let mut lines = vec![]; for i in ids_ { lines.extend(ids(&rec(&e, "pos.order", *i)?, "lines")); }
                orm::recompute_ids(&e, "pos.order.line", &lines)?;
            }
            Ok(())
        })
        // new orders belong to the register's sales team unless told otherwise
        .before_create("pos.order", |env, mut v| {
            if has_field(env, "pos.order", "crm_team_id") && !v.get("crm_team_id").map_or(false, |t| t.truthy()) {
                let team = v.get("session_id").and_then(|s| s.as_i64()).and_then(|s| rec(env, "pos.session", s).ok()).and_then(|s| id_of(&s, "config_id")).and_then(|c| rec(env, "pos.config", c).ok()).and_then(|c| id_of(&c, "crm_team_id"));
                if let Some(t) = team { v.insert("crm_team_id".into(), t.into()); }
            }
            Ok(v)
        })
        .action("pos.order", "action_view_sale_order", |env, ids_, _| Ok(act("Linked Sale Orders", "sale.order", origin_orders(env, &rec(env, "pos.order", first(ids_)?)?)?)))
        .action("sale.order", "action_view_pos_order", |env, ids_, _| Ok(act("Linked POS Orders", "pos.order", pos_orders_of_sale(env, first(ids_)?)?)))
        .action("product.product", "_optional_product_pos_domain", |_, _, _| Ok(Value::List(vec![
            Value::List(vec!["sale_ok".into(), "=".into(), true.into()]), Value::List(vec!["available_in_pos".into(), "=".into(), true.into()])])))
        .action("product.product", "has_optional_product_in_pos", |env, ids_, _| Ok((!optional_products(env, first(ids_)?)?.is_empty()).into()))
}
fn optional_products(env: &Env, pid: i64) -> Result<Vec<Row>> {
    if !has_field(env, "product.template", "optional_product_ids") { return Ok(vec![]); }
    let t = id_of(&rec(env, "product.product", pid)?, "product_tmpl_id").and_then(|t| rec(env, "product.template", t).ok()).unwrap_or_default();
    Ok(crate::pos_methods::many(env, "product.template", &ids(&t, "optional_product_ids"))?.into_iter().filter(|o| flag(o, "sale_ok") && flag(o, "available_in_pos")).collect())
}
/// `get_product_info_pos` of pos_sale: the optional products still sellable in the POS with their cheapest variant price.
pub fn optional_products_info(env: &Env, pid: i64) -> Result<Value> {
    let mut out = vec![];
    for o in optional_products(env, pid)? {
        let mut price: Option<f64> = None;
        for v in children(env, "product.product", "product_tmpl_id", o["id"].as_i64().unwrap())? { let p = num(&o, "list_price") + num(&v, "price_extra"); price = Some(price.map_or(p, |x| x.min(p))); }
        out.push(Value::Map(row(&[("name", o.get("name").cloned().unwrap_or(Value::Null)), ("price", price.unwrap_or(num(&o, "list_price")).into())])));
    }
    Ok(Value::List(out))
}
fn refresh_linked(env: &Env, line_ids: &[i64]) -> Result<()> {
    if !has_field(env, "sale.order", "amount_unpaid") { return Ok(()); }
    let e = env.sudo(); let mut orders: BTreeSet<i64> = BTreeSet::new();
    for i in line_ids { if let Ok(l) = rec(&e, "pos.order.line", *i) { if let Some(so) = id_of(&l, "sale_order_origin_id") { orders.insert(so); } if let Some(sl) = id_of(&l, "sale_order_line_id").and_then(|x| rec(&e, "sale.order.line", x).ok()).and_then(|x| id_of(&x, "order_id")) { orders.insert(sl); } } }
    if orders.is_empty() { return Ok(()); }
    orm::recompute_ids(&e, "sale.order", &orders.into_iter().collect::<Vec<_>>())
}

/// `sync_from_ui` of pos_sale: down payments become sale lines, and settled orders confirm their sale orders.
pub fn after_sync(env: &Env, order_ids: &[i64]) -> Result<()> {
    if !has_field(env, "pos.order.line", "sale_order_origin_id") || !has_model(env, "sale.order") { return Ok(()); }
    let e = env.sudo();
    for oid in order_ids {
        let order = rec(&e, "pos.order", *oid)?;
        let cfg = id_of(&order, "session_id").and_then(|s| rec(&e, "pos.session", s).ok()).and_then(|s| id_of(&s, "config_id")).and_then(|c| rec(&e, "pos.config", c).ok()).unwrap_or_default();
        let dp = id_of(&cfg, "down_payment_product_id");
        for line in children(&e, "pos.order.line", "order_id", *oid)? {
            let refunded = id_of(&line, "refunded_orderline_id").and_then(|r| rec(&e, "pos.order.line", r).ok());
            let origin = id_of(&line, "sale_order_origin_id").or_else(|| refunded.as_ref().and_then(|r| id_of(r, "sale_order_origin_id")));
            let Some(origin) = origin else { continue };
            if dp.is_none() || id_of(&line, "product_id") != dp || num(&line, "qty") == 0.0 || id_of(&line, "sale_order_line_id").is_some() { continue; }
            let lid = line["id"].as_i64().unwrap();
            if let Some(orig_sl) = refunded.as_ref().and_then(|r| id_of(r, "sale_order_line_id")).and_then(|s| rec(&e, "sale.order.line", s).ok()) {
                // refunding a down payment lowers the price of the sale line it created
                let paid: f64 = children(&e, "pos.order.line", "sale_order_line_id", orig_sl["id"].as_i64().unwrap())?.iter().map(|p| num(p, "price_unit")).sum();
                orm::write(&e, "sale.order.line", &[orig_sl["id"].as_i64().unwrap()], row(&[("price_unit", (paid - num(&line, "price_unit")).into())]))?;
                continue;
            }
            let sale_lines = children(&e, "sale.order.line", "order_id", origin)?;
            if !sale_lines.iter().any(|l| flag(l, "is_downpayment") && text(l, "display_type").as_deref() == Some("line_section")) {
                let seq = sale_lines.last().map_or(10, |l| l.get("sequence").and_then(|v| v.as_i64()).unwrap_or(10) + 1);
                orm::create(&e, "sale.order.line", row(&[("order_id", origin.into()), ("name", "Down Payments".into()), ("display_type", "line_section".into()), ("is_downpayment", true.into()), ("product_uom_qty", 0.0.into()), ("sequence", seq.into())]))?;
            }
            let desc = format!("Down payment (ref: {} on \n {})", text(&line, "name").unwrap_or_default(), text(&order, "date_order").map_or(String::new(), |d| d.chars().take(10).collect()));
            let seq = sale_lines.last().map_or(10, |l| l.get("sequence").and_then(|v| v.as_i64()).unwrap_or(10) + 2);
            let mut v = row(&[("order_id", origin.into()), ("product_id", id_of(&line, "product_id").map_or(Value::Null, Value::Int)), ("price_unit", num(&line, "price_unit").into()), ("product_uom_qty", 0.0.into()), ("is_downpayment", true.into()), ("discount", num(&line, "discount").into()), ("sequence", seq.into()), ("name", desc.into())]);
            let tx = ids(&line, "tax_ids"); if !tx.is_empty() { v.insert("tax_id".into(), crate::pos_methods::set6(&tx)); }
            let sl = orm::create(&e, "sale.order.line", v)?;
            orm::write(&e, "pos.order.line", &[lid], row(&[("sale_order_line_id", sl.into())]))?;
        }
        // a settled order confirms the quotations it pays
        if text(&order, "state").as_deref() != Some("draft") {
            let mut sos: BTreeSet<i64> = BTreeSet::new();
            for l in children(&e, "pos.order.line", "order_id", *oid)? { if let Some(so) = id_of(&l, "sale_order_line_id").and_then(|s| rec(&e, "sale.order.line", s).ok()).and_then(|s| id_of(&s, "order_id")) { sos.insert(so); } }
            for so in sos { if matches!(text(&rec(&e, "sale.order", so)?, "state").as_deref(), Some("draft" | "sent")) { orm::call(&e, "sale.order", "action_confirm", &[so], &Row::new())?; } }
        }
    }
    Ok(())
}
