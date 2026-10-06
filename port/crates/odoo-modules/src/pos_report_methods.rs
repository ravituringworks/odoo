//! `report.point_of_sale.report_saledetails` (addons/point_of_sale/models/report_sale_details.py): the sales details / session
//! closing report data. The tax breakdown prices each tax on its own (no tax sequence / base-affecting taxes).
use crate::pos_methods::{epoch_secs, flag, has_model, id_list, many, pm_type};
use crate::util::*;
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, OdooError, Result, Row, Rules, Value};
use std::collections::BTreeMap;

const MODEL: &str = "report.point_of_sale.report_saledetails";

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468; let era = z.div_euclid(146097); let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1; let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}
fn fmt_secs(s: i64) -> String {
    let (days, rem) = (s.div_euclid(86400), s.rem_euclid(86400)); let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}", rem / 3600, rem % 3600 / 60, rem % 60)
}
/// `_get_date_start_and_date_stop`: defaults to today 00:00:00 .. 23:59:59 (UTC), and never ends before it starts.
fn date_range(start: Option<&str>, stop: Option<&str>) -> (String, String) {
    let s = start.filter(|d| !d.is_empty()).and_then(epoch_secs).unwrap_or_else(|| epoch_secs(&orm::today()).unwrap_or(0));
    let e = match stop.filter(|d| !d.is_empty()).and_then(epoch_secs) { Some(e) if e >= s => e, _ => s + 86400 - 1 };
    (fmt_secs(s), fmt_secs(e))
}
fn int_list(kw: &Row, k: &str) -> Vec<i64> { match kw.get(k) { Some(Value::List(l)) => l.iter().filter_map(|v| v.as_i64()).collect(), _ => vec![] } }

fn currency_rate_at(env: &Env, cur: i64, date: &str) -> f64 {
    if !has_model(env, "res.currency.rate") { return 1.0; }
    let d = if date.len() >= 10 { &date[..10] } else { date };
    orm::search(env, "res.currency.rate", &Domain::And(vec![term("currency_id", "=", cur), term("name", "<=", d)]), Some("name desc"), Some(1), 0).ok().and_then(|v| v.first().copied()).and_then(|r| rec(env, "res.currency.rate", r).ok()).map(|r| num(&r, "rate")).filter(|r| *r > 0.0).unwrap_or(1.0)
}
fn convert(env: &Env, amount: f64, from: i64, to: i64, date: &str) -> f64 { if from == to { amount } else { amount / currency_rate_at(env, from, date) * currency_rate_at(env, to, date) } }
fn round_to(x: f64, rounding: f64) -> f64 { let r = if rounding > 0.0 { rounding } else { 0.01 }; ((x / r).round() * r * 1e9).round() / 1e9 }
fn rounding_of(env: &Env, cur: Option<i64>) -> f64 { cur.and_then(|c| rec(env, "res.currency", c).ok()).map(|c| num(&c, "rounding")).filter(|r| *r > 0.0).unwrap_or(0.01) }

/// One product line group: (product, price_unit, discount) -> [qty, total paid, base amount].
#[derive(Default)]
struct Bucket { cats: Vec<String>, rows: BTreeMap<String, Vec<((i64, String, String), [f64; 3])>>, taxes: Vec<(i64, String, f64, f64)>, base: f64 }
fn tax_bucket(b: &mut Bucket, id: i64, name: &str) -> usize {
    if let Some(i) = b.taxes.iter().position(|t| t.0 == id) { return i; }
    b.taxes.push((id, name.to_string(), 0.0, 0.0)); b.taxes.len() - 1
}
/// `_get_products_and_taxes_dict`.
fn add_line(env: &Env, b: &mut Bucket, line: &Row, order_cur: Option<i64>) -> Result<()> {
    let pid = id_of(line, "product_id").unwrap_or(0);
    let tmpl = rec(env, "product.product", pid).ok().and_then(|p| id_of(&p, "product_tmpl_id")).and_then(|t| rec(env, "product.template", t).ok()).unwrap_or_default();
    let mut cats = many(env, "pos.category", &ids(&tmpl, "pos_categ_ids"))?; cats.sort_by(|a, c| num(a, "sequence").partial_cmp(&num(c, "sequence")).unwrap_or(std::cmp::Ordering::Equal).then(text(a, "name").cmp(&text(c, "name"))));
    let cat = cats.first().and_then(|c| text(c, "name")).unwrap_or_else(|| "Not Categorized".into());
    if !b.cats.contains(&cat) { b.cats.push(cat.clone()); }
    let key = (pid, format!("{}", num(line, "price_unit")), format!("{}", num(line, "discount")));
    let rows = b.rows.entry(cat).or_default();
    let idx = match rows.iter().position(|r| r.0 == key) { Some(i) => i, None => { rows.push((key, [0.0; 3])); rows.len() - 1 } };
    let rounding = rounding_of(env, order_cur);
    let total = round_to(num(line, "price_unit") * num(line, "qty") * (100.0 - num(line, "discount")) / 100.0, rounding);
    rows[idx].1[0] = ((rows[idx].1[0] + num(line, "qty")) * 1000.0).round() / 1000.0;
    rows[idx].1[1] += total; rows[idx].1[2] += num(line, "price_subtotal");
    // taxes after the fiscal position
    let tx = crate::pos_order_methods::map_tax(env, id_of(line, "order_id").and_then(|o| rec(env, "pos.order", o).ok()).and_then(|o| id_of(&o, "fiscal_position_id")), &ids(line, "tax_ids"))?;
    if !tx.is_empty() {
        for t in tx {
            let tr = rec(env, "account.tax", t)?; let loaded = crate::tax::load(env, &[t])?;
            let (base, amount, _) = crate::tax::compute(num(line, "qty"), num(line, "price_unit"), num(line, "discount"), &loaded);
            let i = tax_bucket(b, t, &text(&tr, "name").unwrap_or_default()); b.taxes[i].2 += amount; b.taxes[i].3 += base;
        }
    } else {
        let i = tax_bucket(b, 0, "No Taxes"); b.taxes[i].3 += num(line, "price_subtotal_incl");
    }
    b.base += num(line, "price_subtotal");
    Ok(())
}
fn taxes_info(b: &Bucket) -> Value { Value::Map(row(&[("tax_amount", b.taxes.iter().map(|t| t.2).sum::<f64>().into()), ("base_amount", b.base.into())])) }
fn tax_list(b: &Bucket) -> Value { Value::List(b.taxes.iter().map(|t| Value::Map(row(&[("name", t.1.as_str().into()), ("tax_amount", t.2.into()), ("base_amount", t.3.into())]))).collect()) }
/// Product categories (sorted by name) with their products (sorted by product name), totals per category and overall.
fn categories(env: &Env, b: &Bucket) -> Result<(Value, Value)> {
    let mut cats: Vec<(String, Vec<Row>)> = vec![];
    for c in &b.cats {
        let mut prods: Vec<Row> = vec![];
        for ((pid, price, disc), v) in &b.rows[c] {
            let p = rec(env, "product.product", *pid)?; let t = id_of(&p, "product_tmpl_id").and_then(|t| rec(env, "product.template", t).ok()).unwrap_or_default();
            let uom = id_of(&t, "uom_id").and_then(|u| rec(env, "uom.uom", u).ok()).and_then(|u| text(&u, "name"));
            prods.push(row(&[("product_id", (*pid).into()), ("product_name", text(&t, "name").or_else(|| text(&p, "name")).unwrap_or_default().into()), ("code", p.get("default_code").cloned().filter(|v| !v.is_null()).unwrap_or(Value::Bool(false))), ("quantity", v[0].into()), ("price_unit", price.parse::<f64>().unwrap_or(0.0).into()), ("discount", disc.parse::<f64>().unwrap_or(0.0).into()), ("uom", uom.map_or(Value::Bool(false), Value::Text)), ("total_paid", v[1].into()), ("base_amount", v[2].into())]));
        }
        prods.sort_by_key(|p| text(p, "product_name").unwrap_or_default());
        cats.push((c.clone(), prods));
    }
    cats.sort_by(|a, c| a.0.cmp(&c.0));
    let (mut all_qty, mut all_total) = (0.0, 0.0); let mut out = vec![];
    for (name, prods) in cats {
        let qty: f64 = prods.iter().map(|p| num(p, "quantity")).sum(); let total: f64 = prods.iter().map(|p| num(p, "base_amount")).sum();
        all_qty += qty; all_total += total;
        out.push(Value::Map(row(&[("name", name.as_str().into()), ("products", Value::List(prods.into_iter().map(Value::Map).collect())), ("total", ((total * 100.0).round() / 100.0).into()), ("qty", ((qty * 1000.0).round() / 1000.0).into())])));
    }
    Ok((Value::List(out), Value::Map(row(&[("total", all_total.into()), ("qty", all_qty.into())]))))
}

pub(crate) fn sale_details(env: &Env, kw: &Row) -> Result<Value> {
    let e = env.sudo();
    let session_ids = int_list(kw, "session_ids"); let config_ids = int_list(kw, "config_ids");
    let (mut date_start, mut date_stop) = (text(kw, "date_start"), text(kw, "date_stop"));
    if session_ids.is_empty() { let (a, b) = date_range(date_start.as_deref(), date_stop.as_deref()); date_start = Some(a); date_stop = Some(b); }
    // _get_domain
    let mut dom = vec![term("state", "in", Value::List(vec!["paid".into(), "invoiced".into(), "done".into()]))];
    if !session_ids.is_empty() { dom.push(term("session_id", "in", id_list(&session_ids))); } else {
        dom.push(term("date_order", ">=", date_start.clone().unwrap_or_default().as_str())); dom.push(term("date_order", "<=", date_stop.clone().unwrap_or_default().as_str()));
        if !config_ids.is_empty() { dom.push(term("config_id", "in", id_list(&config_ids))); }
    }
    // pos_hr: the report of a single cashier
    if let Some(emp) = kw.get("employee_id").and_then(|v| v.as_i64()) { if e.reg.field("pos.order", "employee_id").is_ok() { dom.push(term("employee_id", "=", emp)); } }
    let order_ids = orm::search(&e, "pos.order", &Domain::And(dom), Some("date_order desc, name desc, id desc"), None, 0)?;
    let orders: Vec<Row> = order_ids.iter().map(|o| rec(&e, "pos.order", *o)).collect::<Result<_>>()?;
    let cfg_cur = |c: i64| crate::pos_methods::config_currency_id(&e, c);
    let company_cur = find_one(&e, "res.company", Domain::True)?.and_then(|c| rec(&e, "res.company", c).ok()).and_then(|c| id_of(&c, "currency_id")).unwrap_or(0);
    let cur_ids: Vec<Option<i64>> = if !config_ids.is_empty() { config_ids.iter().map(|c| cfg_cur(*c)).collect() } else { session_ids.iter().filter_map(|s| rec(&e, "pos.session", *s).ok()).filter_map(|s| id_of(&s, "config_id")).map(|c| cfg_cur(c)).collect() };
    let user_cur = if !cur_ids.is_empty() && cur_ids.iter().all(|c| *c == cur_ids[0]) && cur_ids[0].is_some() { cur_ids[0].unwrap() } else { company_cur };
    let (mut total, mut sold, mut refunded) = (0.0, Bucket::default(), Bucket::default());
    for o in &orders {
        let pl_cur = id_of(o, "pricelist_id").and_then(|p| rec(&e, "product.pricelist", p).ok()).and_then(|p| id_of(&p, "currency_id"));
        let date = text(o, "date_order").unwrap_or_else(orm::now);
        total += match pl_cur { Some(pc) if pc != user_cur => convert(&e, num(o, "amount_total"), pc, user_cur, &date), _ => num(o, "amount_total") };
        let ocur = id_of(o, "session_id").and_then(|s| rec(&e, "pos.session", s).ok()).and_then(|s| id_of(&s, "config_id")).and_then(|c| cfg_cur(c));
        for l in children(&e, "pos.order.line", "order_id", o["id"].as_i64().unwrap())? { add_line(&e, if num(&l, "qty") >= 0.0 { &mut sold } else { &mut refunded }, &l, ocur)?; }
    }
    // payments per (method, session)
    let mut grouped: BTreeMap<(i64, i64), Row> = BTreeMap::new();
    for o in &orders {
        for p in children(&e, "pos.payment", "pos_order_id", o["id"].as_i64().unwrap())? {
            let Some(mid) = id_of(&p, "payment_method_id") else { continue };
            let sid = id_of(&p, "session_id").or_else(|| id_of(o, "session_id")).unwrap_or(0);
            let m = rec(&e, "pos.payment.method", mid)?;
            let entry = grouped.entry((mid, sid)).or_insert_with(|| row(&[("id", mid.into()), ("session", sid.into()), ("name", text(&m, "name").unwrap_or_default().into()), ("cash", flag(&m, "is_cash_count").into()), ("total", 0.0.into()), ("journal_id", id_of(&m, "journal_id").map_or(Value::Null, Value::Int))]));
            let t = num(entry, "total") + num(&p, "amount"); entry.insert("total".into(), t.into());
        }
    }
    let mut payments: Vec<Row> = grouped.into_values().collect();
    for p in payments.iter_mut() { p.insert("count".into(), false.into()); }
    // sessions and configs
    let (configs, sessions): (Vec<Row>, Vec<Row>) = if !config_ids.is_empty() {
        let cfgs = many(&e, "pos.config", &config_ids)?;
        let ss = if !session_ids.is_empty() { many(&e, "pos.session", &session_ids)? } else {
            let ids_ = orm::search(&e, "pos.session", &Domain::And(vec![term("config_id", "in", id_list(&config_ids)), term("start_at", ">=", date_start.clone().unwrap_or_default().as_str()), term("stop_at", "<=", date_stop.clone().unwrap_or_default().as_str())]), None, None, 0)?;
            many(&e, "pos.session", &ids_)?
        };
        (cfgs, ss)
    } else {
        let ss = many(&e, "pos.session", &session_ids)?;
        let cfgs = ss.iter().filter_map(|s| id_of(s, "config_id")).filter_map(|c| rec(&e, "pos.config", c).ok()).collect();
        (cfgs, ss)
    };
    // cash rounding: what was paid on top of the total
    let mut rounding_total = 0.0;
    for o in &orders {
        let ocur = id_of(o, "session_id").and_then(|s| rec(&e, "pos.session", s).ok()).and_then(|s| id_of(&s, "config_id")).and_then(|c| cfg_cur(c));
        let diff = num(o, "amount_paid") - num(o, "amount_total");
        rounding_total += match ocur { Some(c) if c != user_cur => convert(&e, diff, c, user_cur, &text(o, "date_order").unwrap_or_else(orm::now)), _ => diff };
    }
    let user_rounding = rounding_of(&e, Some(user_cur));
    rounding_total = round_to(rounding_total, user_rounding);
    // per session: how each payment method was counted at closing
    for s in &sessions {
        let sid = s["id"].as_i64().unwrap(); let cash_counted = num(s, "cash_register_balance_end_real"); let mut is_cash_method = false;
        let sname = text(s, "name").unwrap_or_default();
        for p in payments.iter_mut() {
            if p.get("session").and_then(|v| v.as_i64()) != Some(sid) { continue; }
            if !flag(p, "cash") {
                let mid = p["id"].as_i64().unwrap(); let pm = rec(&e, "pos.payment.method", mid)?;
                let reference = format!("Closing difference in {} ({})", text(p, "name").unwrap_or_default(), sname);
                let mut dom = vec![term("ref", "=", reference.as_str())]; if let Some(j) = id_of(&pm, "journal_id") { dom.push(term("journal_id", "=", j)); }
                if let Some(mv) = find_one(&e, "account.move", Domain::And(dom))? {
                    let lines = children(&e, "account.move.line", "move_id", mv)?;
                    let j = id_of(&pm, "journal_id").and_then(|j| rec(&e, "account.journal", j).ok()).unwrap_or_default();
                    let is_loss = id_of(&j, "loss_account_id").map_or(false, |a| lines.iter().any(|l| id_of(l, "account_id") == Some(a)));
                    let is_profit = id_of(&j, "profit_account_id").map_or(false, |a| lines.iter().any(|l| id_of(l, "account_id") == Some(a)));
                    let amount_total = lines.iter().map(|l| num(l, "debit")).sum::<f64>();
                    let diff = if is_loss { -amount_total } else { amount_total };
                    p.insert("final_count".into(), p["total"].clone()); p.insert("money_difference".into(), diff.into()); p.insert("money_counted".into(), (num(p, "total") + diff).into());
                    let moves = if is_profit { vec![("Difference observed during the counting (Profit)", diff)] } else if is_loss { vec![("Difference observed during the counting (Loss)", diff)] } else { vec![] };
                    p.insert("cash_moves".into(), Value::List(moves.into_iter().map(|(n, a)| Value::Map(row(&[("name", n.into()), ("amount", a.into())]))).collect()));
                    p.insert("count".into(), true.into());
                }
            } else {
                is_cash_method = true;
                let final_count = num(p, "total") + num(s, "cash_register_balance_start") + num(s, "cash_real_transaction");
                p.insert("final_count".into(), final_count.into()); p.insert("money_counted".into(), cash_counted.into()); p.insert("money_difference".into(), (cash_counted - final_count).into());
                let mut list: Vec<Value> = vec![]; let (mut cin, mut cout) = (0, 0);
                if num(s, "cash_register_balance_start") > 0.0 { list.push(Value::Map(row(&[("name", "Cash Opening".into()), ("amount", num(s, "cash_register_balance_start").into())]))); }
                for l in many(&e, "account.bank.statement.line", &ids(s, "statement_line_ids"))? {
                    let amt = num(&l, "amount");
                    let name = if amt > 0.0 { cin += 1; format!("Cash in {cin}") } else { cout += 1; format!("Cash out {cout}") };
                    let lj = id_of(&l, "move_id").and_then(|m| rec(&e, "account.move", m).ok()).and_then(|m| id_of(&m, "journal_id"));
                    if lj == id_of(p, "journal_id") { list.push(Value::Map(row(&[("name", text(&l, "payment_ref").filter(|r| !r.is_empty()).unwrap_or(name).into()), ("amount", amt.into())]))); }
                }
                p.insert("cash_moves".into(), Value::List(list)); p.insert("count".into(), true.into());
            }
        }
        if !is_cash_method {
            // no cash payment in the session: describe the drawer from the previous closed session
            let prev = orm::search(&e, "pos.session", &Domain::And(vec![term("id", "<", sid), term("state", "=", "closed"), term("config_id", "=", id_of(s, "config_id").unwrap_or(0))]), None, Some(1), 0)?.first().and_then(|i| rec(&e, "pos.session", *i).ok()).unwrap_or_default();
            let final_count = num(&prev, "cash_register_balance_end_real") + num(s, "cash_real_transaction");
            let diff = num(s, "cash_register_balance_end_real") - final_count;
            let mut moves = many(&e, "account.bank.statement.line", &ids(s, "statement_line_ids"))?; moves.sort_by_key(|l| text(l, "date"));
            let mut list: Vec<Value> = vec![];
            if num(&prev, "cash_register_balance_end_real") > 0.0 { list.push(Value::Map(row(&[("name", "Cash Opening".into()), ("amount", num(&prev, "cash_register_balance_end_real").into())]))); }
            if round_to(diff, user_rounding) != 0.0 && !moves.is_empty() { moves.pop(); }
            for m in moves { list.push(Value::Map(row(&[("name", m.get("payment_ref").cloned().unwrap_or(Value::Null)), ("amount", m.get("amount").cloned().unwrap_or(0.0.into()))]))); }
            payments.insert(0, row(&[("name", format!("Cash {sname}").into()), ("total", 0.0.into()), ("final_count", final_count.into()), ("money_counted", s.get("cash_register_balance_end_real").cloned().filter(|v| !v.is_null()).unwrap_or(0.0.into())), ("money_difference", diff.into()), ("cash_moves", Value::List(list)), ("count", true.into()), ("session", sid.into())]));
        }
    }
    let (products, products_info) = categories(&e, &sold)?; let (refund_products, refund_info) = categories(&e, &refunded)?;
    let cur = rec(&e, "res.currency", user_cur).unwrap_or_default();
    let decimals = { let r = rounding_of(&e, Some(user_cur)); (-(r.log10())).round().max(0.0) as i64 };
    let currency = Value::Map(row(&[("symbol", cur.get("symbol").cloned().unwrap_or(Value::Null)), ("position", (text(&cur, "position").as_deref() == Some("after")).into()), ("total_paid", round_to(total, user_rounding).into()), ("precision", cur.get("decimal_places").cloned().filter(|v| !v.is_null()).unwrap_or(decimals.into()))]));
    let (state, session_name, mut d_start, mut d_stop, opening, closing) = if sessions.len() == 1 {
        let s = &sessions[0]; (text(s, "state").unwrap_or_default(), text(s, "name").map_or(Value::Bool(false), Value::Text), s.get("start_at").cloned().unwrap_or(Value::Null), s.get("stop_at").cloned().unwrap_or(Value::Null), s.get("opening_notes").cloned().unwrap_or(Value::Bool(false)), s.get("closing_notes").cloned().unwrap_or(Value::Bool(false)))
    } else { ("multiple".to_string(), Value::Bool(false), date_start.clone().map_or(Value::Bool(false), Value::Text), date_stop.clone().map_or(Value::Bool(false), Value::Text), Value::Bool(false), Value::Bool(false)) };
    if sessions.len() != 1 { d_start = date_start.clone().map_or(Value::Bool(false), Value::Text); d_stop = date_stop.clone().map_or(Value::Bool(false), Value::Text); }
    // discounts
    let (mut discount_number, mut discount_amount) = (0, 0.0);
    for o in &orders {
        let mut any = false;
        for l in children(&e, "pos.order.line", "order_id", o["id"].as_i64().unwrap())? { if num(&l, "discount") > 0.0 { any = true; discount_amount += crate::pos_order_methods::line_discount_amount(&e, &l)?; } }
        if any { discount_number += 1; }
    }
    // invoices and payments per session
    let (mut invoice_list, mut invoice_total, mut total_payments) = (vec![], 0.0, 0.0);
    for s in &sessions {
        let sid = s["id"].as_i64().unwrap();
        invoice_list.push(Value::Map(row(&[("name", s.get("name").cloned().unwrap_or(Value::Null)), ("invoices", orm::call(&e, "pos.session", "_get_invoice_total_list", &[sid], &Row::new())?)])));
        invoice_total += orm::call(&e, "pos.session", "_get_total_invoice", &[sid], &Row::new())?.as_f64().unwrap_or(0.0);
        total_payments += crate::pos_session_methods::captured_payments(&e, sid)?.iter().map(|p| num(p, "amount")).sum::<f64>();
    }
    let mut per_method: Vec<(i64, String, f64)> = vec![];
    for p in payments.iter_mut() {
        let Some(mid) = p.get("id").and_then(|v| v.as_i64()) else { continue };
        let mname = text(&rec(&e, "pos.payment.method", mid)?, "name").unwrap_or_default();
        let sname = rec(&e, "pos.session", p.get("session").and_then(|v| v.as_i64()).unwrap_or(0)).ok().and_then(|s| text(&s, "name")).unwrap_or_default();
        p.insert("name".into(), format!("{mname} {sname}").into());
        match per_method.iter_mut().find(|x| x.0 == mid) { Some(x) => x.2 += num(p, "total"), None => per_method.push((mid, mname, num(p, "total"))) }
    }
    let _ = pm_type; let company_name = find_one(&e, "res.company", Domain::True)?.and_then(|c| rec(&e, "res.company", c).ok()).and_then(|c| text(&c, "name")).unwrap_or_default();
    Ok(Value::Map(row(&[
        ("opening_note", if sessions.len() == 1 { opening } else { Value::Bool(false) }), ("closing_note", if sessions.len() == 1 { closing } else { Value::Bool(false) }),
        ("state", state.as_str().into()), ("currency", currency), ("nbr_orders", (orders.len() as i64).into()), ("date_start", d_start), ("date_stop", d_stop), ("session_name", session_name),
        ("config_names", Value::List(configs.iter().filter_map(|c| text(c, "name")).map(Value::Text).collect())),
        ("payments", Value::List(payments.into_iter().map(Value::Map).collect())), ("company_name", company_name.as_str().into()),
        ("taxes", tax_list(&sold)), ("taxes_info", taxes_info(&sold)), ("products", products), ("products_info", products_info),
        ("refund_taxes", tax_list(&refunded)), ("refund_taxes_info", taxes_info(&refunded)), ("refund_info", refund_info), ("refund_products", refund_products),
        ("discount_number", discount_number.into()), ("discount_amount", discount_amount.into()), ("invoiceList", Value::List(invoice_list)), ("invoiceTotal", invoice_total.into()), ("total_paid", total_payments.into()),
        ("payments_per_method", Value::List(per_method.into_iter().map(|(_, n, t)| Value::Map(row(&[("name", n.as_str().into()), ("total", t.into())]))).collect())),
        ("show_payment_per_method", session_ids.is_empty().into()), ("cash_rounding_total", rounding_total.into()),
    ])))
}

pub fn rules() -> Rules {
    Rules::default()
        .action(MODEL, "get_sale_details", |env, _, kw| sale_details(env, kw))
        .action(MODEL, "_get_report_values", |env, ids_, kw| {
            // called from the POS, the docids are session ids unless a period or configs were given
            let data = match kw.get("data") { Some(Value::Map(m)) => m.clone(), _ => Row::new() };
            let no_filter = !data.get("config_ids").map_or(false, |v| v.truthy()) && !data.get("date_start").map_or(false, |v| v.truthy()) && !data.get("date_stop").map_or(false, |v| v.truthy());
            let sessions = if data.get("session_ids").map_or(false, |v| v.truthy()) { data.get("session_ids").cloned().unwrap() } else if no_filter { id_list(ids_) } else { Value::Null };
            let mut args = row(&[("session_ids", sessions.clone())]);
            for k in ["config_ids", "date_start", "date_stop"] { if let Some(v) = data.get(k) { args.insert(k.into(), v.clone()); } }
            let mut out = data; out.insert("session_ids".into(), sessions);
            if let Value::Map(m) = sale_details(env, &args)? { out.extend(m); }
            Ok(Value::Map(out))
        })
        .action(MODEL, "_get_date_start_and_date_stop", |_, _, kw| { let (a, b) = date_range(text(kw, "date_start").as_deref(), text(kw, "date_stop").as_deref()); Ok(Value::List(vec![Value::Text(a), Value::Text(b)])) })
        .action(MODEL, "_get_domain", |_, _, kw| {
            let sessions = int_list(kw, "session_ids"); let t = |f: &str, op: &str, v: Value| Value::List(vec![f.into(), op.into(), v]);
            let mut d = vec![t("state", "in", Value::List(vec!["paid".into(), "invoiced".into(), "done".into()]))];
            if !sessions.is_empty() { d.push(t("session_id", "in", id_list(&sessions))); } else {
                let (a, b) = date_range(text(kw, "date_start").as_deref(), text(kw, "date_stop").as_deref());
                d.push(t("date_order", ">=", a.into())); d.push(t("date_order", "<=", b.into()));
                let cfgs = int_list(kw, "config_ids"); if !cfgs.is_empty() { d.push(t("config_id", "in", id_list(&cfgs))); }
            }
            Ok(Value::List(d))
        })
        .action(MODEL, "_get_product_total_amount", |env, ids_, _| {
            let l = rec(env, "pos.order.line", *ids_.first().ok_or_else(|| OdooError::User("Select a line".into()))?)?;
            let cur = id_of(&l, "order_id").and_then(|o| rec(env, "pos.order", o).ok()).and_then(|o| id_of(&o, "session_id")).and_then(|s| rec(env, "pos.session", s).ok()).and_then(|s| id_of(&s, "config_id")).and_then(|c| crate::pos_methods::config_currency_id(env, c));
            Ok(round_to(num(&l, "price_unit") * num(&l, "qty") * (100.0 - num(&l, "discount")) / 100.0, rounding_of(env, cur)).into())
        })
        .action(MODEL, "_get_taxes_info", |_, _, kw| {
            let t = match kw.get("taxes") { Some(Value::Map(m)) => m.clone(), _ => Row::new() };
            let amount: f64 = match t.get("taxes") { Some(Value::Map(m)) => m.values().map(|v| match v { Value::Map(x) => num(x, "tax_amount"), _ => 0.0 }).sum(), Some(Value::List(l)) => l.iter().map(|v| match v { Value::Map(x) => num(x, "tax_amount"), _ => 0.0 }).sum(), _ => 0.0 };
            Ok(Value::Map(row(&[("tax_amount", amount.into()), ("base_amount", num(&t, "base_amount").into())])))
        })
}
