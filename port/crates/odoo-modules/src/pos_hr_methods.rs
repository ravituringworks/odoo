//! pos_hr: cashiers by employee (`addons/pos_hr/models`): cashier names, hashed badge/PIN data for the terminal,
//! manager employees on every register, per-employee closing data and the employee sales report.
use crate::pos_methods::{id_list, many, user_err, xmlid};
use crate::util::*;
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, OdooError, Result, Row, Rules, Value};
use std::collections::BTreeMap;

fn has_hr(env: &Env) -> bool { env.reg.field("pos.order", "employee_id").is_ok() }
fn first(ids_: &[i64]) -> Result<i64> { ids_.first().copied().ok_or_else(|| OdooError::User("Select a record".into())) }

/// SHA-1 as lowercase hex (the terminal compares badges and PINs by this hash).
pub fn sha1_hex(data: &[u8]) -> String {
    let mut h: [u32; 5] = [0x6745_2301, 0xEFCD_AB89, 0x98BA_DCFE, 0x1032_5476, 0xC3D2_E1F0];
    let mut msg = data.to_vec(); let bits = (data.len() as u64) * 8;
    msg.push(0x80); while msg.len() % 64 != 56 { msg.push(0); } msg.extend_from_slice(&bits.to_be_bytes());
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 80];
        for i in 0..16 { w[i] = u32::from_be_bytes([chunk[4 * i], chunk[4 * i + 1], chunk[4 * i + 2], chunk[4 * i + 3]]); }
        for i in 16..80 { w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1); }
        let [mut a, mut b, mut c, mut d, mut e] = h;
        for (i, wi) in w.iter().enumerate() {
            let (f, k) = match i { 0..=19 => ((b & c) | (!b & d), 0x5A82_7999), 20..=39 => (b ^ c ^ d, 0x6ED9_EBA1), 40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC), _ => (b ^ c ^ d, 0xCA62_C1D6u32) };
            let t = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(*wi);
            e = d; d = c; c = b.rotate_left(30); b = a; a = t;
        }
        h = [h[0].wrapping_add(a), h[1].wrapping_add(b), h[2].wrapping_add(c), h[3].wrapping_add(d), h[4].wrapping_add(e)];
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

/// Employees of the users in the POS manager group.
fn manager_employees(env: &Env) -> Vec<i64> {
    let e = env.sudo(); let Some(g) = xmlid(&e, "point_of_sale", "group_pos_manager") else { return vec![] };
    let Ok(users) = orm::search(&e, "res.users", &term("groups_id", "in", id_list(&[g])), None, None, 0) else { return vec![] };
    if e.reg.field("hr.employee", "user_id").is_err() { return vec![]; }
    orm::search(&e, "hr.employee", &term("user_id", "in", id_list(&users)), None, None, 0).unwrap_or_default()
}
fn employee_name(env: &Env, id: i64) -> String { rec(env, "hr.employee", id).ok().and_then(|e| text(&e, "name")).unwrap_or_default() }

/// Payments grouped by cashier; payments without one are listed last as "Others".
fn payments_by_employee(env: &Env, payments: &[Row]) -> Value {
    let mut groups: Vec<(Option<i64>, f64)> = vec![];
    for p in payments { let emp = id_of(p, "employee_id"); match groups.iter_mut().find(|g| g.0 == emp) { Some(g) => g.1 += num(p, "amount"), None => groups.push((emp, num(p, "amount"))) } }
    let mut rows: Vec<(bool, String, Value, f64)> = groups.into_iter().map(|(e, a)| match e { Some(i) => (false, employee_name(env, i), Value::Int(i), a), None => (true, "Others".to_string(), Value::Text("others".into()), a) }).collect();
    rows.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    Value::List(rows.into_iter().map(|(_, n, id, a)| Value::Map(row(&[("id", id), ("name", n.as_str().into()), ("amount", a.into())]))).collect())
}
fn moves_by_employee(env: &Env, sid: i64) -> Result<Value> {
    let e = env.sudo(); let s = rec(&e, "pos.session", sid)?; let mut by: BTreeMap<i64, f64> = BTreeMap::new();
    if e.reg.field("account.bank.statement.line", "employee_id").is_err() { return Ok(Value::List(vec![])); }
    for l in many(&e, "account.bank.statement.line", &ids(&s, "statement_line_ids"))? { if let Some(emp) = id_of(&l, "employee_id") { *by.entry(emp).or_default() += num(&l, "amount"); } }
    let mut v: Vec<(i64, f64)> = by.into_iter().collect(); v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    Ok(Value::List(v.into_iter().map(|(i, a)| Value::Map(row(&[("id", i.into()), ("name", employee_name(&e, i).as_str().into()), ("amount", a.into())]))).collect()))
}
/// `get_closing_control_data` of pos_hr: who took which payments and cash moves.
pub fn extend_closing_data(env: &Env, sid: i64, data: &mut Row) -> Result<()> {
    if !has_hr(env) { return Ok(()); }
    let e = env.sudo(); let s = rec(&e, "pos.session", sid)?;
    let mut payments: Vec<Row> = vec![];
    for o in crate::pos_session_methods::closed_orders(&e, sid)? { for p in children(&e, "pos.payment", "pos_order_id", o["id"].as_i64().unwrap())? { payments.push(p); } }
    let cfg = rec(&e, "pos.config", id_of(&s, "config_id").unwrap_or(0))?;
    let mut pms = many(&e, "pos.payment.method", &ids(&cfg, "payment_method_ids"))?; pms.sort_by_key(|p| (p.get("sequence").and_then(|v| v.as_i64()).unwrap_or(0), p["id"].as_i64().unwrap_or(0)));
    let cash = pms.iter().find(|p| crate::pos_methods::pm_type(&e, p) == "cash").and_then(|p| p["id"].as_i64());
    let not_later = |p: &&Row| id_of(p, "payment_method_id").and_then(|m| rec(&e, "pos.payment.method", m).ok()).map_or(true, |m| crate::pos_methods::pm_type(&e, &m) != "pay_later");
    if let Some(Value::Map(cd)) = data.get_mut("default_cash_details") {
        if !cd.is_empty() {
            let mine: Vec<Row> = payments.iter().filter(not_later).filter(|p| id_of(p, "payment_method_id") == cash).cloned().collect();
            cd.insert("amount_per_employee".into(), payments_by_employee(&e, &mine));
            cd.insert("moves_per_employee".into(), moves_by_employee(&e, sid)?);
        }
    }
    if let Some(Value::List(non_cash)) = data.get_mut("non_cash_payment_methods") {
        for pm in non_cash.iter_mut() {
            if let Value::Map(m) = pm { let id = m.get("id").and_then(|v| v.as_i64()); let mine: Vec<Row> = payments.iter().filter(|p| id_of(p, "payment_method_id") == id).cloned().collect(); m.insert("amount_per_employee".into(), payments_by_employee(&e, &mine)); }
        }
    }
    Ok(())
}

pub fn rules() -> Rules {
    Rules::default()
        .compute("pos.order", "cashier", |env, r| {
            let e = env.sudo();
            if let Some(emp) = id_of(r, "employee_id") { return Ok(Value::Text(employee_name(&e, emp))); }
            let n = id_of(r, "user_id").and_then(|u| rec(&e, "res.users", u).ok()).and_then(|u| text(&u, "name").or_else(|| id_of(&u, "partner_id").and_then(|p| rec(&e, "res.partner", p).ok()).and_then(|p| text(&p, "name"))));
            Ok(n.map_or(Value::Bool(false), Value::Text))
        })
        // managers are always among the advanced employees of a register
        .before_write("pos.config", |env, mut v| {
            if env.reg.field("pos.config", "advanced_employee_ids").is_err() { return Ok(v); }
            let extra = manager_employees(env);
            let mut cmds = match v.remove("advanced_employee_ids") { Some(Value::List(l)) => l, _ => vec![] };
            cmds.extend(extra.into_iter().map(|i| Value::List(vec![4.into(), i.into()])));
            v.insert("advanced_employee_ids".into(), Value::List(cmds));
            Ok(v)
        })
        .action("pos.config", "_employee_domain", |env, ids_, kw| {
            let cfg = rec(env, "pos.config", first(ids_)?)?; let user = kw.get("user_id").and_then(|v| v.as_i64());
            let (basic, adv) = (ids(&cfg, "basic_employee_ids"), ids(&cfg, "advanced_employee_ids"));
            let mut d: Vec<Value> = vec![];
            if !basic.is_empty() {
                let allowed: Vec<i64> = basic.iter().chain(adv.iter()).copied().collect();
                d.push(Value::List(vec!["|".into(), Value::List(vec!["user_id".into(), "=".into(), user.map_or(Value::Bool(false), Value::Int)]), Value::List(vec!["id".into(), "in".into(), id_list(&allowed)])]));
            }
            Ok(Value::List(d))
        })
        .action("pos.config", "_get_group_pos_manager", |env, _, _| Ok(xmlid(env, "point_of_sale", "group_pos_manager").map_or(Value::Bool(false), Value::Int)))
        .action("hr.employee", "get_barcodes_and_pin_hashed", |env, ids_, _| {
            let e = env.sudo(); let mut out = vec![];
            for i in ids_ {
                let Ok(emp) = rec(&e, "hr.employee", *i) else { continue };
                let hash = |k: &str| text(&emp, k).filter(|v| !v.is_empty()).map_or(Value::Bool(false), |v| Value::Text(sha1_hex(v.as_bytes())));
                out.push(Value::Map(row(&[("id", (*i).into()), ("barcode", hash("barcode")), ("pin", hash("pin"))])));
            }
            Ok(Value::List(out))
        })
        .on_unlink("hr.employee", |env, ids_| {
            let e = env.sudo(); let mut bad: Vec<String> = vec![];
            if e.reg.field("pos.config", "module_pos_hr").is_err() { return Ok(vec![]); }
            let configs: Vec<Row> = children_by(&e, "pos.config", &term("module_pos_hr", "=", true))?.into_iter().filter(|c| crate::pos_methods::current_session(&e, c["id"].as_i64().unwrap()).ok().flatten().is_some()).collect();
            for emp in ids_ {
                let mut names: Vec<String> = vec![];
                for c in &configs {
                    let (b, a) = (ids(c, "basic_employee_ids"), ids(c, "advanced_employee_ids"));
                    if (b.is_empty() && a.is_empty()) || b.contains(emp) { names.push(text(c, "name").unwrap_or_default()); }
                }
                if !names.is_empty() { bad.push(format!("Employee: {} - PoS Config(s): {} \n", employee_name(&e, *emp), names.join(", "))); }
            }
            if !bad.is_empty() { return user_err(format!("You cannot delete an employee that may be used in an active PoS session, close the session(s) first: \n{}", bad.concat())); }
            Ok(vec![])
        })
        .action("pos.session", "_aggregate_payments_amounts_by_employee", |env, _, kw| {
            let ps = match kw.get("payment_ids") { Some(Value::List(l)) => l.iter().filter_map(|v| v.as_i64()).collect::<Vec<_>>(), _ => vec![] };
            Ok(payments_by_employee(&env.sudo(), &many(&env.sudo(), "pos.payment", &ps)?))
        })
        .action("pos.session", "_aggregate_moves_by_employee", |env, ids_, _| moves_by_employee(env, first(ids_)?))
        // the sales report of one cashier (the period is not forwarded by Odoo's override, so the default day applies)
        .action("report.pos_hr.single_employee_sales_report", "get_sale_details", |env, _, kw| {
            let mut args = Row::new();
            for k in ["config_ids", "session_ids", "employee_id"] { if let Some(v) = kw.get(k) { args.insert(k.into(), v.clone()); } }
            let Value::Map(mut data) = crate::pos_report_methods::sale_details(env, &args)? else { return Ok(Value::Null) };
            if let Some(emp) = kw.get("employee_id").and_then(|v| v.as_i64()) { data.insert("employee_name".into(), Value::Text(employee_name(env, emp))); }
            Ok(Value::Map(data))
        })
        .action("report.pos_hr.single_employee_sales_report", "_get_domain", |env, _, kw| {
            let t = |f: &str, op: &str, v: Value| Value::List(vec![f.into(), op.into(), v]);
            let mut d = match orm::call(env, "report.point_of_sale.report_saledetails", "_get_domain", &[], &{ let mut a = Row::new(); for k in ["config_ids", "session_ids"] { if let Some(v) = kw.get(k) { a.insert(k.into(), v.clone()); } } a })? { Value::List(l) => l, _ => vec![] };
            if let Some(e) = kw.get("employee_id").filter(|v| v.truthy()) { d.push(t("employee_id", "=", e.clone())); }
            Ok(Value::List(d))
        })
        .action("report.pos_hr.multi_employee_sales_report", "_get_report_values", |_, _, kw| {
            let mut data = match kw.get("data") { Some(Value::Map(m)) => m.clone(), _ => Row::new() };
            for k in ["session_ids", "employee_ids", "config_ids", "date_start", "date_stop"] { data.entry(k.into()).or_insert(Value::Null); }
            Ok(Value::Map(data))
        })
}
fn children_by(e: &Env, model: &str, dom: &Domain) -> Result<Vec<Row>> { orm::search(e, model, dom, None, None, 0)?.into_iter().map(|i| rec(e, model, i)).collect() }
