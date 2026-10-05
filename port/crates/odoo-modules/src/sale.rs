//! sale: quotation → confirmed order → invoicing (+ optional delivery via stock).
use crate::{account, stock, tax, util::*};
use odoo_core::orm::{self, Env};
use odoo_core::{OdooError, Result, Row, Rules, Value};

fn line_amounts(env: &Env, l: &Row) -> Result<(f64, f64, f64)> {
    Ok(tax::compute(num(l, "product_uom_qty"), num(l, "price_unit"), num(l, "discount"), &tax::load(env, &ids(l, "tax_id"))?))
}
fn is_real(l: &Row) -> bool { text(l, "display_type").map_or(true, |d| d.is_empty()) }

fn order_sum(env: &Env, order: i64) -> Result<(f64, f64)> {
    let mut u = 0.0; let mut t = 0.0;
    for l in children(env, "sale.order.line", "order_id", order)?.iter().filter(|l| is_real(l)) { let (a, b, _) = line_amounts(env, l)?; u += a; t += b; }
    Ok((r2(u), r2(t)))
}

pub fn rules() -> Rules {
    Rules::default()
        .before_create("sale.order", |env, mut v| {
            if text(&v, "name").map_or(true, |n| n == "New" || n.is_empty()) { v.insert("name".into(), next_seq(env, "sale.order", "S", 5)?.into()); }
            v.entry("state".into()).or_insert("draft".into());
            v.entry("date_order".into()).or_insert(orm::now().into());
            Ok(v)
        })
        .before_create("sale.order.line", |env, mut v| {
            if let Some(pid) = v.get("product_id").and_then(|p| p.as_i64()) {
                let e = env.sudo();
                let p = rec(&e, "product.product", pid)?;
                let tmpl = id_of(&p, "product_tmpl_id").map(|t| rec(&e, "product.template", t)).transpose()?;
                if let Some(t) = &tmpl {
                    v.entry("price_unit".into()).or_insert(num(t, "list_price").into());
                    v.entry("name".into()).or_insert(text(t, "name").unwrap_or_default().into());
                    if !v.contains_key("tax_id") { let tx = ids(t, "taxes_id"); if !tx.is_empty() { v.insert("tax_id".into(), Value::List(vec![Value::List(vec![6.into(), 0.into(), Value::List(tx.into_iter().map(Value::Int).collect())])])); } }
                }
            }
            v.entry("product_uom_qty".into()).or_insert(1.0.into());
            v.entry("name".into()).or_insert("".into());
            Ok(v)
        })
        .onchange("sale.order.line", |env, mut v| {
            if let Some(pid) = v.get("product_id").and_then(|p| p.as_i64()) {
                let e = env.sudo(); let p = rec(&e, "product.product", pid)?;
                if let Some(t) = id_of(&p, "product_tmpl_id") { let t = rec(&e, "product.template", t)?; v.insert("price_unit".into(), num(&t, "list_price").into()); v.insert("name".into(), text(&t, "name").unwrap_or_default().into()); }
            }
            let (u, tx, tot) = line_amounts(env, &v)?; v.insert("price_subtotal".into(), u.into()); v.insert("price_tax".into(), tx.into()); v.insert("price_total".into(), tot.into());
            Ok(v)
        })
        .compute("sale.order.line", "price_subtotal", |env, r| Ok(line_amounts(env, r)?.0.into()))
        .compute("sale.order.line", "price_tax", |env, r| Ok(line_amounts(env, r)?.1.into()))
        .compute("sale.order.line", "price_total", |env, r| Ok(line_amounts(env, r)?.2.into()))
        .compute("sale.order.line", "qty_to_invoice", |_, r| Ok(if text(r, "state").as_deref() == Some("sale") { (num(r, "product_uom_qty") - num(r, "qty_invoiced")).max(0.0).into() } else { 0.0.into() }))
        .compute("sale.order", "amount_untaxed", |env, r| Ok(order_sum(env, r["id"].as_i64().unwrap())?.0.into()))
        .compute("sale.order", "amount_tax", |env, r| Ok(order_sum(env, r["id"].as_i64().unwrap())?.1.into()))
        .compute("sale.order", "amount_total", |env, r| { let (u, t) = order_sum(env, r["id"].as_i64().unwrap())?; Ok(r2(u + t).into()) })
        .compute("sale.order", "invoice_status", |env, r| {
            if text(r, "state").as_deref() != Some("sale") { return Ok("no".into()); }
            let ls: Vec<Row> = children(env, "sale.order.line", "order_id", r["id"].as_i64().unwrap())?.into_iter().filter(is_real).collect();
            Ok(if ls.iter().any(|l| num(l, "qty_to_invoice") > 0.0) { "to invoice" } else if !ls.is_empty() && ls.iter().all(|l| num(l, "qty_invoiced") >= num(l, "product_uom_qty")) { "invoiced" } else { "no" }.into())
        })
        .after_create("sale.order.line", |env, _, v| { if let Some(o) = v.get("order_id").and_then(|v| v.as_i64()) { orm::recompute_ids(env, "sale.order", &[o])?; } Ok(()) })
        .after_write("sale.order.line", |env, ids, _| { let os: Vec<i64> = ids.iter().filter_map(|i| rec(env, "sale.order.line", *i).ok().and_then(|r| id_of(&r, "order_id"))).collect(); orm::recompute_ids(env, "sale.order", &os) })
        .on_unlink("sale.order.line", |env, ids| Ok(vec![("sale.order".into(), ids.iter().filter_map(|i| rec(env, "sale.order.line", *i).ok().and_then(|r| id_of(&r, "order_id"))).collect())]))
        .action("sale.order", "action_confirm", |env, ids, _| {
            let e = env.sudo();
            for id in ids {
                let o = rec(&e, "sale.order", *id)?;
                if !matches!(text(&o, "state").as_deref(), Some("draft" | "sent")) { return Err(OdooError::User(format!("order {} cannot be confirmed from state {:?}", text(&o, "name").unwrap_or_default(), text(&o, "state")))); }
                if children(&e, "sale.order.line", "order_id", *id)?.iter().filter(|l| is_real(l)).count() == 0 { return Err(OdooError::User("You need to add a line before confirming".into())); }
                orm::write(&e, "sale.order", &[*id], row(&[("state", "sale".into()), ("date_order", orm::now().into())]))?;
                let lines: Vec<i64> = children(&e, "sale.order.line", "order_id", *id)?.iter().map(|l| l["id"].as_i64().unwrap()).collect();
                orm::write(&e, "sale.order.line", &lines, row(&[("state", "sale".into())]))?;
                orm::recompute_ids(&e, "sale.order", &[*id])?;
                if env.reg.model("stock.picking").is_ok() && env.reg.field("stock.move", "sale_line_id").is_ok() { stock::deliver_sale_order(&e, *id)?; }
            }
            Ok(Value::Bool(true))
        })
        .action("sale.order", "action_cancel", |env, ids, _| { let e = env.sudo(); orm::write(&e, "sale.order", ids, row(&[("state", "cancel".into())]))?; Ok(Value::Bool(true)) })
        .action("sale.order", "action_draft", |env, ids, _| { orm::write(&env.sudo(), "sale.order", ids, row(&[("state", "draft".into())]))?; Ok(Value::Bool(true)) })
        .action("sale.order", "_create_invoices", |env, ids, _| { let mut out = vec![]; for id in ids { out.push(Value::Int(create_invoice(env, *id)?)); } Ok(Value::List(out)) })
}

/// Build a draft customer invoice from the order's invoiceable quantities; link lines and update qty_invoiced.
pub fn create_invoice(env: &Env, order: i64) -> Result<i64> {
    let e = env.sudo();
    let o = rec(&e, "sale.order", order)?;
    if text(&o, "state").as_deref() != Some("sale") { return Err(OdooError::User("Only confirmed orders can be invoiced".into())); }
    let lines: Vec<Row> = children(&e, "sale.order.line", "order_id", order)?.into_iter().filter(|l| is_real(l) && num(l, "qty_to_invoice") > 0.0).collect();
    if lines.is_empty() { return Err(OdooError::User("There is nothing to invoice!".into())); }
    let mid = orm::create(&e, "account.move", row(&[("move_type", "out_invoice".into()), ("partner_id", id_of(&o, "partner_id").map(Value::Int).unwrap_or(Value::Null)), ("invoice_origin", text(&o, "name").unwrap_or_default().into())]))?;
    for l in &lines {
        let q = num(l, "qty_to_invoice");
        let mut v = row(&[("move_id", mid.into()), ("name", text(l, "name").unwrap_or_default().into()), ("quantity", q.into()), ("price_unit", num(l, "price_unit").into()), ("discount", num(l, "discount").into()), ("display_type", "product".into())]);
        if let Some(p) = id_of(l, "product_id") { v.insert("product_id".into(), p.into()); }
        let tx = ids(l, "tax_id"); if !tx.is_empty() { v.insert("tax_ids".into(), Value::List(vec![Value::List(vec![6.into(), 0.into(), Value::List(tx.into_iter().map(Value::Int).collect())])])); }
        if env.reg.field("account.move.line", "sale_line_ids").is_ok() { v.insert("sale_line_ids".into(), Value::List(vec![Value::List(vec![4.into(), l["id"].clone()])])); }
        let lid = orm::create(&e, "account.move.line", v)?;
        let linked = [("invoice_lines".to_string(), Value::List(vec![Value::List(vec![4.into(), lid.into()])]))].into_iter().collect::<Row>();
        orm::write(&e, "sale.order.line", &[l["id"].as_i64().unwrap()], { let mut w = linked; w.insert("qty_invoiced".into(), (num(l, "qty_invoiced") + q).into()); w })?;
    }
    orm::recompute_ids(&e, "account.move", &[mid])?;
    orm::recompute_ids(&e, "sale.order", &[order])?;
    let _ = account::ensure_journal;
    Ok(mid)
}
