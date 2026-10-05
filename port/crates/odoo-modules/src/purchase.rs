//! purchase: RFQ → PO → receipt (stock) → vendor bill (account).
use crate::{stock, tax, util::*};
use odoo_core::orm::{self, Env};
use odoo_core::{OdooError, Result, Row, Rules, Value};

fn amounts(env: &Env, l: &Row) -> Result<(f64, f64, f64)> { Ok(tax::compute(num(l, "product_qty"), num(l, "price_unit"), 0.0, &tax::load(env, &ids(l, "taxes_id"))?)) }
fn sum(env: &Env, order: i64) -> Result<(f64, f64)> {
    let (mut u, mut t) = (0.0, 0.0);
    for l in children(env, "purchase.order.line", "order_id", order)?.iter().filter(|l| text(l, "display_type").map_or(true, |d| d.is_empty())) { let (a, b, _) = amounts(env, l)?; u += a; t += b; }
    Ok((r2(u), r2(t)))
}

pub fn rules() -> Rules {
    Rules::default()
        .before_create("purchase.order", |env, mut v| { if text(&v, "name").map_or(true, |n| n == "New" || n.is_empty()) { v.insert("name".into(), next_seq(env, "purchase.order", "P", 5)?.into()); } v.entry("state".into()).or_insert("draft".into()); v.entry("date_order".into()).or_insert(orm::now().into());
            if !v.contains_key("picking_type_id") && env.reg.model("stock.picking.type").is_ok() { v.insert("picking_type_id".into(), stock::ensure_picking_type(env, "incoming")?.into()); }
            Ok(v) })
        .before_create("purchase.order.line", |env, mut v| {
            if let Some(pid) = v.get("product_id").and_then(|p| p.as_i64()) {
                let e = env.sudo(); let p = rec(&e, "product.product", pid)?;
                if let Some(t) = id_of(&p, "product_tmpl_id") { let t = rec(&e, "product.template", t)?; v.entry("price_unit".into()).or_insert(num(&t, "standard_price").into()); v.entry("name".into()).or_insert(text(&t, "name").unwrap_or_default().into()); }
            }
            v.entry("product_qty".into()).or_insert(1.0.into()); v.entry("name".into()).or_insert("".into()); Ok(v)
        })
        .compute("purchase.order.line", "price_subtotal", |env, r| Ok(amounts(env, r)?.0.into()))
        .compute("purchase.order.line", "price_tax", |env, r| Ok(amounts(env, r)?.1.into()))
        .compute("purchase.order.line", "price_total", |env, r| Ok(amounts(env, r)?.2.into()))
        .compute("purchase.order", "amount_untaxed", |env, r| Ok(sum(env, r["id"].as_i64().unwrap())?.0.into()))
        .compute("purchase.order", "amount_tax", |env, r| Ok(sum(env, r["id"].as_i64().unwrap())?.1.into()))
        .compute("purchase.order", "amount_total", |env, r| { let (u, t) = sum(env, r["id"].as_i64().unwrap())?; Ok(r2(u + t).into()) })
        .compute("purchase.order", "invoice_status", |env, r| {
            if !matches!(text(r, "state").as_deref(), Some("purchase" | "done")) { return Ok("no".into()); }
            let ls = children(env, "purchase.order.line", "order_id", r["id"].as_i64().unwrap())?;
            Ok(if ls.iter().any(|l| num(l, "qty_invoiced") < num(l, "product_qty")) { "to invoice" } else { "invoiced" }.into())
        })
        .after_create("purchase.order.line", |env, _, v| { if let Some(o) = v.get("order_id").and_then(|v| v.as_i64()) { orm::recompute_ids(env, "purchase.order", &[o])?; } Ok(()) })
        .after_write("purchase.order.line", |env, ids, _| { let os: Vec<i64> = ids.iter().filter_map(|i| rec(env, "purchase.order.line", *i).ok().and_then(|r| id_of(&r, "order_id"))).collect(); orm::recompute_ids(env, "purchase.order", &os) })
        .on_unlink("purchase.order.line", |env, ids| Ok(vec![("purchase.order".into(), ids.iter().filter_map(|i| rec(env, "purchase.order.line", *i).ok().and_then(|r| id_of(&r, "order_id"))).collect())]))
        .action("purchase.order", "button_confirm", |env, ids, _| {
            let e = env.sudo();
            for id in ids {
                let o = rec(&e, "purchase.order", *id)?;
                if !matches!(text(&o, "state").as_deref(), Some("draft" | "sent")) { return Err(OdooError::User("Only RFQs can be confirmed".into())); }
                orm::write(&e, "purchase.order", &[*id], row(&[("state", "purchase".into()), ("date_approve", orm::now().into())])).or_else(|_| orm::write(&e, "purchase.order", &[*id], row(&[("state", "purchase".into())])))?;
                orm::recompute_ids(&e, "purchase.order", &[*id])?;
                if env.reg.model("stock.picking").is_ok() && env.reg.field("stock.move", "purchase_line_id").is_ok() { receive(&e, *id)?; }
            }
            Ok(Value::Bool(true))
        })
        .action("purchase.order", "button_cancel", |env, ids, _| { orm::write(&env.sudo(), "purchase.order", ids, row(&[("state", "cancel".into())]))?; Ok(Value::Bool(true)) })
        .action("purchase.order", "action_create_invoice", |env, ids, _| { let mut out = vec![]; for id in ids { out.push(Value::Int(create_bill(env, *id)?)); } Ok(Value::List(out)) })
}

fn receive(env: &Env, order: i64) -> Result<()> {
    let o = rec(env, "purchase.order", order)?;
    let pt = stock::ensure_picking_type(env, "incoming")?;
    let mut pv = row(&[("picking_type_id", pt.into()), ("origin", text(&o, "name").unwrap_or_default().into()), ("state", "assigned".into())]);
    if let Some(p) = id_of(&o, "partner_id") { pv.insert("partner_id".into(), p.into()); }
    let mut moves = vec![];
    for l in children(env, "purchase.order.line", "order_id", order)? {
        let Some(pid) = id_of(&l, "product_id") else { continue };
        let tmpl = rec(env, "product.product", pid).ok().and_then(|p| id_of(&p, "product_tmpl_id")).map(|t| rec(env, "product.template", t)).transpose()?;
        if tmpl.as_ref().and_then(|t| text(t, "type")).as_deref() == Some("service") { continue; }
        moves.push(row(&[("product_id", pid.into()), ("product_uom_qty", num(&l, "product_qty").into()), ("name", text(&l, "name").unwrap_or_default().into()), ("purchase_line_id", l["id"].clone()), ("state", "assigned".into())]));
    }
    if moves.is_empty() { return Ok(()); }
    let pk = orm::create(env, "stock.picking", pv)?;
    for mut m in moves { m.insert("picking_id".into(), pk.into()); orm::create(env, "stock.move", m)?; }
    Ok(())
}

pub fn create_bill(env: &Env, order: i64) -> Result<i64> {
    let e = env.sudo();
    let o = rec(&e, "purchase.order", order)?;
    let lines: Vec<Row> = children(&e, "purchase.order.line", "order_id", order)?.into_iter().filter(|l| num(l, "product_qty") > num(l, "qty_invoiced")).collect();
    if lines.is_empty() { return Err(OdooError::User("There is nothing to bill!".into())); }
    let mid = orm::create(&e, "account.move", row(&[("move_type", "in_invoice".into()), ("partner_id", id_of(&o, "partner_id").map(Value::Int).unwrap_or(Value::Null)), ("invoice_origin", text(&o, "name").unwrap_or_default().into())]))?;
    for l in &lines {
        let q = num(l, "product_qty") - num(l, "qty_invoiced");
        let mut v = row(&[("move_id", mid.into()), ("name", text(l, "name").unwrap_or_default().into()), ("quantity", q.into()), ("price_unit", num(l, "price_unit").into()), ("display_type", "product".into())]);
        if let Some(p) = id_of(l, "product_id") { v.insert("product_id".into(), p.into()); }
        let tx = ids(l, "taxes_id"); if !tx.is_empty() { v.insert("tax_ids".into(), Value::List(vec![Value::List(vec![6.into(), 0.into(), Value::List(tx.into_iter().map(Value::Int).collect())])])); }
        orm::create(&e, "account.move.line", v)?;
        orm::write(&e, "purchase.order.line", &[l["id"].as_i64().unwrap()], row(&[("qty_invoiced", num(l, "product_qty").into())]))?;
    }
    orm::recompute_ids(&e, "account.move", &[mid])?;
    orm::recompute_ids(&e, "purchase.order", &[order])?;
    Ok(mid)
}
