//! account: invoices/bills, double-entry posting, payments. Pure transitions over `Env`.
use crate::tax;
use crate::util::*;
use odoo_core::orm::{self, Env};
use odoo_core::{OdooError, Result, Row, Rules, Value};

pub fn ensure_account(env: &Env, account_type: &str, name: &str) -> Result<i64> {
    let e = env.sudo();
    let mut vals = row(&[("name", name.into()), ("account_type", account_type.into())]);
    if env.reg.field("account.account", "code_store").is_ok() { vals.insert("code_store".into(), Value::Text(format!("{{\"1\":\"{}\"}}", account_type))); }
    find_or_create(&e, "account.account", term("account_type", "=", account_type), vals)
}
pub fn ensure_journal(env: &Env, jtype: &str, code: &str, name: &str) -> Result<i64> {
    find_or_create(&env.sudo(), "account.journal", term("type", "=", jtype), row(&[("name", name.into()), ("code", code.into()), ("type", jtype.into())]))
}

fn is_product_line(l: &Row) -> bool { !matches!(text(l, "display_type").as_deref(), Some("line_section" | "line_note" | "tax" | "payment_term" | "rounding")) && l.get("product_id").map_or(false, |v| !v.is_null()) || text(l, "display_type").as_deref() == Some("product") }

fn line_amounts(env: &Env, l: &Row) -> Result<(f64, f64, f64)> {
    let taxes = tax::load(env, &ids(l, "tax_ids"))?;
    Ok(tax::compute(num(l, "quantity"), num(l, "price_unit"), num(l, "discount"), &taxes))
}

fn line_subtotal(env: &Env, r: &Row) -> Result<Value> { if is_product_line(r) { Ok(line_amounts(env, r)?.0.into()) } else { Ok(0.0.into()) } }
fn line_total(env: &Env, r: &Row) -> Result<Value> { if is_product_line(r) { Ok(line_amounts(env, r)?.2.into()) } else { Ok(0.0.into()) } }

fn sign_of(move_type: &str) -> f64 { match move_type { "out_invoice" | "in_refund" => 1.0, _ => -1.0 } }  // + => customer receivable side debit

fn totals(env: &Env, move_id: i64) -> Result<(f64, f64)> {
    let mut u = 0.0; let mut t = 0.0;
    for l in children(env, "account.move.line", "move_id", move_id)?.iter().filter(|l| is_product_line(l) && text(l, "display_type").as_deref() != Some("tax") && text(l, "display_type").as_deref() != Some("payment_term")) {
        let (a, _, c) = line_amounts(env, l)?; u += a; t += c;
    }
    Ok((r2(u), r2(t)))
}

pub fn rules() -> Rules {
    Rules::default()
        .compute("account.move.line", "price_subtotal", line_subtotal)
        .compute("account.move.line", "price_total", line_total)
        .compute("account.move", "amount_untaxed", |env, r| Ok(totals(env, r["id"].as_i64().unwrap())?.0.into()))
        .compute("account.move", "amount_total", |env, r| Ok(totals(env, r["id"].as_i64().unwrap())?.1.into()))
        .compute("account.move", "amount_tax", |env, r| { let (u, t) = totals(env, r["id"].as_i64().unwrap())?; Ok(r2(t - u).into()) })
        .compute("account.move", "payment_state", |_, r| {
            let st = text(r, "state").unwrap_or_default();
            Ok(Value::Text(if st != "posted" { "not_paid" } else if num(r, "amount_residual").abs() < 0.005 { "paid" } else if num(r, "amount_residual") + 0.005 < num(r, "amount_total") { "partial" } else { "not_paid" }.into()))
        })
        .before_create("account.move", |env, mut v| {
            v.entry("move_type".into()).or_insert("entry".into());
            if !v.contains_key("journal_id") {
                let (jt, code, name) = match text(&v, "move_type").as_deref() { Some("out_invoice" | "out_refund") => ("sale", "INV", "Customer Invoices"), Some("in_invoice" | "in_refund") => ("purchase", "BILL", "Vendor Bills"), _ => ("general", "MISC", "Miscellaneous Operations") };
                v.insert("journal_id".into(), ensure_journal(env, jt, code, name)?.into());
            }
            v.entry("name".into()).or_insert("/".into());
            Ok(v)
        })
        .after_create("account.move", |env, ids, _| { let _ = (env, ids); Ok(()) })
        .after_create("account.move.line", |env, _, vals| { if let Some(m) = vals.get("move_id").and_then(|v| v.as_i64()) { orm::recompute_ids(env, "account.move", &[m])?; } Ok(()) })
        .after_write("account.move.line", |env, ids, _| { let mv: Vec<i64> = ids.iter().filter_map(|i| rec(env, "account.move.line", *i).ok().and_then(|r| id_of(&r, "move_id"))).collect(); orm::recompute_ids(env, "account.move", &mv) })
        .on_unlink("account.move.line", |env, ids| Ok(vec![("account.move".into(), ids.iter().filter_map(|i| rec(env, "account.move.line", *i).ok().and_then(|r| id_of(&r, "move_id"))).collect())]))
        .action("account.move", "action_post", |env, ids, _| { for id in ids { post(env, *id)?; } Ok(Value::Bool(true)) })
        .action("account.move", "button_draft", |env, ids, _| {
            for id in ids {
                let e = env.sudo();
                for l in children(&e, "account.move.line", "move_id", *id)?.iter().filter(|l| matches!(text(l, "display_type").as_deref(), Some("tax" | "payment_term"))) { orm::unlink(&e, "account.move.line", &[l["id"].as_i64().unwrap()])?; }
                orm::write(&e, "account.move", &[*id], row(&[("state", "draft".into())]))?;
            }
            Ok(Value::Bool(true))
        })
        .action("account.move", "button_cancel", |env, ids, _| { orm::write(&env.sudo(), "account.move", ids, row(&[("state", "cancel".into())]))?; Ok(Value::Bool(true)) })
        .action("account.move", "action_register_payment", |env, ids, args| { for id in ids { register_payment(env, *id, args.get("amount").and_then(|v| v.as_f64()))?; } Ok(Value::Bool(true)) })
}

/// Post: assign number, generate balanced tax + receivable/payable lines. Idempotent guard on state.
pub fn post(env: &Env, id: i64) -> Result<()> {
    let e = env.sudo();
    let m = rec(&e, "account.move", id)?;
    match text(&m, "state").as_deref() { Some("posted") => return Ok(()), Some("cancel") => return Err(OdooError::User("cannot post a cancelled entry".into())), _ => {} }
    let mt = text(&m, "move_type").unwrap_or_else(|| "entry".into());
    let lines = children(&e, "account.move.line", "move_id", id)?;
    let (untaxed, total) = totals(&e, id)?;
    if mt != "entry" {
        if untaxed == 0.0 && total == 0.0 { return Err(OdooError::User("cannot post an invoice with no amount".into())); }
        let (inc_t, rec_t) = if mt.starts_with("out") { ("income", "asset_receivable") } else { ("expense", "liability_payable") };
        let s = sign_of(&mt);
        let income = ensure_account(&e, inc_t, if mt.starts_with("out") { "Product Sales" } else { "Expenses" })?;
        let recv = ensure_account(&e, rec_t, if mt.starts_with("out") { "Account Receivable" } else { "Account Payable" })?;
        let tax_acc = ensure_account(&e, if mt.starts_with("out") { "liability_current" } else { "asset_current" }, "Tax Payable")?;
        let partner = id_of(&m, "partner_id");
        let side = |amount: f64, debit_if_positive: bool| -> (f64, f64) { let d = if debit_if_positive { amount } else { -amount }; if d >= 0.0 { (d, 0.0) } else { (0.0, -d) } };
        let mut touched = vec![];
        for l in lines.iter().filter(|l| is_product_line(l)) {
            let (sub, _, _) = line_amounts(&e, l)?;
            let (dr, cr) = side(sub, s < 0.0);
            let mut v = row(&[("debit", dr.into()), ("credit", cr.into()), ("balance", (dr - cr).into()), ("display_type", "product".into())]);
            if l.get("account_id").map_or(true, |a| a.is_null()) { v.insert("account_id".into(), income.into()); }
            orm::write(&e, "account.move.line", &[l["id"].as_i64().unwrap()], v)?; touched.push(sub);
        }
        let tax_amt = r2(total - untaxed);
        let mk = |name: &str, acc: i64, dr: f64, cr: f64, dt: &str, residual: f64| -> Row {
            let mut r = row(&[("move_id", id.into()), ("name", name.into()), ("account_id", acc.into()), ("debit", dr.into()), ("credit", cr.into()), ("balance", (dr - cr).into()), ("display_type", dt.into()), ("amount_residual", residual.into()), ("quantity", 0.0.into()), ("price_unit", 0.0.into())]);
            if let Some(p) = partner { r.insert("partner_id".into(), p.into()); } r
        };
        if tax_amt.abs() > 0.004 { let (dr, cr) = side(tax_amt, s < 0.0); orm::create(&e, "account.move.line", mk("Tax", tax_acc, dr, cr, "tax", 0.0))?; }
        let (dr, cr) = side(total, s > 0.0);
        orm::create(&e, "account.move.line", mk("Receivable/Payable", recv, dr, cr, "payment_term", total * s.abs()))?;
        orm::write(&e, "account.move", &[id], row(&[("amount_residual", total.into())]))?;
    }
    let name = if text(&m, "name").map_or(true, |n| n == "/" || n == "New") {
        let (code, prefix) = match mt.as_str() { "out_invoice" => ("account.move.out_invoice", "INV/"), "out_refund" => ("account.move.out_refund", "RINV/"), "in_invoice" => ("account.move.in_invoice", "BILL/"), "in_refund" => ("account.move.in_refund", "RBILL/"), _ => ("account.move.entry", "MISC/") };
        let year = &orm::today()[..4];
        next_seq(&e, &format!("{code}.{year}"), &format!("{prefix}{year}/"), 5)?
    } else { text(&m, "name").unwrap() };
    let mut v = row(&[("state", "posted".into()), ("name", name.into())]);
    if m.get("date").map_or(true, |d| d.is_null()) { v.insert("date".into(), orm::today().into()); }
    if mt != "entry" && m.get("invoice_date").map_or(true, |d| d.is_null()) { v.insert("invoice_date".into(), orm::today().into()); }
    orm::write(&e, "account.move", &[id], v)?;
    check_balanced(&e, id)
}

pub fn check_balanced(env: &Env, id: i64) -> Result<()> {
    let lines = children(env, "account.move.line", "move_id", id)?;
    let (d, c): (f64, f64) = lines.iter().fold((0.0, 0.0), |(d, c), l| (d + num(l, "debit"), c + num(l, "credit")));
    if (d - c).abs() > 0.005 { Err(OdooError::User(format!("Cannot create unbalanced journal entry (debit {d:.2} != credit {c:.2})"))) } else { Ok(()) }
}

/// Registers a payment as a posted journal entry (bank vs receivable/payable) and updates residual/payment_state.
pub fn register_payment(env: &Env, id: i64, amount: Option<f64>) -> Result<()> {
    let e = env.sudo();
    let m = rec(&e, "account.move", id)?;
    if text(&m, "state").as_deref() != Some("posted") { return Err(OdooError::User("only posted invoices can be paid".into())); }
    let residual = num(&m, "amount_residual");
    let pay = amount.unwrap_or(residual).min(residual);
    if pay <= 0.0 { return Err(OdooError::User("nothing to pay".into())); }
    let mt = text(&m, "move_type").unwrap_or_default();
    let inbound = mt == "out_invoice" || mt == "in_refund";
    let bank_j = ensure_journal(&e, "bank", "BNK1", "Bank")?;
    let bank = ensure_account(&e, "asset_cash", "Bank")?;
    let counter = ensure_account(&e, if mt.starts_with("out") { "asset_receivable" } else { "liability_payable" }, if mt.starts_with("out") { "Account Receivable" } else { "Account Payable" })?;
    let pm = orm::create(&e, "account.move", row(&[("move_type", "entry".into()), ("journal_id", bank_j.into()), ("ref", format!("Payment {}", text(&m, "name").unwrap_or_default()).into()), ("date", orm::today().into())]))?;
    let (bdr, bcr) = if inbound { (pay, 0.0) } else { (0.0, pay) };
    for (acc, dr, cr, nm) in [(bank, bdr, bcr, "Bank"), (counter, bcr, bdr, "Counterpart")] {
        let mut v = row(&[("move_id", pm.into()), ("account_id", acc.into()), ("name", nm.into()), ("debit", dr.into()), ("credit", cr.into()), ("balance", (dr - cr).into()), ("display_type", "payment_term".into())]);
        if let Some(p) = id_of(&m, "partner_id") { v.insert("partner_id".into(), p.into()); }
        orm::create(&e, "account.move.line", v)?;
    }
    post_entry(&e, pm)?;
    let new_res = r2(residual - pay);
    orm::write(&e, "account.move", &[id], row(&[("amount_residual", new_res.into())]))?;
    orm::recompute_ids(&e, "account.move", &[id])?;
    Ok(())
}

fn post_entry(env: &Env, id: i64) -> Result<()> {
    check_balanced(env, id)?;
    let n = next_seq(env, "account.move.entry", "MISC/", 5)?;
    orm::write(env, "account.move", &[id], row(&[("state", "posted".into()), ("name", n.into())]))
}
