//! pos.session behaviour ported from `addons/point_of_sale/models/pos_session.py`: computes, opening/closing control,
//! cash in/out, closing data and the validation that books the session's journal entry (through `pos_post::close_entry`).
//!
//! Not ported: stock picking at closing (`update_stock_at_closing`; the runtime already moves stock per order), receivable
//! reconciliation and the force-close wizard (the entry written by `close_entry` always balances).
use crate::pos_methods::{cash_control_of, flag, has_model, id_list, invalid, pm_type, user_err};
use crate::{account, pos_post, util::*};
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, OdooError, Result, Row, Rules, Value};

fn sid_of(ids_: &[i64]) -> Result<i64> { ids_.first().copied().ok_or_else(|| OdooError::User("Select a session".into())) }
fn sess_cfg(env: &Env, s: &Row) -> Result<Row> { rec(env, "pos.config", id_of(s, "config_id").ok_or_else(|| OdooError::User("You should assign a Point of Sale to your session.".into()))?) }

pub(crate) fn orders_of(env: &Env, sid: i64) -> Result<Vec<Row>> { children(env, "pos.order", "session_id", sid) }
/// `_get_closed_orders`: orders neither draft nor cancelled.
pub(crate) fn closed_orders(env: &Env, sid: i64) -> Result<Vec<Row>> { Ok(orders_of(env, sid)?.into_iter().filter(|o| !matches!(text(o, "state").as_deref(), Some("draft" | "cancel"))).collect()) }
/// Payments of orders that are paid, invoiced or done (`_get_captured_payments_domain`).
pub(crate) fn captured_payments(env: &Env, sid: i64) -> Result<Vec<Row>> {
    let mut out = vec![];
    for o in orders_of(env, sid)? { if matches!(text(&o, "state").as_deref(), Some("paid" | "invoiced" | "done")) { out.extend(children(env, "pos.payment", "pos_order_id", o["id"].as_i64().unwrap())?); } }
    Ok(out)
}
fn session_payment_methods(env: &Env, s: &Row) -> Result<Vec<Row>> {
    let mut pms = crate::pos_methods::many(env, "pos.payment.method", &ids(&sess_cfg(env, s)?, "payment_method_ids"))?;
    pms.sort_by_key(|p| (p.get("sequence").and_then(|v| v.as_i64()).unwrap_or(0), p["id"].as_i64().unwrap_or(0)));
    Ok(pms)
}
fn first_cash_method(env: &Env, s: &Row) -> Result<Option<Row>> { Ok(session_payment_methods(env, s)?.into_iter().find(|p| flag(p, "is_cash_count"))) }
fn statement_lines(env: &Env, s: &Row) -> Result<Vec<Row>> { crate::pos_methods::many(env, "account.bank.statement.line", &ids(s, "statement_line_ids")) }

/// `_compute_cash_balance`: (theoretical closing balance, difference with the counted one).
fn cash_balance(env: &Env, s: &Row) -> Result<(f64, f64)> {
    let Some(pm) = first_cash_method(env, s)? else { return Ok((0.0, 0.0)) };
    let pmid = pm["id"].as_i64().unwrap();
    let total_cash_payment: f64 = captured_payments(env, s["id"].as_i64().unwrap())?.iter().filter(|p| id_of(p, "payment_method_id") == Some(pmid)).map(|p| num(p, "amount")).sum();
    let total_cash = if text(s, "state").as_deref() == Some("closed") { num(s, "cash_real_transaction") + total_cash_payment } else { statement_lines(env, s)?.iter().map(|l| num(l, "amount")).sum::<f64>() + total_cash_payment };
    let end = r2(num(s, "cash_register_balance_start") + total_cash);
    Ok((end, r2(num(s, "cash_register_balance_end_real") - end)))
}

pub fn rules() -> Rules {
    Rules::default()
        .compute("pos.session", "is_in_company_currency", |env, r| {
            let cur = id_of(r, "config_id").and_then(|c| crate::pos_methods::config_currency_id(env, c));
            let comp = id_of(r, "config_id").and_then(|c| rec(env, "pos.config", c).ok()).and_then(|c| id_of(&c, "company_id")).and_then(|c| rec(env, "res.company", c).ok()).and_then(|c| id_of(&c, "currency_id"));
            Ok((cur == comp).into())
        })
        .compute("pos.session", "cash_control", |env, r| {
            if id_of(r, "cash_journal_id").is_none() { return Ok(false.into()); }
            Ok(cash_control_of(env, &sess_cfg(env, r)?)?.into())
        })
        .compute("pos.session", "cash_journal_id", |env, r| {
            Ok(first_cash_method(env, r)?.and_then(|p| id_of(&p, "journal_id")).map_or(Value::Null, Value::Int))
        })
        .compute("pos.session", "cash_register_balance_end", |env, r| Ok(cash_balance(env, r)?.0.into()))
        .compute("pos.session", "cash_register_difference", |env, r| Ok(cash_balance(env, r)?.1.into()))
        .compute("pos.session", "total_payments_amount", |env, r| Ok(r2(captured_payments(env, r["id"].as_i64().unwrap())?.iter().map(|p| num(p, "amount")).sum()).into()))
        .compute("pos.session", "order_count", |env, r| Ok((orders_of(env, r["id"].as_i64().unwrap())?.len() as i64).into()))
        .compute("pos.session", "picking_count", |env, r| Ok((pickings_of(env, r["id"].as_i64().unwrap())?.len() as i64).into()))
        .compute("pos.session", "failed_pickings", |env, r| Ok(pickings_of(env, r["id"].as_i64().unwrap())?.iter().any(|p| text(p, "state").as_deref() != Some("done")).into()))
        // update_stock_at_closing comes from the company setting
        .before_create("pos.session", |env, mut v| {
            if !v.contains_key("update_stock_at_closing") {
                let closing = id_of(&v, "config_id").and_then(|c| rec(env, "pos.config", c).ok()).and_then(|c| id_of(&c, "company_id")).and_then(|c| rec(env, "res.company", c).ok()).and_then(|c| text(&c, "point_of_sale_update_stock_quantities")).as_deref() == Some("closing");
                v.insert("update_stock_at_closing".into(), closing.into());
            }
            Ok(v)
        })
        .after_create("pos.session", |env, ids_, _| {
            for i in ids_ { check_pos_config(env, *i)?; open_session(env, *i)?; }
            Ok(())
        })
        .action("pos.session", "_check_pos_config", |env, ids_, _| { for i in ids_ { check_pos_config(env, *i)?; } Ok(Value::Bool(true)) })
        .action("pos.session", "action_pos_session_open", |env, ids_, _| { for i in ids_ { open_session(env, *i)?; } Ok(Value::Bool(true)) })
        .action("pos.session", "set_opening_control", |env, ids_, kw| {
            let sid = sid_of(ids_)?; let s = rec(env, "pos.session", sid)?;
            if text(&s, "state").as_deref() != Some("opening_control") { return Ok(Value::Null); }
            set_opening_control_data(env, sid, num(kw, "cashbox_value"), text(kw, "notes").unwrap_or_default())?;
            if !flag(&s, "rescue") {
                let name = next_seq(&env.sudo(), "pos.session", "POS/", 5)?;
                orm::write(&env.sudo(), "pos.session", &[sid], row(&[("name", name.into())]))?;
            }
            Ok(Value::Null)
        })
        .action("pos.session", "_set_opening_control_data", |env, ids_, kw| { set_opening_control_data(env, sid_of(ids_)?, num(kw, "cashbox_value"), text(kw, "notes").unwrap_or_default())?; Ok(Value::Null) })
        .action("pos.session", "get_session_orders", |env, ids_, _| Ok(id_list(&orders_of(env, sid_of(ids_)?)?.iter().filter_map(|o| o["id"].as_i64()).collect::<Vec<_>>())))
        .action("pos.session", "_get_closed_orders", |env, ids_, _| Ok(id_list(&closed_orders(env, sid_of(ids_)?)?.iter().filter_map(|o| o["id"].as_i64()).collect::<Vec<_>>())))
        .action("pos.session", "_get_captured_payments_domain", |_, ids_, _| Ok(Value::List(vec![
            Value::List(vec!["session_id".into(), "in".into(), id_list(ids_)]), Value::List(vec!["pos_order_id.state".into(), "in".into(), Value::List(vec!["paid".into(), "invoiced".into(), "done".into()])])])))
        .action("pos.session", "_check_if_no_draft_orders", |env, ids_, _| { check_no_draft(env, sid_of(ids_)?)?; Ok(Value::Bool(true)) })
        .action("pos.session", "_check_invoices_are_posted", |env, ids_, _| { check_invoices_posted(env, sid_of(ids_)?)?; Ok(Value::Bool(true)) })
        .action("pos.session", "action_pos_session_closing_control", |env, ids_, kw| closing_control(env, sid_of(ids_)?, &diffs_of(kw)))
        .action("pos.session", "action_pos_session_validate", |env, ids_, kw| validate_session(env, sid_of(ids_)?, &diffs_of(kw)))
        .action("pos.session", "action_pos_session_close", |env, ids_, kw| validate_session(env, sid_of(ids_)?, &diffs_of(kw)))
        .action("pos.session", "_validate_session", |env, ids_, kw| validate_session(env, sid_of(ids_)?, &diffs_of(kw)))
        .action("pos.session", "_post_statement_difference", |env, ids_, kw| { post_statement_difference(env, sid_of(ids_)?, num(kw, "amount"))?; Ok(Value::Null) })
        .action("pos.session", "_cannot_close_session", |env, ids_, kw| Ok(cannot_close(env, sid_of(ids_)?, &diffs_of(kw))?.map_or(Value::Null, Value::Map)))
        .action("pos.session", "close_session_from_ui", |env, ids_, kw| {
            let sid = sid_of(ids_)?;
            let open_orders: Vec<i64> = orders_of(env, sid)?.iter().filter(|o| text(o, "state").as_deref() == Some("draft")).filter_map(|o| o["id"].as_i64()).collect();
            let diffs = diffs_of(kw);
            if let Some(mut check) = cannot_close(env, sid, &diffs)? { check.insert("open_order_ids".into(), id_list(&open_orders)); return Ok(Value::Map(check)); }
            let res = closing_control(env, sid, &diffs)?;
            if let Value::Map(m) = res {
                return Ok(Value::Map(row(&[("open_order_ids", id_list(&open_orders)), ("successful", false.into()), ("message", m.get("name").cloned().unwrap_or(Value::Null)), ("redirect", true.into())])));
            }
            Ok(Value::Map(row(&[("successful", true.into())])))
        })
        .action("pos.session", "update_closing_control_state_session", |env, ids_, kw| {
            let sid = sid_of(ids_)?; let s = rec(env, "pos.session", sid)?;
            if text(&s, "state").as_deref() == Some("closed") { return user_err("This session is already closed."); }
            orm::write(&env.sudo(), "pos.session", &[sid], row(&[("state", "closing_control".into()), ("stop_at", orm::now().into()), ("closing_notes", text(kw, "notes").unwrap_or_default().into())]))?;
            Ok(Value::Null)
        })
        .action("pos.session", "post_closing_cash_details", |env, ids_, kw| {
            let sid = sid_of(ids_)?;
            if let Some(mut check) = cannot_close(env, sid, &[])? {
                let open: Vec<i64> = orders_of(env, sid)?.iter().filter(|o| text(o, "state").as_deref() == Some("draft")).filter_map(|o| o["id"].as_i64()).collect();
                check.insert("open_order_ids".into(), id_list(&open)); return Ok(Value::Map(check));
            }
            let s = rec(env, "pos.session", sid)?;
            if id_of(&s, "cash_journal_id").is_none() { return user_err("There is no cash register in this session."); }
            orm::write(&env.sudo(), "pos.session", &[sid], row(&[("cash_register_balance_end_real", num(kw, "counted_cash").into())]))?;
            Ok(Value::Map(row(&[("successful", true.into())])))
        })
        .action("pos.session", "get_closing_control_data", |env, ids_, _| closing_control_data(env, sid_of(ids_)?))
        .action("pos.session", "try_cash_in_out", |env, ids_, kw| {
            let sign = if text(kw, "_type").as_deref() == Some("in") { 1.0 } else { -1.0 };
            let extras = match kw.get("extras") { Some(Value::Map(m)) => m.clone(), _ => Row::new() };
            let mut made = false;
            for sid in ids_ {
                let s = rec(env, "pos.session", *sid)?;
                let Some(j) = id_of(&s, "cash_journal_id") else { continue };
                let reference = [text(&s, "name").unwrap_or_default(), text(&extras, "translatedType").unwrap_or_default(), text(kw, "reason").unwrap_or_default()].join("-");
                create_statement_line(env, *sid, j, sign * num(kw, "amount"), &reference, &orm::today())?; made = true;
            }
            if !made { return user_err("There is no cash payment method for this PoS Session"); }
            Ok(Value::Null)
        })
        .action("pos.session", "delete_opening_control_session", |env, ids_, _| {
            let sid = sid_of(ids_)?;
            if rec(env, "pos.session", sid).is_err() { return Ok(Value::Map(row(&[("status", "success".into())]))); }
            let s = rec(env, "pos.session", sid)?;
            if text(&s, "state").as_deref() != Some("opening_control") || !orders_of(env, sid)?.is_empty() { return user_err("You can only cancel a session that is in opening control state and has no orders."); }
            for l in ids(&s, "statement_line_ids") { orm::unlink(&env.sudo(), "account.bank.statement.line", &[l])?; }
            orm::unlink(&env.sudo(), "pos.session", &[sid])?;
            Ok(Value::Map(row(&[("status", "success".into())])))
        })
        .action("pos.session", "get_total_discount", |env, ids_, _| {
            let mut amount = 0.0;
            for o in closed_orders(env, sid_of(ids_)?)? { for l in children(env, "pos.order.line", "order_id", o["id"].as_i64().unwrap())? { if num(&l, "discount") > 0.0 { amount += crate::pos_order_methods::line_discount_amount(env, &l)?; } } }
            Ok(r2(amount).into())
        })
        .action("pos.session", "_get_invoice_total_list", |env, ids_, _| {
            let mut out = vec![];
            for o in orders_of(env, sid_of(ids_)?)? {
                if let Some(m) = id_of(&o, "account_move").and_then(|m| rec(env, "account.move", m).ok()) {
                    out.push(Value::Map(row(&[("total", m.get("amount_total").cloned().unwrap_or(0.0.into())), ("name", m.get("name").cloned().unwrap_or(Value::Null)), ("order_ref", o.get("pos_reference").cloned().unwrap_or(Value::Null))])));
                }
            }
            Ok(Value::List(out))
        })
        .action("pos.session", "_get_total_invoice", |env, ids_, _| Ok(orders_of(env, sid_of(ids_)?)?.iter().filter(|o| id_of(o, "account_move").is_some()).map(|o| num(o, "amount_paid")).sum::<f64>().into()))
        .action("pos.session", "action_view_order", |_, ids_, _| Ok(Value::Map(row(&[("name", "Orders".into()), ("res_model", "pos.order".into()), ("view_mode", "list,form".into()), ("type", "ir.actions.act_window".into()), ("domain", Value::List(vec![Value::List(vec!["session_id".into(), "in".into(), id_list(ids_)])]))]))))
        .action("pos.session", "action_stock_picking", |env, ids_, _| {
            let sid = sid_of(ids_)?; let pk: Vec<i64> = pickings_of(env, sid)?.iter().filter_map(|p| p["id"].as_i64()).collect();
            Ok(Value::Map(row(&[("display_name", "Pickings".into()), ("res_model", "stock.picking".into()), ("type", "ir.actions.act_window".into()), ("domain", Value::List(vec![Value::List(vec!["id".into(), "in".into(), id_list(&pk)])]))])))
        })
        .action("pos.session", "login", |env, ids_, _| {
            // an ir.sequence per session, created on first login, counts every resume of the session
            let sid = sid_of(ids_)?; let code = format!("pos.session.login_number{sid}");
            if find_one(&env.sudo(), "ir.sequence", term("code", "=", code.as_str()))?.is_none() {
                orm::create(&env.sudo(), "ir.sequence", row(&[("name", format!("POS Session {sid}").into()), ("code", code.as_str().into()), ("padding", 0.into()), ("number_next", 1.into()), ("number_increment", 1.into())]))?;
            }
            Ok(Value::Text(next_seq(&env.sudo(), &code, "", 0)?))
        })
        .action("pos.session", "_get_diff_vals", |env, ids_, kw| {
            let s = rec(env, "pos.session", sid_of(ids_)?)?;
            Ok(match diff_vals(env, &s, kw.get("payment_method_id").and_then(|v| v.as_i64()).unwrap_or(0), num(kw, "diff_amount"), kw.get("outstanding_account").and_then(|v| v.as_i64()))? {
                Some((src, dst, amt)) => Value::List(vec![Value::Map(row(&[("account_id", src.into()), ("debit", amt.max(0.0).into()), ("credit", (-amt).max(0.0).into())])), Value::Map(row(&[("account_id", dst.into()), ("debit", (-amt).max(0.0).into()), ("credit", amt.max(0.0).into())]))]),
                None => Value::Bool(false),
            })
        })
        .action("pos.session", "_create_diff_account_move_for_split_payment_method", |env, ids_, kw| {
            let sid = sid_of(ids_)?;
            Ok(create_split_diff_move(env, sid, kw.get("payment_method_id").and_then(|v| v.as_i64()).unwrap_or(0), num(kw, "diff_amount"))?.map_or(Value::Bool(false), Value::Int))
        })
}

fn pickings_of(env: &Env, sid: i64) -> Result<Vec<Row>> {
    if !has_model(env, "stock.picking") || env.reg.field("stock.picking", "pos_session_id").is_err() { return Ok(vec![]); }
    children(env, "stock.picking", "pos_session_id", sid)
}
fn diffs_of(kw: &Row) -> Vec<(i64, f64)> {
    let mut out = vec![];
    match kw.get("bank_payment_method_diffs").or_else(|| kw.get("bank_payment_method_diff_pairs")) {
        Some(Value::List(l)) => for p in l { if let Value::List(pair) = p { if let (Some(a), Some(b)) = (pair.first().and_then(|v| v.as_i64()), pair.get(1).and_then(|v| v.as_f64())) { out.push((a, b)); } } },
        Some(Value::Map(m)) => for (k, v) in m { if let (Ok(a), Some(b)) = (k.parse::<i64>(), v.as_f64()) { out.push((a, b)); } },
        _ => {}
    }
    out
}

/// `_check_pos_config`: only one unclosed non-rescue session per config.
fn check_pos_config(env: &Env, sid: i64) -> Result<()> {
    if env.ctx.get("onboarding_creation").map_or(false, |v| v.truthy()) { return Ok(()); }
    let s = rec(env, "pos.session", sid)?; let Some(cfg) = id_of(&s, "config_id") else { return Ok(()) };
    let n = orm::search(&env.sudo(), "pos.session", &Domain::And(vec![term("state", "!=", "closed"), term("config_id", "=", cfg), Domain::Or(vec![term("rescue", "=", false), term("rescue", "=", Value::Bool(false))])]), None, None, 0)?.len();
    if n > 1 { return invalid("Another session is already opened for this point of sale."); }
    Ok(())
}

/// `action_pos_session_open`: a new session of a cash-controlled config starts with the cash the previous one ended with.
fn open_session(env: &Env, sid: i64) -> Result<()> {
    let e = env.sudo(); let s = rec(&e, "pos.session", sid)?;
    if text(&s, "state").as_deref() != Some("opening_control") { return Ok(()); }
    let cfg = sess_cfg(&e, &s)?;
    if cash_control_of(&e, &cfg)? && !flag(&s, "rescue") {
        let last = orm::search(&e, "pos.session", &Domain::And(vec![term("config_id", "=", cfg["id"].as_i64().unwrap()), term("id", "!=", sid)]), Some("id desc"), Some(1), 0)?;
        let start = last.first().map(|l| rec(&e, "pos.session", *l)).transpose()?.map_or(0.0, |l| num(&l, "cash_register_balance_end_real"));
        orm::write(&e, "pos.session", &[sid], row(&[("cash_register_balance_start", start.into())]))?;
    }
    Ok(())
}

fn set_opening_control_data(env: &Env, sid: i64, cashbox_value: f64, notes: String) -> Result<()> {
    let e = env.sudo(); let s = rec(&e, "pos.session", sid)?;
    let mut w = row(&[("state", "opened".into()), ("start_at", orm::now().into())]);
    if !notes.is_empty() { w.insert("opening_notes".into(), notes.into()); }
    if crate::pos_methods::many(&e, "pos.payment.method", &ids(&sess_cfg(&e, &s)?, "payment_method_ids"))?.iter().any(|p| flag(p, "is_cash_count")) { w.insert("cash_register_balance_start".into(), cashbox_value.into()); }
    orm::write(&e, "pos.session", &[sid], w)
}

fn check_no_draft(env: &Env, sid: i64) -> Result<()> {
    let names: Vec<String> = orders_of(env, sid)?.iter().filter(|o| text(o, "state").as_deref() == Some("draft")).map(|o| text(o, "name").unwrap_or_default()).collect();
    if !names.is_empty() { return user_err(format!("There are still orders in draft state in the session. Pay or cancel the following orders to validate the session:\n{}", names.join(", "))); }
    Ok(())
}
fn check_invoices_posted(env: &Env, sid: i64) -> Result<()> {
    let mut bad = vec![];
    for o in closed_orders(env, sid)? { if let Some(m) = id_of(&o, "account_move").and_then(|m| rec(env, "account.move", m).ok()) { if text(&m, "state").as_deref() != Some("posted") { bad.push(format!("{} - {}", text(&m, "name").unwrap_or_default(), text(&m, "state").unwrap_or_default())); } } }
    if !bad.is_empty() { return user_err(format!("You cannot close the POS when invoices are not posted.\nInvoices: {}", bad.join("\n"))); }
    Ok(())
}

/// `_cannot_close_session`: `Some(result dict)` when the session cannot be closed right now.
fn cannot_close(env: &Env, sid: i64, diffs: &[(i64, f64)]) -> Result<Option<Row>> {
    let s = rec(env, "pos.session", sid)?;
    if orders_of(env, sid)?.iter().any(|o| text(o, "state").as_deref() == Some("draft")) {
        return Ok(Some(row(&[("successful", false.into()), ("message", "You cannot close the POS when orders are still in draft".into()), ("redirect", false.into())])));
    }
    if text(&s, "state").as_deref() == Some("closed") {
        return Ok(Some(row(&[("successful", false.into()), ("type", "alert".into()), ("title", "Session already closed".into()), ("message", "The session has been already closed by another User. All sales completed in the meantime have been saved in a Rescue Session, which can be reviewed anytime and posted to Accounting from Point of Sale's dashboard.".into()), ("redirect", true.into())])));
    }
    if !diffs.is_empty() {
        let (mut no_loss, mut no_profit): (Vec<String>, Vec<String>) = (vec![], vec![]);
        for (pm, diff) in diffs {
            let j = rec(env, "pos.payment.method", *pm).ok().and_then(|p| id_of(&p, "journal_id")).and_then(|j| rec(env, "account.journal", j).ok()).unwrap_or_default();
            let name = text(&j, "name").unwrap_or_default();
            if r2(*diff) < 0.0 && id_of(&j, "loss_account_id").is_none() && !no_loss.contains(&name) { no_loss.push(name); }
            else if r2(*diff) > 0.0 && id_of(&j, "profit_account_id").is_none() && !no_profit.contains(&name) { no_profit.push(name); }
        }
        let mut message = String::new();
        if !no_loss.is_empty() { message += &format!("Need loss account for the following journals to post the lost amount: {}\n", no_loss.join(", ")); }
        if !no_profit.is_empty() { message += &format!("Need profit account for the following journals to post the gained amount: {}", no_profit.join(", ")); }
        if !message.is_empty() { return Ok(Some(row(&[("successful", false.into()), ("message", message.into()), ("redirect", false.into())]))); }
    }
    Ok(None)
}

/// `action_pos_session_closing_control`.
fn closing_control(env: &Env, sid: i64, diffs: &[(i64, f64)]) -> Result<Value> {
    let e = env.sudo(); let s = rec(&e, "pos.session", sid)?;
    if orders_of(&e, sid)?.iter().any(|o| text(o, "state").as_deref() == Some("draft")) { return user_err("You cannot close the POS when orders are still in draft"); }
    if text(&s, "state").as_deref() == Some("closed") { return user_err("This session is already closed."); }
    let stop = text(&s, "stop_at").unwrap_or_else(orm::now);
    orm::write(&e, "pos.session", &[sid], row(&[("state", "closing_control".into()), ("stop_at", stop.into())]))?;
    let cfg = sess_cfg(&e, &s)?;
    if !cash_control_of(&e, &cfg)? { return validate_session(env, sid, diffs); }
    if flag(&s, "rescue") {
        // a rescue session cannot be counted at the till: expected cash is what the session received
        let cash = session_payment_methods(&e, &s)?.into_iter().find(|p| pm_type(&e, p) == "cash");
        if let Some(cash) = cash {
            let cid = cash["id"].as_i64().unwrap();
            let total: f64 = closed_orders(&e, sid)?.iter().flat_map(|o| children(&e, "pos.payment", "pos_order_id", o["id"].as_i64().unwrap()).unwrap_or_default()).filter(|p| id_of(p, "payment_method_id") == Some(cid)).map(|p| num(&p, "amount")).sum::<f64>() + num(&s, "cash_register_balance_start");
            orm::write(&e, "pos.session", &[sid], row(&[("cash_register_balance_end_real", total.into())]))?;
        }
    }
    validate_session(env, sid, diffs)
}

/// `_validate_session`: book the session, post the cash difference, settle the orders and close.
fn validate_session(env: &Env, sid: i64, diffs: &[(i64, f64)]) -> Result<Value> {
    let e = env.sudo(); let s = rec(&e, "pos.session", sid)?;
    let live = orders_of(&e, sid)?.iter().any(|o| text(o, "state").as_deref() != Some("cancel")) || !ids(&s, "statement_line_ids").is_empty();
    if live {
        let lines_total: f64 = statement_lines(&e, &s)?.iter().map(|l| num(l, "amount")).sum();
        orm::write(&e, "pos.session", &[sid], row(&[("cash_real_transaction", lines_total.into())]))?;
        if text(&s, "state").as_deref() == Some("closed") { return user_err("This session is already closed."); }
        check_no_draft(&e, sid)?; check_invoices_posted(&e, sid)?;
        let diff_before = cash_balance(&e, &rec(&e, "pos.session", sid)?)?.1;
        let mv = pos_post::close_entry(&e, sid, 0.0)?;
        if let Some(mv) = mv {
            orm::write(&e, "pos.session", &[sid], row(&[("move_id", mv.into())]))?;
            // split bank methods book what the terminal reported against what was taken
            for pm in session_payment_methods(&e, &s)? { if pm_type(&e, &pm) == "bank" && flag(&pm, "split_transactions") { let pid = pm["id"].as_i64().unwrap(); if let Some((_, d)) = diffs.iter().find(|(p, _)| *p == pid) { create_split_diff_move(&e, sid, pid, *d)?; } } }
            for o in orders_of(&e, sid)? { if text(&o, "state").as_deref() == Some("paid") { orm::write(&e, "pos.order", &[o["id"].as_i64().unwrap()], row(&[("state", "done".into())]))?; } }
        }
        post_statement_difference(&e, sid, diff_before)?;
    } else {
        post_statement_difference(&e, sid, cash_balance(&e, &s)?.1)?;
    }
    orm::write(&e, "pos.session", &[sid], row(&[("state", "closed".into())]))?;
    Ok(Value::Bool(true))
}

/// A statement line on the cash journal: the liquidity line on the journal's account against the suspense account, posted.
fn create_statement_line(env: &Env, sid: i64, journal: i64, amount: f64, reference: &str, date: &str) -> Result<i64> {
    let e = env.sudo();
    let line = orm::create(&e, "account.bank.statement.line", row(&[("journal_id", journal.into()), ("amount", amount.into()), ("date", date.into()), ("payment_ref", reference.into()), ("pos_session_id", sid.into())]))?;
    let lr = rec(&e, "account.bank.statement.line", line)?;
    if let Some(mv) = id_of(&lr, "move_id") {
        let j = rec(&e, "account.journal", journal)?;
        let liquidity = match id_of(&j, "default_account_id") { Some(a) => a, None => account::ensure_account(&e, "asset_cash", "Cash")? };
        let suspense = account::ensure_account(&e, "asset_current", "Bank Suspense Account")?;
        for (acct, bal) in [(liquidity, amount), (suspense, -amount)] {
            let (dr, cr) = if bal >= 0.0 { (bal, 0.0) } else { (0.0, -bal) };
            orm::create(&e, "account.move.line", row(&[("move_id", mv.into()), ("name", reference.into()), ("account_id", acct.into()), ("debit", dr.into()), ("credit", cr.into()), ("balance", (dr - cr).into())]))?;
        }
        account::post(&e, mv)?;
    }
    Ok(line)
}

/// `_post_statement_difference`: the cash counted differs from the expected one, so the difference goes to the journal's loss/profit account.
fn post_statement_difference(env: &Env, sid: i64, amount: f64) -> Result<()> {
    if r2(amount) == 0.0 { return Ok(()); }
    let e = env.sudo(); let s = rec(&e, "pos.session", sid)?; let cfg = sess_cfg(&e, &s)?;
    if !cash_control_of(&e, &cfg)? { return Ok(()); }
    let Some(jid) = id_of(&s, "cash_journal_id") else { return Ok(()) };
    let j = rec(&e, "account.journal", jid)?; let jname = text(&j, "name").unwrap_or_default();
    let (reference, counter) = if amount < 0.0 {
        let a = id_of(&j, "loss_account_id").ok_or_else(|| OdooError::User(format!("Please go on the {jname} journal and define a Loss Account. This account will be used to record cash difference.")))?;
        ("Cash difference observed during the counting (Loss) - closing", a)
    } else {
        let a = id_of(&j, "profit_account_id").ok_or_else(|| OdooError::User(format!("Please go on the {jname} journal and define a Profit Account. This account will be used to record cash difference.")))?;
        ("Cash difference observed during the counting (Profit) - closing", a)
    };
    let date = statement_lines(&e, &s)?.iter().filter_map(|l| text(l, "date")).max().unwrap_or_else(orm::today);
    let line = orm::create(&e, "account.bank.statement.line", row(&[("journal_id", jid.into()), ("amount", amount.into()), ("date", date.into()), ("payment_ref", reference.into()), ("pos_session_id", sid.into())]))?;
    if let Some(mv) = id_of(&rec(&e, "account.bank.statement.line", line)?, "move_id") {
        let liquidity = match id_of(&j, "default_account_id") { Some(a) => a, None => account::ensure_account(&e, "asset_cash", "Cash")? };
        for (acct, bal) in [(liquidity, amount), (counter, -amount)] {
            let (dr, cr) = if bal >= 0.0 { (bal, 0.0) } else { (0.0, -bal) };
            orm::create(&e, "account.move.line", row(&[("move_id", mv.into()), ("name", reference.into()), ("account_id", acct.into()), ("debit", dr.into()), ("credit", cr.into()), ("balance", (dr - cr).into())]))?;
        }
        account::post(&e, mv)?;
    }
    Ok(())
}

/// `_get_diff_vals`: (source account, destination account, amount) for a bank-method closing difference.
fn diff_vals(env: &Env, _s: &Row, pm: i64, diff: f64, outstanding: Option<i64>) -> Result<Option<(i64, i64, f64)>> {
    let pmr = rec(env, "pos.payment.method", pm)?; let j = id_of(&pmr, "journal_id").and_then(|j| rec(env, "account.journal", j).ok()).unwrap_or_default();
    let source = id_of(&pmr, "outstanding_account_id").or(outstanding);
    let diff = r2(diff);
    let dest = if diff > 0.0 { id_of(&j, "profit_account_id") } else if diff < 0.0 { id_of(&j, "loss_account_id") } else { None };
    match (diff == 0.0, source, dest) { (false, Some(s), Some(d)) => Ok(Some((s, d, diff))), _ => Ok(None) }
}
fn create_split_diff_move(env: &Env, sid: i64, pm: i64, diff: f64) -> Result<Option<i64>> {
    let e = env.sudo(); let s = rec(&e, "pos.session", sid)?;
    let Some((src, dst, amt)) = diff_vals(&e, &s, pm, diff, None)? else { return Ok(None) };
    let pmr = rec(&e, "pos.payment.method", pm)?;
    let reference = format!("Closing difference in {} ({})", text(&pmr, "name").unwrap_or_default(), text(&s, "name").unwrap_or_default());
    let mut mv = row(&[("move_type", "entry".into()), ("date", orm::today().into()), ("ref", reference.as_str().into())]);
    if let Some(j) = id_of(&pmr, "journal_id") { mv.insert("journal_id".into(), j.into()); }
    let mid = orm::create(&e, "account.move", mv)?;
    for (acct, bal) in [(src, amt), (dst, -amt)] {
        let (dr, cr) = if bal >= 0.0 { (bal, 0.0) } else { (0.0, -bal) };
        orm::create(&e, "account.move.line", row(&[("move_id", mid.into()), ("name", reference.as_str().into()), ("account_id", acct.into()), ("debit", dr.into()), ("credit", cr.into()), ("balance", (dr - cr).into())]))?;
    }
    account::post(&e, mid)?;
    Ok(Some(mid))
}

fn closing_control_data(env: &Env, sid: i64) -> Result<Value> {
    let e = env.sudo(); let s = rec(&e, "pos.session", sid)?;
    let orders = closed_orders(&e, sid)?;
    let mut payments: Vec<Row> = vec![]; let mut by_order_payments: Vec<Row> = vec![];
    for o in &orders { for p in children(&e, "pos.payment", "pos_order_id", o["id"].as_i64().unwrap())? { by_order_payments.push(p.clone()); if id_of(&p, "payment_method_id").and_then(|m| rec(&e, "pos.payment.method", m).ok()).map_or(true, |m| pm_type(&e, &m) != "pay_later") { payments.push(p); } } }
    let pms = session_payment_methods(&e, &s)?;
    let cash_pm = pms.iter().find(|p| pm_type(&e, p) == "cash").cloned();
    let cash_total: f64 = cash_pm.as_ref().map_or(0.0, |c| payments.iter().filter(|p| id_of(p, "payment_method_id") == c["id"].as_i64()).map(|p| num(p, "amount")).sum());
    let mut lines = statement_lines(&e, &s)?; lines.sort_by_key(|l| text(l, "create_date").unwrap_or_default());
    let (mut cin, mut cout) = (0, 0); let mut moves = vec![]; let mut lines_total = 0.0;
    for l in &lines {
        let amt = num(l, "amount"); lines_total += amt;
        let name = if amt > 0.0 { cin += 1; format!("Cash in {cin}") } else { cout += 1; format!("Cash out {cout}") };
        moves.push(Value::Map(row(&[("name", text(l, "payment_ref").filter(|p| !p.is_empty()).unwrap_or(name).into()), ("amount", amt.into())])));
    }
    let cfg = sess_cfg(&e, &s)?;
    let non_cash: Vec<Value> = pms.iter().filter(|p| cash_pm.as_ref().map_or(true, |c| c["id"] != p["id"])).map(|pm| {
        let mine: Vec<&Row> = by_order_payments.iter().filter(|p| id_of(p, "payment_method_id") == pm["id"].as_i64()).collect();
        Value::Map(row(&[("name", pm.get("name").cloned().unwrap_or(Value::Null)), ("amount", r2(mine.iter().map(|p| num(p, "amount")).sum()).into()), ("number", (mine.len() as i64).into()), ("id", pm["id"].clone()), ("type", pm_type(&e, pm).into())]))
    }).collect();
    Ok(Value::Map(row(&[
        ("orders_details", Value::Map(row(&[("quantity", (orders.len() as i64).into()), ("amount", r2(orders.iter().map(|o| num(o, "amount_total")).sum()).into())]))),
        ("opening_notes", s.get("opening_notes").cloned().unwrap_or(Value::Bool(false))),
        ("default_cash_details", match &cash_pm { Some(c) => Value::Map(row(&[("name", c.get("name").cloned().unwrap_or(Value::Null)), ("amount", r2(num(&s, "cash_register_balance_start") + cash_total + lines_total).into()), ("opening", num(&s, "cash_register_balance_start").into()), ("payment_amount", r2(cash_total).into()), ("moves", Value::List(moves)), ("id", c["id"].clone())])), None => Value::Map(Row::new()) }),
        ("non_cash_payment_methods", Value::List(non_cash)),
        ("amount_authorized_diff", if flag(&cfg, "set_maximum_difference") { cfg.get("amount_authorized_diff").cloned().unwrap_or(Value::Null) } else { Value::Null }),
    ])))
}
