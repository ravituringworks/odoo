//! point_of_sale: payment-method configuration helpers.
use odoo_core::{Rules, Value};

/// `hide_use_payment_terminal`: Odoo hides the terminal selector when no payment interface module (pos_adyen, pos_stripe, ...)
/// is installed, i.e. when the field's selection is empty.
fn hide_terminal(env: &odoo_core::Env, _r: &odoo_core::Row) -> odoo_core::Result<Value> {
    Ok(Value::Bool(env.reg.field("pos.payment.method", "use_payment_terminal").map_or(true, |f| f.selection_values().is_empty())))
}

pub fn rules() -> Rules { Rules::default().compute("pos.payment.method", "hide_use_payment_terminal", hide_terminal).merge(runtime_rules()).merge(crate::pos_restaurant::rules()).merge(crate::pos_kiosk::rules()).merge(crate::pos_hw::rules()) }

// ---------------------------------------------------------------------------------------------------------------------
// Point of Sale runtime: terminal bootstrap, order capture (idempotent by uuid → offline sync), refunds, session close.
// ---------------------------------------------------------------------------------------------------------------------
use crate::{pos_loyalty, pos_post, tax, util::*};
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, OdooError, Result, Row};
use std::collections::BTreeMap;

fn set6(ids: &[i64]) -> Value { Value::List(vec![Value::List(vec![6.into(), 0.into(), Value::List(ids.iter().map(|i| Value::Int(*i)).collect())])]) }
fn map(pairs: Vec<(&str, Value)>) -> Value { Value::Map(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect::<BTreeMap<_, _>>()) }
fn list(rows: Vec<Row>) -> Value { Value::List(rows.into_iter().map(Value::Map).collect()) }
fn kw_list<'a>(kw: &'a Row, k: &str) -> &'a [Value] { match kw.get(k) { Some(Value::List(l)) => l, _ => &[] } }
fn as_map(v: &Value) -> Row { if let Value::Map(m) = v { m.clone() } else { Row::new() } }
fn pick(r: &Row, keys: &[&str]) -> Row { keys.iter().filter_map(|k| r.get(*k).map(|v| (k.to_string(), v.clone()))).collect() }
fn flag(r: &Row, k: &str) -> bool { r.get(k).map_or(false, |v| v.truthy()) }

/// First-run defaults so a fresh install is sellable: Cash/Card/Customer-account methods and POS-enabled products.
fn ensure_defaults(env: &Env, config: i64) -> Result<()> {
    let e = env.sudo();
    let cfg = rec(&e, "pos.config", config)?;
    if ids(&cfg, "payment_method_ids").is_empty() {
        let mut have = vec![];
        for (name, cash, split) in [("Cash", true, false), ("Card", false, false), ("Customer Account", false, true)] {
            let mut v = row(&[("name", name.into()), ("is_cash_count", cash.into()), ("split_transactions", split.into())]);
            v.retain(|k, _| e.reg.field("pos.payment.method", k).is_ok());
            have.push(find_or_create(&e, "pos.payment.method", term("name", "=", name), v)?);
        }
        orm::write(&e, "pos.config", &[config], row(&[("payment_method_ids", set6(&have))]))?;
    }
    if find_one(&e, "product.template", term("available_in_pos", "=", true))?.is_none() {
        let all = orm::search(&e, "product.template", &term("sale_ok", "=", true), Some("id"), Some(200), 0)?;
        if !all.is_empty() { orm::write(&e, "product.template", &all, row(&[("available_in_pos", true.into())]))?; }
    }
    Ok(())
}

fn terminal_data(env: &Env, config: i64, session: &Row) -> Result<Value> {
    let e = env.sudo();
    let cfg = rec(&e, "pos.config", config)?;
    let methods = ids(&cfg, "payment_method_ids").into_iter().map(|i| rec(&e, "pos.payment.method", i)).collect::<Result<Vec<_>>>()?
        .into_iter().map(|m| { let mut o = pick(&m, &["id", "name", "is_cash_count", "split_transactions", "use_payment_terminal"]); o.insert("kind".into(), (if flag(&m, "is_cash_count") { "cash" } else if flag(&m, "split_transactions") { "account" } else { "bank" }).into()); o }).collect();
    let limit = ids(&cfg, "iface_available_categ_ids");
    let cats = if env.reg.model("pos.category").is_ok() { orm::search_read(&e, "pos.category", &Domain::True, &["id".into(), "name".into(), "parent_id".into(), "sequence".into()], Some("sequence, id"), None, 0)? } else { vec![] };
    let mut tdom = term("available_in_pos", "=", true);
    if flag(&cfg, "limit_categories") && !limit.is_empty() { tdom = Domain::And(vec![tdom, term("pos_categ_ids", "in", Value::List(limit.iter().map(|i| Value::Int(*i)).collect()))]); }
    let mut tfields: Vec<String> = ["id", "name", "list_price", "default_code", "taxes_id", "pos_categ_ids", "to_weight", "type", "categ_id", "tracking", "combo_ids"].iter().filter(|f| e.reg.field("product.template", f).is_ok()).map(|f| f.to_string()).collect();
    if !tfields.contains(&"id".to_string()) { tfields.push("id".into()); }
    let tmpl = orm::search_read(&e, "product.template", &tdom, &tfields, Some("name"), Some(500), 0)?;
    let tids: Vec<i64> = tmpl.iter().filter_map(|t| t["id"].as_i64()).collect();
    let prods = if tids.is_empty() { vec![] } else { orm::search_read(&e, "product.product", &term("product_tmpl_id", "in", Value::List(tids.iter().map(|i| Value::Int(*i)).collect())), &["id".into(), "product_tmpl_id".into(), "barcode".into()], None, None, 0)? };
    let by_tmpl: BTreeMap<i64, &Row> = prods.iter().filter_map(|p| id_of(p, "product_tmpl_id").map(|t| (t, p))).collect();
    let mut taxes_used = std::collections::BTreeSet::new();
    let products: Vec<Row> = tmpl.iter().filter_map(|t| {
        let p = by_tmpl.get(&t["id"].as_i64()?)?;
        let tx = ids(t, "taxes_id"); taxes_used.extend(tx.iter().copied());
        Some(map_row(vec![("id", p["id"].clone()), ("name", t["name"].clone()), ("price", t["list_price"].clone()), ("code", t.get("default_code").cloned().unwrap_or(Value::Null)), ("barcode", p.get("barcode").cloned().unwrap_or(Value::Null)), ("tax_ids", Value::List(tx.into_iter().map(Value::Int).collect())), ("category_ids", t.get("pos_categ_ids").cloned().unwrap_or(Value::List(vec![]))), ("to_weight", t.get("to_weight").cloned().unwrap_or(false.into())), ("categ_chain", Value::List(categ_chain(&e, id_of(t, "categ_id")).into_iter().map(Value::Int).collect())),
            ("type", t.get("type").cloned().unwrap_or(Value::Null)), ("tracking", t.get("tracking").cloned().unwrap_or(Value::Text("none".into()))), ("combo", combo_groups(&e, &ids(t, "combo_ids")))]))
    }).collect();
    let taxes = taxes_used.into_iter().map(|i| { let t = rec(&e, "account.tax", i)?; Ok(pick(&t, &["id", "name", "amount", "amount_type", "price_include"])) }).collect::<Result<Vec<_>>>()?;
    let pl_ids = { let mut v = ids(&cfg, "available_pricelist_ids"); if let Some(p) = id_of(&cfg, "pricelist_id") { if !v.contains(&p) { v.push(p); } } v };
    let pricelists = pl_ids.iter().map(|i| {
        let p = rec(&e, "product.pricelist", *i)?;
        let items = if e.reg.model("product.pricelist.item").is_ok() { children(&e, "product.pricelist.item", "pricelist_id", *i)?.into_iter().map(|it| pick(&it, &["applied_on", "compute_price", "fixed_price", "percent_price", "min_quantity", "product_tmpl_id", "product_id", "categ_id"])).collect() } else { vec![] };
        Ok(map_row(vec![("id", (*i).into()), ("name", p.get("name").cloned().unwrap_or(Value::Null)), ("items", list(items))]))
    }).collect::<Result<Vec<_>>>()?;
    let user = rec(&e, "res.users", env.uid.max(1)).ok();
    let cashier = user.as_ref().and_then(|u| id_of(u, "partner_id")).and_then(|p| rec(&e, "res.partner", p).ok()).and_then(|p| text(&p, "name")).unwrap_or_else(|| "Administrator".into());
    let currency = id_of(&cfg, "company_id").and_then(|c| rec(&e, "res.company", c).ok()).and_then(|c| id_of(&c, "currency_id")).and_then(|c| rec(&e, "res.currency", c).ok()).map(|c| pick(&c, &["name", "symbol", "rounding"])).unwrap_or_default();
    Ok(map(vec![
        ("session", Value::Map(pick(session, &["id", "name", "state", "start_at", "cash_register_balance_start"]))),
        ("config", Value::Map(pick(&cfg, &["id", "name", "iface_tax_included", "manual_discount", "use_pricelist", "pricelist_id", "receipt_header", "receipt_footer", "iface_print_auto", "cash_rounding", "only_round_cash_method", "restrict_price_control", "ship_later", "takeaway", "iface_tipproduct", "tip_product_id", "customer_display_type", "iface_electronic_scale", "set_maximum_difference", "module_pos_restaurant"]))),
        ("payment_methods", list(methods)), ("categories", list(cats)), ("products", list(products)), ("taxes", list(taxes)), ("pricelists", list(pricelists)),
        ("floors", crate::pos_restaurant::plan(&e, &cfg)?), ("printers", printers_data(&e, &cfg)), ("loyalty", loyalty_data(&e, config)?), ("employees", employees_data(&e, &cfg)?), ("cashier_lock", Value::Bool(uses_employees(&e) && (!ids(&cfg, "basic_employee_ids").is_empty() || !ids(&cfg, "advanced_employee_ids").is_empty()))), ("cashier", cashier.into()), ("currency", Value::Map(currency)), ("invoicing", Value::Bool(env.reg.model("account.move").is_ok())),
    ]))
}
fn map_row(p: Vec<(&str, Value)>) -> Row { p.into_iter().map(|(k, v)| (k.to_string(), v)).collect() }

fn order_view(env: &Env, id: i64) -> Result<Value> {
    let o = rec(env, "pos.order", id)?;
    let lines = children(env, "pos.order.line", "order_id", id)?.into_iter().map(|l| pick(&l, &["id", "product_id", "full_product_name", "qty", "price_unit", "discount", "price_subtotal", "price_subtotal_incl", "refunded_orderline_id", "customer_note"])).collect();
    let pays = children(env, "pos.payment", "pos_order_id", id)?.into_iter().map(|p| { let mut r = pick(&p, &["amount", "payment_method_id", "is_change"]); if let Some(m) = id_of(&p, "payment_method_id").and_then(|m| rec(env, "pos.payment.method", m).ok()) { r.insert("method".into(), m.get("name").cloned().unwrap_or(Value::Null)); } r }).collect();
    let mut head = pick(&o, &["id", "name", "pos_reference", "state", "date_order", "amount_total", "amount_tax", "amount_paid", "amount_return", "partner_id", "to_invoice", "account_move", "session_id", "uuid", "general_note"]);
    head.insert("lines".into(), list(lines)); head.insert("payments".into(), list(pays));
    Ok(Value::Map(head))
}

fn session_summary(env: &Env, sid: i64) -> Result<Row> {
    let s = rec(env, "pos.session", sid)?;
    let orders = children(env, "pos.order", "session_id", sid)?;
    let mut by: BTreeMap<String, (f64, i64)> = BTreeMap::new();
    let mut cash = 0.0;
    for o in &orders { for p in children(env, "pos.payment", "pos_order_id", o["id"].as_i64().unwrap())? {
        let m = id_of(&p, "payment_method_id").map(|m| rec(env, "pos.payment.method", m)).transpose()?;
        let name = m.as_ref().and_then(|m| text(m, "name")).unwrap_or_default();
        if m.as_ref().map_or(false, |m| flag(m, "is_cash_count")) { cash += num(&p, "amount"); }
        let e = by.entry(name).or_insert((0.0, 0)); e.0 = r2(e.0 + num(&p, "amount")); e.1 += 1;
    } }
    let sales: f64 = orders.iter().map(|o| num(o, "amount_total")).sum();
    let refunds: f64 = orders.iter().filter(|o| num(o, "amount_total") < 0.0).map(|o| num(o, "amount_total")).sum();
    Ok(map_row(vec![("session_id", sid.into()), ("name", s.get("name").cloned().unwrap_or(Value::Null)), ("state", s.get("state").cloned().unwrap_or(Value::Null)),
        ("orders", (orders.len() as i64).into()), ("total", r2(sales).into()), ("refunds", r2(refunds).into()), ("opening_cash", num(&s, "cash_register_balance_start").into()),
        ("expected_cash", r2(num(&s, "cash_register_balance_start") + cash).into()),
        ("payments", Value::List(by.into_iter().map(|(n, (a, c))| map(vec![("name", n.into()), ("amount", a.into()), ("count", c.into())])).collect()))]))
}

pub fn runtime_rules() -> Rules {
    Rules::default()
        .before_create("pos.session", |env, mut v| {
            if text(&v, "name").map_or(true, |n| n.is_empty() || n == "/") { v.insert("name".into(), next_seq(env, "pos.session", "POS/", 5)?.into()); }
            v.entry("state".into()).or_insert("opened".into()); v.entry("start_at".into()).or_insert(orm::now().into());
            Ok(v)
        })
        // open_ui: find/create the config's open session and return everything the terminal needs.
        .action("pos.config", "open_ui", |env, ids, kw| {
            let e = env.sudo(); let cfg = *ids.first().ok_or_else(|| OdooError::User("Select a point of sale".into()))?;
            ensure_defaults(&e, cfg)?;
            let sess = match find_one(&e, "pos.session", Domain::And(vec![term("config_id", "=", cfg), term("state", "in", Value::List(vec!["opening_control".into(), "opened".into()]))]))? {
                Some(s) => s,
                None => orm::create(&e, "pos.session", row(&[("config_id", cfg.into()), ("state", "opened".into()), ("user_id", env.uid.into()), ("cash_register_balance_start", num(kw, "opening_cash").into())]))?,
            };
            terminal_data(env, cfg, &rec(&e, "pos.session", sess)?)
        })
        .action("pos.order", "create_from_ui", |env, _, kw| {
            let e = env.sudo();
            let sid = kw.get("session_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("Missing POS session".into()))?;
            let sess = rec(&e, "pos.session", sid)?;
            if text(&sess, "state").as_deref() != Some("opened") { return Err(OdooError::User("This POS session is closed. Open a new session to keep selling.".into())); }
            let uuid = text(kw, "uuid").unwrap_or_default();
            let mut draft: Option<i64> = None;
            if !uuid.is_empty() { if let Some(o) = find_one(&e, "pos.order", term("uuid", "=", uuid.as_str()))? { if text(&rec(&e, "pos.order", o)?, "state").as_deref() == Some("draft") { draft = Some(o); } else { return order_view(&e, o); } } }
            let cfg = rec(&e, "pos.config", id_of(&sess, "config_id").unwrap_or(0))?;
            let (emp, role) = employee_for(&e, &cfg, kw)?;
            let lines = kw_list(kw, "lines"); if lines.is_empty() { return Err(OdooError::User("Add at least one product before paying".into())); }
            let refund_of = kw.get("refund_of").and_then(|v| v.as_i64());
            let is_refund = lines.iter().any(|l| as_map(l).get("refunded_orderline_id").map_or(false, |v| v.truthy()));
            let Built { mut prepared, items, mut untaxed, tax: mut tax_sum, mut total } = build_lines(&e, &cfg, role, lines, true)?;
            if prepared.is_empty() { return Err(OdooError::User("Add at least one product before paying".into())); }
            let partner = kw.get("partner_id").and_then(|v| v.as_i64());
            let loyalty = pos_loyalty::process(&e, id_of(&sess, "config_id").unwrap_or(0), partner, kw, &items, is_refund)?;
            for l in &loyalty.lines { untaxed += num(l, "price_subtotal"); total += num(l, "price_subtotal_incl"); tax_sum += num(l, "price_subtotal_incl") - num(l, "price_subtotal"); prepared.push(l.clone()); }
            let (untaxed, tax_sum, total) = (r2(untaxed), r2(tax_sum), r2(total));
            if total < 0.0 && !is_refund { return Err(OdooError::User("Rewards cannot make the order total negative".into())); }
            let (mut paid, mut cash_paid) = (0.0, 0.0); let mut pays: Vec<Row> = vec![]; let mut consume: Vec<i64> = vec![];
            for p in kw_list(kw, "payments") {
                let p = as_map(p); let mid = p.get("payment_method_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("Payment without method".into()))?;
                if !ids(&cfg, "payment_method_ids").contains(&mid) { return Err(OdooError::User("That payment method is not enabled for this point of sale".into())); }
                let m = rec(&e, "pos.payment.method", mid)?; let amt = r2(num(&p, "amount"));
                if flag(&m, "split_transactions") && partner.is_none() { return Err(OdooError::User("Select a customer to pay on account".into())); }
                if flag(&m, "is_cash_count") { cash_paid += amt; }
                let mut pv = row(&[("payment_method_id", mid.into()), ("amount", amt.into())]);
                // card terminals: the money must really have been taken — the app's ledger holds an unused approval for this exact method and amount
                if m.get("use_payment_terminal").and_then(|v| v.as_str()).map_or(false, |x| !x.is_empty()) && amt != 0.0 {
                    let tx = text(&p, "transaction_id").unwrap_or_default();
                    let entry = if tx.is_empty() { None } else { find_one(&e, "ir.config_parameter", term("key", "=", format!("pos.terminal.{tx}").as_str()))? };
                    let entry = entry.ok_or_else(|| OdooError::User("The card payment was not approved by the terminal".into()))?;
                    let v = text(&rec(&e, "ir.config_parameter", entry)?, "value").unwrap_or_default(); let f: Vec<&str> = v.split('|').collect();
                    if f.len() < 5 || f[0].parse::<i64>().ok() != Some(mid) || f[1].parse::<i64>().ok() != Some((amt * 100.0).round() as i64) { return Err(OdooError::User("The terminal approval does not match this payment".into())); }
                    consume.push(entry);
                    for (k, val) in [("transaction_id", tx.as_str()), ("card_brand", f[2]), ("card_no", f[3]), ("payment_status", "done"), ("payment_ref_no", f[4])] { pv.insert(k.into(), val.into()); }
                }
                paid += amt; pays.push(pv);
            }
            let paid = r2(paid); let cash_method = pays.iter().find(|p| p.get("payment_method_id").and_then(|v| v.as_i64()).map_or(false, |m| rec(&e, "pos.payment.method", m).map_or(false, |m| flag(&m, "is_cash_count")))).and_then(|p| p.get("payment_method_id").cloned());
            let change = if total >= 0.0 { r2((paid - total).max(0.0)) } else { 0.0 };
            if total >= 0.0 && paid + 0.005 < total { return Err(OdooError::User(format!("The order is not fully paid: {:.2} of {:.2}", paid, total))); }
            if total < 0.0 && (paid - total).abs() > 0.005 { return Err(OdooError::User("A refund must return exactly the refunded amount".into())); }
            if change > 0.0 && change > r2(cash_paid) + 0.005 { return Err(OdooError::User("Only cash payments can give change back".into())); }
            let to_invoice = flag(kw, "to_invoice");
            if to_invoice && partner.is_none() { return Err(OdooError::User("Select a customer to issue an invoice".into())); }
            let name = next_seq(&e, "pos.order", "Order ", 5)?;
            let mut ov = row(&[("name", name.clone().into()), ("pos_reference", name.clone().into()), ("session_id", sid.into()), ("config_id", id_of(&sess, "config_id").unwrap_or(0).into()), ("state", (if to_invoice { "invoiced" } else { "paid" }).into()), ("date_order", orm::now().into()), ("amount_total", total.into()), ("amount_tax", tax_sum.into()), ("amount_paid", r2(paid - change).into()), ("amount_return", change.into()), ("to_invoice", to_invoice.into()), ("uuid", uuid.clone().into()), ("user_id", env.uid.into()), ("general_note", text(kw, "note").unwrap_or_default().into())]);
            if let Some(c) = id_of(&sess, "company_id").or_else(|| id_of(&cfg, "company_id")) { ov.insert("company_id".into(), c.into()); }
            if let Some(p) = partner { ov.insert("partner_id".into(), p.into()); }
            if let Some(p) = kw.get("pricelist_id").and_then(|v| v.as_i64()) { ov.insert("pricelist_id".into(), p.into()); }
            if let Some((eid, ename)) = &emp { ov.insert("employee_id".into(), (*eid).into()); ov.insert("cashier".into(), ename.clone().into()); }
            ov.retain(|k, _| e.reg.field("pos.order", k).is_ok());
            for k in ["table_id", "customer_count", "takeaway"] { if let Some(v) = kw.get(k) { if e.reg.field("pos.order", k).is_ok() { ov.insert(k.into(), v.clone()); } } }
            let oid = match draft {
                Some(d) => { for l in children(&e, "pos.order.line", "order_id", d)? { orm::unlink(&e, "pos.order.line", &[l["id"].as_i64().unwrap()])?; } orm::write(&e, "pos.order", &[d], ov)?; d }
                None => orm::create(&e, "pos.order", ov)?,
            };
            insert_lines(&e, oid, prepared)?;
            if change > 0.0 { if let Some(m) = cash_method { pays.push(row(&[("payment_method_id", m), ("amount", (-change).into()), ("is_change", true.into())])); } }
            for mut p in pays { p.insert("pos_order_id".into(), oid.into()); p.insert("session_id".into(), sid.into()); p.insert("name".into(), name.clone().into()); p.insert("payment_date".into(), orm::now().into()); p.retain(|k, _| e.reg.field("pos.payment", k).is_ok()); orm::create(&e, "pos.payment", p)?; }
            if to_invoice && e.reg.model("account.move").is_ok() {
                let mid = orm::create(&e, "account.move", row(&[("move_type", (if total < 0.0 { "out_refund" } else { "out_invoice" }).into()), ("partner_id", partner.unwrap().into()), ("invoice_origin", name.clone().into())]))?;
                for l in children(&e, "pos.order.line", "order_id", oid)? {
                    let mut v = row(&[("move_id", mid.into()), ("name", text(&l, "name").unwrap_or_default().into()), ("quantity", num(&l, "qty").abs().into()), ("price_unit", num(&l, "price_unit").into()), ("discount", num(&l, "discount").into()), ("display_type", "product".into())]);
                    if let Some(p) = id_of(&l, "product_id") { v.insert("product_id".into(), p.into()); }
                    let tx = ids(&l, "tax_ids"); if !tx.is_empty() { v.insert("tax_ids".into(), set6(&tx)); }
                    orm::create(&e, "account.move.line", v)?;
                }
                orm::recompute_ids(&e, "account.move", &[mid])?;
                if e.reg.field("pos.order", "account_move").is_ok() { orm::write(&e, "pos.order", &[oid], row(&[("account_move", mid.into())]))?; }
            }
            for c in consume { orm::unlink(&e, "ir.config_parameter", &[c])?; }   // an approval pays for exactly one order
            let issued = pos_loyalty::commit(&e, oid, &name, partner, &loyalty)?;
            let src = id_of(&cfg, "picking_type_id").and_then(|t| rec(&e, "stock.picking.type", t).ok()).and_then(|t| id_of(&t, "default_location_src_id"));
            pos_post::move_stock(&e, oid, &name, partner, src)?;
            let _ = (untaxed, refund_of);
            let mut v = order_view(&e, oid)?; if let Value::Map(m) = &mut v { m.insert("loyalty_issued".into(), Value::List(issued)); } Ok(v)
        })
        // Refund: a mirror order with negative quantities, returning the money through the same payment methods.
        .action("pos.order", "refund", |env, ids_, kw| {
            let e = env.sudo(); let oid = *ids_.first().ok_or_else(|| OdooError::User("Select an order".into()))?;
            let o = rec(&e, "pos.order", oid)?;
            { let sid0 = kw.get("session_id").and_then(|v| v.as_i64()).unwrap_or(0); let cfg0 = rec(&e, "pos.session", sid0).ok().and_then(|s| id_of(&s, "config_id")).and_then(|c| rec(&e, "pos.config", c).ok()).unwrap_or_default();
              if employee_for(&e, &cfg0, kw)?.1 == Role::Basic { return Err(OdooError::User("Only managers can refund orders".into())); } }
            if num(&o, "amount_total") < 0.0 { return Err(OdooError::User("A refund cannot be refunded again".into())); }
            let sid = kw.get("session_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("Missing POS session".into()))?;
            let mut lines = vec![];
            for l in children(&e, "pos.order.line", "order_id", oid)? {
                let already: f64 = orm::search_read(&e, "pos.order.line", &term("refunded_orderline_id", "=", l["id"].as_i64().unwrap()), &["qty".into()], None, None, 0)?.iter().map(|r| num(r, "qty")).sum();
                let left = num(&l, "qty") + already; if left <= 0.0 { continue; }
                lines.push(Value::Map(row(&[("product_id", id_of(&l, "product_id").unwrap_or(0).into()), ("qty", (-left).into()), ("price_unit", num(&l, "price_unit").into()), ("discount", num(&l, "discount").into()), ("refunded_orderline_id", l["id"].clone())])));
            }
            if lines.is_empty() { return Err(OdooError::User("Nothing left to refund on this order".into())); }
            let tot: f64 = lines.iter().map(|l| { let l = as_map(l); let p = rec(&e, "product.product", id_of(&l, "product_id").unwrap_or(0)).unwrap_or_default(); let t = id_of(&p, "product_tmpl_id").and_then(|t| rec(&e, "product.template", t).ok()).unwrap_or_default(); tax::compute(num(&l, "qty"), num(&l, "price_unit"), num(&l, "discount"), &tax::load(&e, &ids(&t, "taxes_id")).unwrap_or_default()).2 }).sum();
            let first_pay = children(&e, "pos.payment", "pos_order_id", oid)?.into_iter().find(|p| id_of(p, "payment_method_id").is_some() && !flag(p, "is_change") && num(p, "amount") > 0.0).ok_or_else(|| OdooError::User("The order has no payment to return".into()))?;
            let first = id_of(&first_pay, "payment_method_id").unwrap();
            if flag(kw, "quote") {   // what a refund would return and through which payment (the terminal must hand back card money first)
                let m = rec(&e, "pos.payment.method", first)?;
                return Ok(map(vec![("amount", r2(-tot).into()), ("method_id", first.into()), ("payment_id", first_pay["id"].clone()), ("terminal", Value::Bool(m.get("use_payment_terminal").and_then(|v| v.as_str()).map_or(false, |x| !x.is_empty())))]));
            }
            let mut k = row(&[("session_id", sid.into()), ("lines", Value::List(lines)), ("payments", Value::List(vec![Value::Map({ let mut p = row(&[("payment_method_id", first.into()), ("amount", r2(tot).into())]); if let Some(t) = kw.get("transaction_id") { p.insert("transaction_id".into(), t.clone()); } p })])), ("note", format!("Refund of {}", text(&o, "name").unwrap_or_default()).into())]);
            if let Some(p) = id_of(&o, "partner_id") { k.insert("partner_id".into(), p.into()); }
            if let Some(emp) = kw.get("employee_id") { k.insert("employee_id".into(), emp.clone()); }
            let f = env.rules.actions.get(&("pos.order".to_string(), "create_from_ui".to_string())).ok_or_else(|| OdooError::User("create_from_ui missing".into()))?;
            let res = f(env, &[], &k)?;
            if let Some(nid) = if let Value::Map(m) = &res { m.get("id").and_then(|v| v.as_i64()) } else { None } { pos_loyalty::reverse(&e, oid, nid)?; }
            Ok(res)
        })
        .action("pos.config", "loyalty_cards", |env, ids, kw| Ok(Value::List(pos_loyalty::cards(env, *ids.first().unwrap_or(&0), kw.get("partner_id").and_then(|v| v.as_i64()), text(kw, "code").as_deref())?)))
        .action("pos.config", "verify_employee", |env, ids, kw| {
            let e = env.sudo(); let cfg = rec(&e, "pos.config", *ids.first().unwrap_or(&0))?;
            let id = match (kw.get("employee_id").and_then(|v| v.as_i64()), text(kw, "badge").filter(|b| !b.is_empty())) {
                (Some(i), _) => i,
                (None, Some(b)) => find_one(&e, "hr.employee", term("barcode", "=", b.as_str()))?.ok_or_else(|| OdooError::User("Unknown badge".into()))?,
                _ => return Err(OdooError::User("Select a cashier".into())),
            };
            let emp = rec(&e, "hr.employee", id)?; let role = role_of(&cfg, id);
            if role == Role::None { return Err(OdooError::User("This employee cannot use this point of sale".into())); }
            // a badge scan proves presence; otherwise the PIN must match when the employee has one
            let pin = text(&emp, "pin").unwrap_or_default();
            if text(kw, "badge").is_none() && !pin.is_empty() {
                // lock an employee for 5 minutes after 5 wrong PINs (counter kept in ir.config_parameter, shared by every terminal)
                let key = format!("pos.pin.fail.{id}"); let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
                let slot = find_one(&e, "ir.config_parameter", term("key", "=", key.as_str()))?;
                let (mut n, mut since) = slot.and_then(|p| rec(&e, "ir.config_parameter", p).ok()).and_then(|p| text(&p, "value")).and_then(|v| { let (a, b) = v.split_once('|')?; Some((a.parse::<i64>().ok()?, b.parse::<i64>().ok()?)) }).unwrap_or((0, now));
                if now - since > 300 { n = 0; since = now; }
                if n >= 5 { return Ok(map(vec![("ok", false.into()), ("message", format!("Too many wrong PINs — try again in {} s", (300 - (now - since)).max(1)).into())])); }
                let save = |n: i64, since: i64| -> Result<()> { let v = format!("{n}|{since}"); match slot { Some(p) => orm::write(&e, "ir.config_parameter", &[p], row(&[("value", v.into())])), None => orm::create(&e, "ir.config_parameter", row(&[("key", key.clone().into()), ("value", v.into())])).map(|_| ()) } };
                if text(kw, "pin").unwrap_or_default() != pin { save(n + 1, if n == 0 { now } else { since })?; return Ok(map(vec![("ok", false.into()), ("message", "Wrong PIN".into())])); }   // Ok, not Err: an error would roll the counter back
                if n > 0 { save(0, now)?; }
            }
            Ok(map(vec![("ok", true.into()), ("id", id.into()), ("name", text(&emp, "name").unwrap_or_default().into()), ("role", (if role == Role::Advanced { "advanced" } else { "basic" }).into())]))
        })
        // sales dashboard across registers (multi-store view): totals, top products, payment mix, busiest hours
        .action("pos.config", "sales_report", |env, _, kw| sales_report(&env.sudo(), kw.get("days").and_then(|v| v.as_i64()).unwrap_or(7).clamp(1, 365)))
        // known lots / serial numbers of a tracked product with what is on hand, to suggest at the till
        .action("pos.config", "lots", |env, _, kw| {
            let e = env.sudo(); let pid = kw.get("product_id").and_then(|v| v.as_i64()).unwrap_or(0);
            if e.reg.model("stock.lot").is_err() { return Ok(Value::List(vec![])); }
            let mut out = vec![];
            for l in orm::search(&e, "stock.lot", &term("product_id", "=", pid), Some("name"), Some(200), 0)? {
                let lot = rec(&e, "stock.lot", l)?;
                let qty: f64 = if e.reg.field("stock.quant", "lot_id").is_ok() { children_by(&e, "stock.quant", &term("lot_id", "=", l))?.iter().map(|q| num(q, "quantity")).sum() } else { 0.0 };
                out.push(map(vec![("name", text(&lot, "name").unwrap_or_default().into()), ("qty", qty.into())]));
            }
            Ok(Value::List(out))
        })
        .action("pos.session", "summary", |env, ids, _| { let e = env.sudo(); Ok(Value::Map(session_summary(&e, *ids.first().ok_or_else(|| OdooError::User("Select a session".into()))?)?)) })
        .action("pos.session", "close_session", |env, ids, kw| {
            let e = env.sudo(); let sid = *ids.first().ok_or_else(|| OdooError::User("Select a session".into()))?;
            let s = rec(&e, "pos.session", sid)?;
            if text(&s, "state").as_deref() == Some("closed") { return Err(OdooError::User("This session is already closed".into())); }
            if employee_for(&e, &rec(&e, "pos.config", id_of(&s, "config_id").unwrap_or(0))?, kw)?.1 == Role::Basic { return Err(OdooError::User("Only managers can close the register".into())); }
            let mut sm = session_summary(&e, sid)?;
            let counted = kw.get("counted_cash").and_then(|v| v.as_f64());
            let diff = counted.map(|c| r2(c - num(&sm, "expected_cash")));
            let cfg = rec(&e, "pos.config", id_of(&s, "config_id").unwrap_or(0))?;
            if let Some(d) = diff { if flag(&cfg, "set_maximum_difference") && d.abs() > num(&cfg, "amount_authorized_diff") && !flag(kw, "force") { return Err(OdooError::User(format!("The cash difference {:.2} exceeds the authorized maximum {:.2}", d, num(&cfg, "amount_authorized_diff")))); } }
            let mut w = row(&[("state", "closed".into()), ("stop_at", orm::now().into()), ("closing_notes", text(kw, "notes").unwrap_or_default().into())]);
            if let Some(c) = counted { w.insert("cash_register_balance_end_real".into(), c.into()); }
            w.retain(|k, _| e.reg.field("pos.session", k).is_ok());
            if let Some(mv) = pos_post::close_entry(&e, sid, diff.unwrap_or(0.0))? { if e.reg.field("pos.session", "move_id").is_ok() { w.insert("move_id".into(), mv.into()); } }
            orm::write(&e, "pos.session", &[sid], w)?;
            sm.insert("state".into(), "closed".into()); sm.insert("difference".into(), diff.map(Value::Float).unwrap_or(Value::Null)); if let Some(c) = counted { sm.insert("counted_cash".into(), c.into()); }
            Ok(Value::Map(sm))
        })
}

fn loyalty_data(e: &Env, config: i64) -> Result<Value> {
    if !pos_loyalty::available(e) { return Ok(Value::List(vec![])); }
    let mut out = vec![];
    for p in pos_loyalty::programs(e, config)? {
        let rules = ids(&p, "rule_ids").into_iter().map(|i| rec(e, "loyalty.rule", i)).collect::<Result<Vec<_>>>()?.into_iter().map(|r| pick(&r, &["id", "mode", "code", "minimum_qty", "minimum_amount", "minimum_amount_tax_mode", "reward_point_amount", "reward_point_mode", "product_ids", "valid_product_ids", "any_product", "product_category_id"])).collect();
        let rewards = ids(&p, "reward_ids").into_iter().map(|i| rec(e, "loyalty.reward", i)).collect::<Result<Vec<_>>>()?.into_iter().map(|r| { let mut o = pick(&r, &["id", "description", "reward_type", "required_points", "discount", "discount_mode", "discount_applicability", "discount_max_amount", "discount_product_ids", "all_discount_product_ids", "discount_product_category_id", "reward_product_id", "reward_product_ids", "reward_product_qty"]); o.insert("tax_ids".into(), Value::List(vec![])); o }).collect();
        let mut o = pick(&p, &["id", "name", "program_type", "trigger", "applies_on", "is_nominative", "portal_point_name"]);
        o.insert("rules".into(), list(rules)); o.insert("rewards".into(), list(rewards));
        out.push(Value::Map(o));
    }
    Ok(Value::List(out))
}

fn categ_chain(e: &Env, mut c: Option<i64>) -> Vec<i64> {
    let mut out = vec![];
    while let Some(id) = c { if out.contains(&id) || out.len() > 32 { break; } out.push(id); c = rec(e, "product.category", id).ok().and_then(|r| id_of(&r, "parent_id")); }
    out
}

// ---- cashiers (pos_hr): basic employees sell; advanced ones also refund, discount, reprice and close ----
#[derive(PartialEq, Clone, Copy, Debug)]
enum Role { Open, Basic, Advanced, None }
fn uses_employees(env: &Env) -> bool { env.reg.field("pos.order", "employee_id").is_ok() && env.reg.field("pos.config", "basic_employee_ids").is_ok() }
fn role_of(cfg: &Row, emp: i64) -> Role {
    let (b, a) = (ids(cfg, "basic_employee_ids"), ids(cfg, "advanced_employee_ids"));
    if b.is_empty() && a.is_empty() { Role::Advanced } else if a.contains(&emp) { Role::Advanced } else if b.contains(&emp) { Role::Basic } else { Role::None }
}
/// Who is operating: `employee_id` is mandatory once the register is restricted to named cashiers.
fn employee_for(env: &Env, cfg: &Row, kw: &Row) -> Result<(Option<(i64, String)>, Role)> {
    if !uses_employees(env) { return Ok((None, Role::Open)); }
    let restricted = !ids(cfg, "basic_employee_ids").is_empty() || !ids(cfg, "advanced_employee_ids").is_empty();
    match kw.get("employee_id").and_then(|v| v.as_i64()) {
        Some(i) => { let role = role_of(cfg, i); if role == Role::None { return Err(OdooError::User("This employee cannot use this point of sale".into())); } let n = rec(env, "hr.employee", i).ok().and_then(|e| text(&e, "name")).unwrap_or_default(); Ok((Some((i, n)), role)) }
        None if restricted => Err(OdooError::User("Select a cashier first".into())),
        None => Ok((None, Role::Open)),
    }
}
fn employees_data(env: &Env, cfg: &Row) -> Result<Value> {
    if !uses_employees(env) { return Ok(Value::List(vec![])); }
    let mut out = vec![];
    let allowed: Vec<i64> = { let mut v = ids(cfg, "basic_employee_ids"); v.extend(ids(cfg, "advanced_employee_ids")); v };
    let all = if allowed.is_empty() { orm::search(env, "hr.employee", &term("active", "=", true), Some("name"), Some(50), 0)? } else { allowed };
    for i in all { let e = rec(env, "hr.employee", i)?; let r = role_of(cfg, i);
        out.push(Value::Map(row(&[("id", i.into()), ("name", text(&e, "name").unwrap_or_default().into()), ("role", (if r == Role::Basic { "basic" } else { "advanced" }).into()), ("has_pin", (!text(&e, "pin").unwrap_or_default().is_empty()).into())]))); }
    Ok(Value::List(out))
}

struct Built { prepared: Vec<Row>, items: Vec<pos_loyalty::Item>, untaxed: f64, tax: f64, total: f64 }
/// Price the terminal's lines from product data (never trusting client totals), enforcing discount/price permissions.
fn build_lines(e: &Env, cfg: &Row, role: Role, lines: &[Value], strict: bool) -> Result<Built> {
    let (mut untaxed, mut tax_sum, mut total) = (0.0, 0.0, 0.0);
    let mut prepared: Vec<Row> = vec![]; let mut items: Vec<pos_loyalty::Item> = vec![];
            // combos: parent uuid → (qty, combo groups it needs, groups already chosen)
            let mut combos: BTreeMap<String, (f64, Vec<i64>, Vec<i64>, String)> = BTreeMap::new();
            for l in lines {
                let l = as_map(l); let mut pid = l.get("product_id").and_then(|v| v.as_i64()).unwrap_or(0);   // combo children name an item instead
                let (mut qty, mut forced_price, mut combo_item, mut combo_parent) = (num(&l, "qty"), None::<f64>, None::<i64>, None::<String>);
                if let Some(par) = text(&l, "combo_parent").filter(|x| !x.is_empty()) {
                    // a child of a combo: product, price and quantity come from the combo item, never from the client
                    let item_id = l.get("combo_item_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("Combo line without an item".into()))?;
                    let item = rec(e, "product.combo.item", item_id).map_err(|_| OdooError::User("Unknown combo item".into()))?;
                    let g = combos.get_mut(&par).ok_or_else(|| OdooError::User("Combo line without its combo".into()))?;
                    let cid = id_of(&item, "combo_id").unwrap_or(0);
                    if !g.1.contains(&cid) { return Err(OdooError::User("That item is not part of this combo".into())); }
                    if g.2.contains(&cid) { return Err(OdooError::User("Choose only one item per combo part".into())); }
                    g.2.push(cid); qty = g.0;
                    pid = id_of(&item, "product_id").ok_or_else(|| OdooError::User("Combo item without product".into()))?;
                    forced_price = Some(num(&item, "extra_price")); combo_item = Some(item_id); combo_parent = Some(par);
                }
                if pid == 0 { return Err(OdooError::User("Line without product".into())); }
                let p = rec(e, "product.product", pid)?; let t = id_of(&p, "product_tmpl_id").map(|t| rec(e, "product.template", t)).transpose()?.unwrap_or_default();
                if qty == 0.0 { continue; }
                if text(&t, "type").as_deref() == Some("combo") && combo_parent.is_none() {
                    let need = ids(&t, "combo_ids"); if need.is_empty() { return Err(OdooError::User("This combo has no content".into())); }
                    combos.insert(text(&l, "uuid").unwrap_or_default(), (qty, need, vec![], text(&t, "name").unwrap_or_default()));
                }
                let is_tip = id_of(cfg, "tip_product_id") == Some(pid);   // tips are free-amount lines
                let price = if let Some(fp) = forced_price { fp } else if text(&t, "type").as_deref() == Some("combo") { num(&t, "list_price") } else if l.contains_key("price_unit") && (is_tip || !flag(cfg, "restrict_price_control")) { num(&l, "price_unit") } else { num(&t, "list_price") };
                let disc = num(&l, "discount").clamp(0.0, 100.0);
                if role == Role::Basic && !is_tip && forced_price.is_none() && text(&t, "type").as_deref() != Some("combo") && (disc > 0.0 || (l.contains_key("price_unit") && (num(&l, "price_unit") - num(&t, "list_price")).abs() > 0.004)) { return Err(OdooError::User("Only managers can change prices or give discounts".into())); }
                if disc > 0.0 && !flag(cfg, "manual_discount") { return Err(OdooError::User("Manual discounts are disabled for this point of sale".into())); }
                let tx = ids(&t, "taxes_id");
                let (u, tt, tot) = tax::compute(qty, price, disc, &tax::load(e, &tx)?);
                untaxed += u; tax_sum += tt; total += tot;
                // lots / serial numbers: required for tracked goods being sold
                let tracking = text(&t, "tracking").unwrap_or_default();
                let lots: Vec<String> = match l.get("lots") { Some(Value::List(v)) => v.iter().filter_map(|x| x.as_str().map(|s| s.trim().to_string())).filter(|s| !s.is_empty()).collect(), _ => vec![] };
                if strict && matches!(tracking.as_str(), "lot" | "serial") && qty > 0.0 { check_lots(e, pid, &tracking, qty, &lots, &text(&t, "name").unwrap_or_default())?; }
                items.push(pos_loyalty::Item { product: pid, categ: id_of(&t, "categ_id"), qty, price, tax_ids: tx.clone(), untaxed: u, total: tot });
                prepared.push(row(&[("product_id", pid.into()), ("full_product_name", text(&t, "name").unwrap_or_default().into()), ("name", text(&t, "name").unwrap_or_default().into()), ("qty", qty.into()), ("price_unit", price.into()), ("discount", disc.into()), ("price_subtotal", u.into()), ("price_subtotal_incl", tot.into()), ("customer_note", text(&l, "note").unwrap_or_default().into()), ("tax_ids", set6(&tx)),
                    ("refunded_orderline_id", l.get("refunded_orderline_id").cloned().unwrap_or(Value::Null)), ("uuid", text(&l, "uuid").unwrap_or_default().into()),
                    ("_lots", Value::List(lots.into_iter().map(Value::Text).collect())), ("_combo_parent", combo_parent.map_or(Value::Null, Value::Text)), ("_combo_item", combo_item.map_or(Value::Null, Value::Int))]));
            }
            for (_, (_, need, got, name)) in &combos { if strict && need.len() != got.len() { return Err(OdooError::User(format!("Choose one item for every part of the combo “{name}”"))); } }
    Ok(Built { prepared, items, untaxed, tax: tax_sum, total })
}

/// (priced lines, tax, total) for an unpaid table order. Rewards are not applied until payment.
pub fn draft_lines(e: &Env, cfg: &Row, kw: &Row) -> Result<(Vec<Row>, f64, f64)> {
    let b = build_lines(e, cfg, Role::Advanced, kw_list(kw, "lines"), false)?;
    Ok((b.prepared, r2(b.tax), r2(b.total)))
}

/// Receipt printer first (no categories), then kitchen/bar printers with the categories they serve.
fn printers_data(e: &Env, cfg: &Row) -> Value {
    Value::List(crate::pos_hw::targets(e, cfg).into_iter().map(|(name, target, cats, kind)| map(vec![("name", name.clone().into()), ("target", target.into()), ("kind", kind.into()), ("receipt", Value::Bool(name == "Receipt printer" && cats.is_empty())), ("category_ids", Value::List(cats.into_iter().map(Value::Int).collect()))])).collect())
}

fn sales_report(e: &Env, days: i64) -> Result<Value> {
    let since = orm::shift_date(&orm::today(), -(days - 1));
    let orders = orm::search_read(e, "pos.order", &Domain::And(vec![term("date_order", ">=", since.as_str()), term("state", "in", Value::List(vec!["paid".into(), "done".into(), "invoiced".into()]))]), &["id".into(), "config_id".into(), "amount_total".into(), "date_order".into(), "session_id".into()], Some("id"), None, 0)?;
    let mut by_cfg: BTreeMap<i64, (String, f64, i64)> = BTreeMap::new(); let mut by_hour = [0.0f64; 24]; let mut by_day: BTreeMap<String, f64> = BTreeMap::new();
    for o in &orders {
        let cid = id_of(o, "config_id").unwrap_or(0);
        let name = by_cfg.get(&cid).map(|x| x.0.clone()).unwrap_or_else(|| rec(e, "pos.config", cid).ok().and_then(|c| text(&c, "name")).unwrap_or_default());
        let en = by_cfg.entry(cid).or_insert((name, 0.0, 0)); en.1 += num(o, "amount_total"); en.2 += 1;
        let d = text(o, "date_order").unwrap_or_default();
        if d.len() >= 13 { if let Ok(h) = d[11..13].parse::<usize>() { if h < 24 { by_hour[h] += num(o, "amount_total"); } } }
        if d.len() >= 10 { *by_day.entry(d[..10].to_string()).or_default() += num(o, "amount_total"); }
    }
    let oids: Vec<Value> = orders.iter().map(|o| o["id"].clone()).collect();
    let mut prods: BTreeMap<String, (f64, f64)> = BTreeMap::new(); let mut pays: BTreeMap<String, f64> = BTreeMap::new();
    if !oids.is_empty() {
        for l in orm::search_read(e, "pos.order.line", &term("order_id", "in", Value::List(oids.clone())), &["full_product_name".into(), "qty".into(), "price_subtotal_incl".into()], None, None, 0)? { let en = prods.entry(text(&l, "full_product_name").unwrap_or_default()).or_default(); en.0 += num(&l, "qty"); en.1 += num(&l, "price_subtotal_incl"); }
        for p in orm::search_read(e, "pos.payment", &term("pos_order_id", "in", Value::List(oids)), &["payment_method_id".into(), "amount".into()], None, None, 0)? {
            let n = id_of(&p, "payment_method_id").and_then(|m| rec(e, "pos.payment.method", m).ok()).and_then(|m| text(&m, "name")).unwrap_or_default(); *pays.entry(n).or_default() += num(&p, "amount"); }
    }
    let total: f64 = orders.iter().map(|o| num(o, "amount_total")).sum();
    let mut top: Vec<(String, (f64, f64))> = prods.into_iter().collect(); top.sort_by(|a, b| b.1 .1.partial_cmp(&a.1 .1).unwrap()); top.truncate(10);
    Ok(map(vec![("days", days.into()), ("since", since.into()), ("orders", (orders.len() as i64).into()), ("total", r2(total).into()), ("average", r2(if orders.is_empty() { 0.0 } else { total / orders.len() as f64 }).into()),
        ("registers", Value::List(by_cfg.into_iter().map(|(id, (n, t, c))| map(vec![("id", id.into()), ("name", n.into()), ("total", r2(t).into()), ("orders", c.into())])).collect())),
        ("top_products", Value::List(top.into_iter().map(|(n, (q, t))| map(vec![("name", n.into()), ("qty", q.into()), ("total", r2(t).into())])).collect())),
        ("payments", Value::List(pays.into_iter().map(|(n, a)| map(vec![("name", n.into()), ("amount", r2(a).into())])).collect())),
        ("by_hour", Value::List(by_hour.iter().map(|x| Value::Float(r2(*x))).collect())),
        ("by_day", Value::List(by_day.into_iter().map(|(d, a)| map(vec![("day", d.into()), ("amount", r2(a).into())])).collect()))]))
}

/// Create the order lines, the lot/serial rows and the combo links (children point at their parent line).
pub fn insert_lines(e: &Env, oid: i64, prepared: Vec<Row>) -> Result<()> {
    let mut by_uuid: BTreeMap<String, i64> = BTreeMap::new(); let mut links: Vec<(i64, String, i64)> = vec![];
    for mut l in prepared {
        let lots = match l.remove("_lots") { Some(Value::List(v)) => v, _ => vec![] };
        let parent = match l.remove("_combo_parent") { Some(Value::Text(t)) => Some(t), _ => None }; let item = l.remove("_combo_item").and_then(|v| v.as_i64());
        let uuid = text(&l, "uuid").unwrap_or_default(); let pid = l.get("product_id").and_then(|v| v.as_i64()).unwrap_or(0);
        l.insert("order_id".into(), oid.into()); l.retain(|k, _| e.reg.field("pos.order.line", k).is_ok());
        let lid = orm::create(e, "pos.order.line", l)?;
        if !uuid.is_empty() { by_uuid.insert(uuid, lid); }
        if let (Some(par), Some(it)) = (parent, item) { links.push((lid, par, it)); }
        if e.reg.model("pos.pack.operation.lot").is_ok() { for n in lots { if let Some(name) = n.as_str() { let mut v = row(&[("pos_order_line_id", lid.into()), ("lot_name", name.into()), ("order_id", oid.into()), ("product_id", pid.into())]); v.retain(|k, _| e.reg.field("pos.pack.operation.lot", k).is_ok()); orm::create(e, "pos.pack.operation.lot", v)?; } } }
    }
    for (child, par, item) in links { if let Some(pl) = by_uuid.get(&par) { orm::write(e, "pos.order.line", &[child], row(&[("combo_parent_id", (*pl).into()), ("combo_item_id", item.into())]))?; } }
    Ok(())
}

/// Serial: exactly one unique number per unit, never sold twice (unless returned). Lot: at least one lot number. Known lots only when the product has lots on file.
fn check_lots(e: &Env, pid: i64, tracking: &str, qty: f64, names: &[String], product: &str) -> Result<()> {
    if tracking == "serial" {
        if qty.fract() != 0.0 || names.len() != qty as usize { return Err(OdooError::User(format!("“{product}” needs {} serial number(s)", qty.ceil() as i64))); }
        let mut seen = std::collections::BTreeSet::new(); for n in names { if !seen.insert(n) { return Err(OdooError::User(format!("Serial number {n} is entered twice"))); } }
    } else if names.is_empty() { return Err(OdooError::User(format!("“{product}” needs a lot number"))); }
    if e.reg.model("stock.lot").is_ok() {
        let known = !orm::search(e, "stock.lot", &term("product_id", "=", pid), None, Some(1), 0)?.is_empty();
        for n in names {
            if known && find_one(e, "stock.lot", Domain::And(vec![term("product_id", "=", pid), term("name", "=", n.as_str())]))?.is_none() { return Err(OdooError::User(format!("Unknown {} {n} for “{product}”", if tracking == "serial" { "serial number" } else { "lot" }))); }
            if tracking == "serial" && e.reg.model("pos.pack.operation.lot").is_ok() {
                for r in children_by(e, "pos.pack.operation.lot", &Domain::And(vec![term("lot_name", "=", n.as_str()), term("product_id", "=", pid)]))? {
                    if let Some(l) = id_of(&r, "pos_order_line_id") { if num(&rec(e, "pos.order.line", l)?, "qty") > 0.0 && orm::search(e, "pos.order.line", &term("refunded_orderline_id", "=", l), None, Some(1), 0)?.is_empty() { return Err(OdooError::User(format!("Serial number {n} was already sold"))); } }
                }
            }
        }
    }
    Ok(())
}
fn children_by(e: &Env, model: &str, dom: &Domain) -> Result<Vec<Row>> { orm::search(e, model, dom, None, None, 0)?.into_iter().map(|i| rec(e, model, i)).collect() }

/// Parts of a combo: each part offers items (product + extra price); the customer picks one per part.
fn combo_groups(e: &Env, combo_ids: &[i64]) -> Value {
    if e.reg.model("product.combo").is_err() { return Value::List(vec![]); }
    Value::List(combo_ids.iter().filter_map(|c| {
        let g = rec(e, "product.combo", *c).ok()?;
        let items = children(e, "product.combo.item", "combo_id", *c).ok()?.into_iter().filter_map(|i| {
            let p = rec(e, "product.product", id_of(&i, "product_id")?).ok()?; let name = id_of(&p, "product_tmpl_id").and_then(|t| rec(e, "product.template", t).ok()).and_then(|t| text(&t, "name")).unwrap_or_default();
            let tx = id_of(&p, "product_tmpl_id").and_then(|t| rec(e, "product.template", t).ok()).map(|t| ids(&t, "taxes_id")).unwrap_or_default();
            Some(map(vec![("id", i["id"].clone()), ("product_id", p["id"].clone()), ("name", name.into()), ("extra", num(&i, "extra_price").into()), ("tax_ids", Value::List(tx.into_iter().map(Value::Int).collect()))]))
        }).collect();
        Some(map(vec![("id", (*c).into()), ("name", text(&g, "name").unwrap_or_default().into()), ("items", Value::List(items))]))
    }).collect())
}
