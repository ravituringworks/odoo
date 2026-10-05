//! POS back-office postings: stock moves per order (inventory) and the session-closing journal entry (accounting).
use crate::{account, stock, util::*};
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, OdooError, Result, Row, Value};
use std::collections::BTreeMap;

/// Deliver (or take back, for refunds) the goods of a paid order.
pub fn move_stock(env: &Env, order: i64, name: &str, partner: Option<i64>, source: Option<i64>) -> Result<Option<i64>> {
    if env.reg.model("stock.picking").is_err() { return Ok(None); }
    let e = env.sudo();
    let mut out: Vec<Row> = vec![]; let mut back: Vec<Row> = vec![];
    for l in children(&e, "pos.order.line", "order_id", order)? {
        if flag(&l, "is_reward_line") { continue; }
        let Some(pid) = id_of(&l, "product_id") else { continue };
        let t = rec(&e, "product.product", pid).ok().and_then(|p| id_of(&p, "product_tmpl_id")).map(|t| rec(&e, "product.template", t)).transpose()?;
        if matches!(t.as_ref().and_then(|t| text(t, "type")).as_deref(), Some("service" | "combo")) { continue; }
        let q = num(&l, "qty"); if q == 0.0 { continue; }
        // lot / serial tracked lines move the quant of the exact lot instead of going through the generic picking
        if e.reg.model("pos.pack.operation.lot").is_ok() && e.reg.model("stock.lot").is_ok() && e.reg.field("stock.quant", "lot_id").is_ok() {
            let names: Vec<String> = children(&e, "pos.pack.operation.lot", "pos_order_line_id", l["id"].as_i64().unwrap())?.iter().filter_map(|r| text(r, "lot_name")).collect();
            if !names.is_empty() {
                let loc = match source { Some(s) => s, None => stock::ensure_location(&e, "internal", "Stock")? };
                let per = q / names.len() as f64;
                for n in names { if let Some(lot) = find_one(&e, "stock.lot", odoo_core::Domain::And(vec![term("product_id", "=", pid), term("name", "=", n.as_str())]))? { adjust_lot_quant(&e, pid, loc, lot, -per)?; } }
                continue;
            }
        }
        let m = row(&[("product_id", pid.into()), ("product_uom_qty", q.abs().into()), ("name", text(&l, "full_product_name").unwrap_or_default().into()), ("state", "confirmed".into())]);
        if q > 0.0 { out.push(m) } else { back.push(m) }
    }
    let mut first = None;
    for (code, mvs) in [("outgoing", out), ("incoming", back)] {
        if mvs.is_empty() { continue; }
        let pt = stock::ensure_picking_type(&e, code)?;
        let internal = match source { Some(l) => l, None => stock::ensure_location(&e, "internal", "Stock")? }; let customer = stock::ensure_location(&e, "customer", "Customers")?;
        let (src, dst) = if code == "outgoing" { (internal, customer) } else { (customer, internal) };
        let mut pv = row(&[("picking_type_id", pt.into()), ("origin", name.into()), ("state", "confirmed".into()), ("location_id", src.into()), ("location_dest_id", dst.into())]);
        if let Some(p) = partner { pv.insert("partner_id".into(), p.into()); }
        let pk = orm::create(&e, "stock.picking", pv)?;
        for mut m in mvs { m.insert("picking_id".into(), pk.into()); m.insert("location_id".into(), src.into()); m.insert("location_dest_id".into(), dst.into()); orm::create(&e, "stock.move", m)?; }
        stock::validate(&e, pk)?; first.get_or_insert(pk);
    }
    Ok(first)
}
fn flag(r: &Row, k: &str) -> bool { r.get(k).map_or(false, |v| v.truthy()) }

/// One balanced entry for the whole session: payments received vs sales, taxes and customer-account settlements.
pub fn close_entry(env: &Env, session: i64, cash_difference: f64) -> Result<Option<i64>> {
    if env.reg.model("account.move").is_err() || env.reg.model("account.move.line").is_err() { return Ok(None); }
    let e = env.sudo();
    let s = rec(&e, "pos.session", session)?; let cfg = rec(&e, "pos.config", id_of(&s, "config_id").unwrap_or(0))?;
    let (mut sales, mut tax, mut settled) = (0.0, 0.0, 0.0);
    let mut by_method: BTreeMap<(String, &'static str), f64> = BTreeMap::new();
    for o in children(&e, "pos.order", "session_id", session)? {
        let oid = o["id"].as_i64().unwrap();
        let (total, t) = (num(&o, "amount_total"), num(&o, "amount_tax"));
        if flag(&o, "to_invoice") { settled += total; } else { sales += total - t; tax += t; }
        for p in children(&e, "pos.payment", "pos_order_id", oid)? {
            let m = id_of(&p, "payment_method_id").map(|m| rec(&e, "pos.payment.method", m)).transpose()?.unwrap_or_default();
            let kind = if flag(&m, "is_cash_count") { "cash" } else if flag(&m, "split_transactions") { "account" } else { "bank" };
            *by_method.entry((text(&m, "name").unwrap_or_default(), kind)).or_default() += num(&p, "amount");
        }
    }
    if by_method.is_empty() { return Ok(None); }
    let journal = match id_of(&cfg, "journal_id") { Some(j) => j, None => account::ensure_journal(&e, "sale", "POS", "Point of Sale")? };
    let mv = orm::create(&e, "account.move", row(&[("move_type", "entry".into()), ("journal_id", journal.into()), ("ref", text(&s, "name").unwrap_or_default().into()), ("date", orm::today().into())]))?;
    let line = |name: &str, acct: i64, amount: f64| -> Result<()> {   // amount > 0 = debit
        if amount.abs() < 0.005 { return Ok(()); }
        let (dr, cr) = if amount > 0.0 { (r2(amount), 0.0) } else { (0.0, r2(-amount)) };
        orm::create(&e, "account.move.line", row(&[("move_id", mv.into()), ("name", name.into()), ("account_id", acct.into()), ("debit", dr.into()), ("credit", cr.into()), ("balance", (dr - cr).into())])).map(|_| ())
    };
    for ((name, kind), amt) in &by_method {
        let acct = account::ensure_account(&e, if *kind == "account" { "asset_receivable" } else { "asset_cash" }, if *kind == "cash" { "Cash" } else if *kind == "bank" { "Bank" } else { "Account Receivable" })?;
        line(&format!("{} - {}", text(&s, "name").unwrap_or_default(), name), acct, *amt)?;
    }
    line("Sales", account::ensure_account(&e, "income", "Product Sales")?, -sales)?;
    line("Taxes", account::ensure_account(&e, "liability_current", "Tax Payable")?, -tax)?;
    line("Invoiced orders", account::ensure_account(&e, "asset_receivable", "Account Receivable")?, -settled)?;
    if cash_difference.abs() >= 0.005 {
        line("Cash difference", account::ensure_account(&e, "asset_cash", "Cash")?, cash_difference)?;
        line("Cash difference", account::ensure_account(&e, if cash_difference > 0.0 { "income_other" } else { "expense" }, "Cash Difference")?, -cash_difference)?;
    }
    // rounding (cash rounding) is booked against sales so the entry always balances
    let lines = children(&e, "account.move.line", "move_id", mv)?;
    let net: f64 = lines.iter().map(|l| num(l, "debit") - num(l, "credit")).sum();
    if net.abs() >= 0.005 { line("Rounding", account::ensure_account(&e, "income", "Product Sales")?, -net)?; }
    account::post(&e, mv).map_err(|x| OdooError::User(format!("Could not post the session entry: {x}")))?;
    let _ = Domain::True; let _ = Value::Null;
    Ok(Some(mv))
}

fn adjust_lot_quant(env: &Env, product: i64, loc: i64, lot: i64, delta: f64) -> Result<()> {
    let dom = odoo_core::Domain::And(vec![term("product_id", "=", product), term("location_id", "=", loc), term("lot_id", "=", lot)]);
    match find_one(env, "stock.quant", dom)? {
        Some(q) => { let cur = num(&rec(env, "stock.quant", q)?, "quantity"); orm::write(env, "stock.quant", &[q], row(&[("quantity", (cur + delta).into())])) }
        None => orm::create(env, "stock.quant", row(&[("product_id", product.into()), ("location_id", loc.into()), ("lot_id", lot.into()), ("quantity", delta.into())])).map(|_| ()),
    }
}
