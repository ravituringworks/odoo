//! stock: pickings, moves, quants. Validation moves quantities between locations.
use crate::util::*;
use odoo_core::orm::{self, Env};
use odoo_core::{OdooError, Result, Row, Rules, Value};

pub fn ensure_location(env: &Env, usage: &str, name: &str) -> Result<i64> {
    find_or_create(&env.sudo(), "stock.location", term("usage", "=", usage), row(&[("name", name.into()), ("usage", usage.into())]))
}
pub fn ensure_picking_type(env: &Env, code: &str) -> Result<i64> {
    let (name, prefix, src, dst) = match code { "incoming" => ("Receipts", "IN", "supplier", "internal"), "outgoing" => ("Delivery Orders", "OUT", "internal", "customer"), _ => ("Internal Transfers", "INT", "internal", "internal") };
    let e = env.sudo();
    let s = ensure_location(&e, src, &format!("{src} location"))?; let d = ensure_location(&e, dst, &format!("{dst} location"))?;
    find_or_create(&e, "stock.picking.type", term("code", "=", code), row(&[("name", name.into()), ("code", code.into()), ("sequence_code", prefix.into()), ("default_location_src_id", s.into()), ("default_location_dest_id", d.into())]))
}

fn usage(env: &Env, loc: i64) -> Result<String> { Ok(text(&rec(env, "stock.location", loc)?, "usage").unwrap_or_default()) }

/// Adjust on-hand quantity of `product` at `loc` (internal locations only are tracked, like Odoo).
pub fn adjust_quant(env: &Env, product: i64, loc: i64, delta: f64) -> Result<()> {
    if usage(env, loc)? != "internal" { return Ok(()); }
    let e = env.sudo();
    let dom = term("product_id", "=", product).and(term("location_id", "=", loc));
    match find_one(&e, "stock.quant", dom)? {
        Some(q) => { let cur = num(&rec(&e, "stock.quant", q)?, "quantity"); orm::write(&e, "stock.quant", &[q], row(&[("quantity", (cur + delta).into())])) }
        None => orm::create(&e, "stock.quant", row(&[("product_id", product.into()), ("location_id", loc.into()), ("quantity", delta.into())])).map(|_| ()),
    }
}

pub fn on_hand(env: &Env, product: i64, loc: i64) -> Result<f64> {
    Ok(match find_one(&env.sudo(), "stock.quant", term("product_id", "=", product).and(term("location_id", "=", loc)))? { Some(q) => num(&rec(env, "stock.quant", q)?, "quantity"), None => 0.0 })
}

pub fn rules() -> Rules {
    Rules::default()
        // `tracking` is a stored compute in Odoo that settles on "none" unless someone chose lots/serials; never fall back to the first selection
        .before_create("product.template", |_, mut v| { v.entry("tracking".into()).or_insert("none".into()); Ok(v) })
        .compute("product.template", "tracking", |_, r| Ok(match text(r, "tracking").as_deref() { Some(t @ ("lot" | "serial")) => t.into(), _ => "none".into() }))
        .before_create("stock.picking", |env, mut v| {
            let pt = match v.get("picking_type_id").and_then(|p| p.as_i64()) { Some(p) => p, None => { let p = ensure_picking_type(env, "internal")?; v.insert("picking_type_id".into(), p.into()); p } };
            let t = rec(env, "stock.picking.type", pt)?;
            if text(&v, "name").map_or(true, |n| n == "/" || n.is_empty()) { v.insert("name".into(), next_seq(env, &format!("stock.picking.{pt}"), &format!("WH/{}/", text(&t, "sequence_code").unwrap_or("INT".into())), 5)?.into()); }
            for (k, src) in [("location_id", "default_location_src_id"), ("location_dest_id", "default_location_dest_id")] { if !v.contains_key(k) { if let Some(l) = id_of(&t, src) { v.insert(k.into(), l.into()); } } }
            v.entry("state".into()).or_insert("draft".into());
            Ok(v)
        })
        .before_create("stock.move", |env, mut v| {
            if let Some(p) = v.get("picking_id").and_then(|p| p.as_i64()) {
                let pk = rec(env, "stock.picking", p)?;
                for k in ["location_id", "location_dest_id"] { if !v.contains_key(k) { if let Some(l) = id_of(&pk, k) { v.insert(k.into(), l.into()); } } }
                v.entry("state".into()).or_insert(text(&pk, "state").unwrap_or("draft".into()).into());
            }
            v.entry("state".into()).or_insert("draft".into());
            if !v.contains_key("name") { v.insert("name".into(), "Move".into()); }
            if !v.contains_key("product_uom") { if let Some(u) = find_one(env, "uom.uom", odoo_core::Domain::True)? { v.insert("product_uom".into(), u.into()); } }
            Ok(v)
        })
        .action("stock.picking", "action_confirm", |env, ids, _| { set_state(env, ids, "draft", "confirmed") })
        .action("stock.picking", "action_assign", |env, ids, _| { set_state(env, ids, "confirmed", "assigned") })
        .action("stock.picking", "button_validate", |env, ids, _| { for id in ids { validate(env, *id)?; } Ok(Value::Bool(true)) })
        .action("stock.picking", "action_cancel", |env, ids, _| { let e = env.sudo(); for id in ids { for m in children(&e, "stock.move", "picking_id", *id)? { orm::write(&e, "stock.move", &[m["id"].as_i64().unwrap()], row(&[("state", "cancel".into())]))?; } } orm::write(&e, "stock.picking", ids, row(&[("state", "cancel".into())]))?; Ok(Value::Bool(true)) })
}

fn set_state(env: &Env, ids: &[i64], from: &str, to: &str) -> Result<Value> {
    let e = env.sudo();
    for id in ids {
        let p = rec(&e, "stock.picking", *id)?;
        if text(&p, "state").as_deref() == Some(from) || (to == "assigned" && text(&p, "state").as_deref() == Some("draft")) {
            for m in children(&e, "stock.move", "picking_id", *id)? { orm::write(&e, "stock.move", &[m["id"].as_i64().unwrap()], row(&[("state", to.into())]))?; }
            orm::write(&e, "stock.picking", &[*id], row(&[("state", to.into())]))?;
        }
    }
    Ok(Value::Bool(true))
}

pub fn validate(env: &Env, id: i64) -> Result<()> {
    let e = env.sudo();
    let p = rec(&e, "stock.picking", id)?;
    if matches!(text(&p, "state").as_deref(), Some("done" | "cancel")) { return Err(OdooError::User("picking already processed".into())); }
    let moves = children(&e, "stock.move", "picking_id", id)?;
    if moves.is_empty() { return Err(OdooError::User("Please add some items to move.".into())); }
    for m in moves {
        let mid = m["id"].as_i64().unwrap();
        let qty = if num(&m, "quantity") > 0.0 { num(&m, "quantity") } else { num(&m, "product_uom_qty") };
        let (src, dst, prod) = (id_of(&m, "location_id"), id_of(&m, "location_dest_id"), id_of(&m, "product_id"));
        if let (Some(s), Some(d), Some(pr)) = (src, dst, prod) {
            adjust_quant(&e, pr, s, -qty)?; adjust_quant(&e, pr, d, qty)?;
            // propagate delivered/received quantity to originating order lines when the linked modules are installed
            if let (Some(l), true) = (id_of(&m, "sale_line_id"), env.reg.field("sale.order.line", "qty_delivered").is_ok()) { let cur = num(&rec(&e, "sale.order.line", l)?, "qty_delivered"); orm::write(&e, "sale.order.line", &[l], row(&[("qty_delivered", (cur + qty).into())]))?; }
            if let (Some(l), true) = (id_of(&m, "purchase_line_id"), env.reg.field("purchase.order.line", "qty_received").is_ok()) { let cur = num(&rec(&e, "purchase.order.line", l)?, "qty_received"); orm::write(&e, "purchase.order.line", &[l], row(&[("qty_received", (cur + qty).into())]))?; }
        }
        orm::write(&e, "stock.move", &[mid], row(&[("state", "done".into()), ("quantity", qty.into())]))?;
    }
    orm::write(&e, "stock.picking", &[id], row(&[("state", "done".into()), ("date_done", orm::now().into())]))
}

/// sale_stock: one outgoing picking per confirmed order for storable/consumable goods lines.
pub fn deliver_sale_order(env: &Env, order: i64) -> Result<Option<i64>> {
    let o = rec(env, "sale.order", order)?;
    let mut mvs: Vec<Row> = vec![];
    for l in children(env, "sale.order.line", "order_id", order)? {
        let Some(pid) = id_of(&l, "product_id") else { continue };
        let tmpl = rec(env, "product.product", pid).ok().and_then(|p| id_of(&p, "product_tmpl_id")).map(|t| rec(env, "product.template", t)).transpose()?;
        if tmpl.as_ref().and_then(|t| text(t, "type")).as_deref() == Some("service") { continue; }
        let mut m = row(&[("product_id", pid.into()), ("product_uom_qty", num(&l, "product_uom_qty").into()), ("name", text(&l, "name").unwrap_or_default().into()), ("sale_line_id", l["id"].clone())]);
        m.insert("state".into(), "confirmed".into()); mvs.push(m);
    }
    if mvs.is_empty() { return Ok(None); }
    let pt = ensure_picking_type(env, "outgoing")?;
    let mut pv = row(&[("picking_type_id", pt.into()), ("origin", text(&o, "name").unwrap_or_default().into()), ("state", "confirmed".into())]);
    if let Some(p) = id_of(&o, "partner_id") { pv.insert("partner_id".into(), p.into()); }
    let pid = orm::create(env, "stock.picking", pv)?;
    for mut m in mvs { m.insert("picking_id".into(), pid.into()); orm::create(env, "stock.move", m)?; }
    Ok(Some(pid))
}
