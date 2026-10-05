//! point_of_sale back-office behaviour ported from `addons/point_of_sale/models`:
//! `pos.config` (computes, constraints, guards), `pos.payment.method` and `pos.payment`.
//! Sessions live in `pos_session_methods`, orders and lines in `pos_order_methods`.
//!
//! Deviations from the Python source, kept on purpose so the terminal runtime in `pos.rs` keeps working:
//! * `pos.config._check_currencies` does not require pricelist currencies to equal the config currency (the runtime sells in pricelist currencies).
//! * `pos.payment.method.is_cash_count/type`: a method without a journal keeps an explicitly given `is_cash_count` (the runtime creates journal-less cash methods).
//! * `pos.payment._check_amount` and `_check_payment_method_id` are enforced on write (and by `sync_from_ui` for the payments it creates), not on a plain create: the runtime stores payments of integrations and of invoiced orders out of band.
use crate::util::*;
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, OdooError, Result, Row, Rules, Value};

pub fn rules() -> Rules {
    config_rules().merge(payment_method_rules()).merge(payment_rules())
        .merge(crate::pos_session_methods::rules()).merge(crate::pos_order_methods::rules())
}

// ---------------------------------------------------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------------------------------------------------
pub(crate) fn flag(r: &Row, k: &str) -> bool { r.get(k).map_or(false, |v| v.truthy()) }
pub(crate) fn has_model(env: &Env, m: &str) -> bool { env.reg.model(m).is_ok() }
pub(crate) fn id_list(ids: &[i64]) -> Value { Value::List(ids.iter().map(|i| Value::Int(*i)).collect()) }
pub(crate) fn set6(ids: &[i64]) -> Value { Value::List(vec![Value::List(vec![6.into(), 0.into(), id_list(ids)])]) }
pub(crate) fn user_err<T>(m: impl Into<String>) -> Result<T> { Err(OdooError::User(m.into())) }
pub(crate) fn invalid<T>(m: impl Into<String>) -> Result<T> { Err(OdooError::Validation(m.into())) }
/// `[id, display_name]` (what a many2one reads as) or `false`.
pub(crate) fn m2o(env: &Env, model: &str, id: Option<i64>) -> Value {
    match id {
        Some(i) => Value::List(vec![Value::Int(i), Value::Text(orm::display_names(&env.sudo(), model, &[i]).ok().and_then(|m| m.get(&i).cloned()).unwrap_or_default())]),
        None => Value::Bool(false),
    }
}
pub(crate) fn many(env: &Env, model: &str, ids_: &[i64]) -> Result<Vec<Row>> { ids_.iter().map(|i| rec(env, model, *i)).collect() }

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y }; let era = y.div_euclid(400); let yoe = y.rem_euclid(400);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1; let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
/// Seconds since the epoch of `YYYY-MM-DD[ HH:MM:SS]`.
pub(crate) fn epoch_secs(s: &str) -> Option<i64> {
    let n = |a: usize, b: usize| s.get(a..b).and_then(|x| x.parse::<i64>().ok());
    let days = days_from_civil(n(0, 4)?, n(5, 7)?, n(8, 10)?);
    Some(days * 86400 + n(11, 13).unwrap_or(0) * 3600 + n(14, 16).unwrap_or(0) * 60 + n(17, 19).unwrap_or(0))
}
fn company_of(env: &Env, r: &Row) -> Option<i64> {
    id_of(r, "company_id").or_else(|| env.ctx.get("company_id").and_then(|v| v.as_i64())).or_else(|| find_one(&env.sudo(), "res.company", Domain::True).ok().flatten())
}
/// Resolve an xml id (`module.name`) to the record id of `model`.
pub(crate) fn xmlid(env: &Env, module: &str, name: &str) -> Option<i64> {
    let e = env.sudo();
    let id = find_one(&e, "ir.model.data", Domain::And(vec![term("module", "=", module), term("name", "=", name)])).ok().flatten()?;
    rec(&e, "ir.model.data", id).ok().and_then(|r| r.get("res_id").and_then(|v| v.as_i64()))
}

// ---------------------------------------------------------------------------------------------------------------------
// pos.config
// ---------------------------------------------------------------------------------------------------------------------
fn config_sessions(env: &Env, cfg: i64) -> Result<Vec<Row>> {
    let e = env.sudo();
    orm::search(&e, "pos.session", &term("config_id", "=", cfg), Some("id desc"), None, 0)?.into_iter().map(|i| rec(&e, "pos.session", i)).collect()
}
fn open_sessions(env: &Env, cfg: i64) -> Result<Vec<Row>> { Ok(config_sessions(env, cfg)?.into_iter().filter(|s| text(s, "state").as_deref() != Some("closed")).collect()) }
/// `_compute_current_session`: the newest unclosed, non-rescue session.
pub(crate) fn current_session(env: &Env, cfg: i64) -> Result<Option<Row>> { Ok(open_sessions(env, cfg)?.into_iter().find(|s| !flag(s, "rescue"))) }
fn cfg_id(r: &Row) -> i64 { r["id"].as_i64().unwrap_or(0) }

/// `_compute_cash_control`: the config handles cash when one of its payment methods is a cash one.
pub(crate) fn cash_control_of(env: &Env, r: &Row) -> Result<bool> {
    for m in many(env, "pos.payment.method", &ids(r, "payment_method_ids"))? { if flag(&m, "is_cash_count") { return Ok(true); } }
    Ok(false)
}
fn config_currency(env: &Env, r: &Row) -> Option<i64> {
    let e = env.sudo();
    match id_of(r, "journal_id").and_then(|j| rec(&e, "account.journal", j).ok()) {
        Some(j) => id_of(&j, "currency_id").or_else(|| id_of(&j, "company_id").and_then(|c| rec(&e, "res.company", c).ok()).and_then(|c| id_of(&c, "currency_id"))),
        None => id_of(r, "company_id").and_then(|c| rec(&e, "res.company", c).ok()).and_then(|c| id_of(&c, "currency_id")),
    }
}
pub(crate) fn config_currency_id(env: &Env, cfg: i64) -> Option<i64> { rec(env, "pos.config", cfg).ok().and_then(|r| config_currency(env, &r)) }

fn config_rules() -> Rules {
    Rules::default()
        .compute("pos.config", "cash_control", |env, r| Ok(cash_control_of(env, r)?.into()))
        .compute("pos.config", "currency_id", |env, r| Ok(m2o(env, "res.currency", config_currency(env, r))))
        .compute("pos.config", "company_has_template", |env, r| {
            let e = env.sudo(); let Some(c) = id_of(r, "company_id") else { return Ok(false.into()) };
            let existing = has_model(&e, "account.move.line") && !orm::search(&e.with_ctx("active_test", Value::Bool(false)), "account.move.line", &Domain::Or(vec![term("company_id", "=", c), term("company_id", "=", Value::Bool(false))]), None, Some(1), 0)?.is_empty();   // move lines of a single-company database may lack the (uncomputed) company
            let chart = rec(&e, "res.company", c).ok().map_or(false, |c| flag(&c, "chart_template"));
            Ok((existing || chart).into())
        })
        .compute("pos.config", "is_installed_account_accountant", |env, _| {
            let e = env.sudo(); if !has_model(&e, "ir.module.module") { return Ok(false.into()); }
            Ok(find_one(&e, "ir.module.module", Domain::And(vec![term("name", "=", "account_accountant"), term("state", "=", "installed")]))?.is_some().into())
        })
        .compute("pos.config", "current_session_id", |env, r| Ok(m2o(env, "pos.session", current_session(env, cfg_id(r))?.and_then(|s| s["id"].as_i64()))))
        .compute("pos.config", "current_session_state", |env, r| Ok(current_session(env, cfg_id(r))?.and_then(|s| text(&s, "state")).map_or(Value::Bool(false), Value::Text)))
        .compute("pos.config", "has_active_session", |env, r| Ok((!open_sessions(env, cfg_id(r))?.is_empty()).into()))
        .compute("pos.config", "number_of_rescue_session", |env, r| Ok((open_sessions(env, cfg_id(r))?.iter().filter(|s| flag(s, "rescue")).count() as i64).into()))
        .compute("pos.config", "last_session_closing_cash", |env, r| Ok(last_closed(env, cfg_id(r))?.map_or(0.0, |s| num(&s, "cash_register_balance_end_real")).into()))
        .compute("pos.config", "last_session_closing_date", |env, r| Ok(last_closed(env, cfg_id(r))?.and_then(|s| text(&s, "stop_at")).filter(|d| d.len() >= 10).map_or(Value::Bool(false), |d| Value::Text(d[..10].to_string()))))
        .compute("pos.config", "pos_session_username", |env, r| Ok(session_user(env, r)?.map_or(Value::Bool(false), |s| id_of(&s, "user_id").and_then(|u| rec(env, "res.users", u).ok()).and_then(|u| id_of(&u, "partner_id")).and_then(|p| rec(env, "res.partner", p).ok()).and_then(|p| text(&p, "name")).map_or(Value::Bool(false), Value::Text))))
        .compute("pos.config", "pos_session_state", |env, r| Ok(session_user(env, r)?.and_then(|s| text(&s, "state")).map_or(Value::Bool(false), Value::Text)))
        .compute("pos.config", "pos_session_duration", |env, r| {
            // (datetime.now() - session.start_at).days, as text because the field is a Char
            let days = session_user(env, r)?.and_then(|s| text(&s, "start_at")).and_then(|d| epoch_secs(&d)).and_then(|t| epoch_secs(&orm::now()).map(|n| (n - t).div_euclid(86400))).unwrap_or(0);
            Ok(Value::Text(days.to_string()))
        })
        .compute("pos.config", "current_user_id", |env, r| Ok(m2o(env, "res.users", session_user(env, r)?.and_then(|s| id_of(&s, "user_id")))))
        // stored: the picking type's warehouse, else the company's first warehouse
        .compute("pos.config", "warehouse_id", |env, r| {
            let e = env.sudo(); if !has_model(&e, "stock.warehouse") { return Ok(Value::Null); }
            if let Some(w) = id_of(r, "picking_type_id").and_then(|p| rec(&e, "stock.picking.type", p).ok()).and_then(|p| id_of(&p, "warehouse_id")) { return Ok(Value::Int(w)); }
            let dom = match company_of(&e, r) { Some(c) => term("company_id", "=", c), None => Domain::True };
            Ok(find_one(&e, "stock.warehouse", dom)?.map_or(Value::Null, Value::Int))
        })
        // ---- constraints (run after create/write like @api.constrains) ----
        .after_create("pos.config", |env, ids_, _| { for i in ids_ { check_config(env, *i, None)?; } Ok(()) })
        .after_write("pos.config", |env, ids_, vals| {
            for i in ids_ { check_config(env, *i, Some(vals))?; }
            forbid_changes_while_open(env, ids_, vals)?;
            set_fiscal_position(env, ids_)
        })
        .before_write("pos.config", |env, mut vals| {
            // _reset_default_on_vals: an emptied tip product falls back to the module's default one
            if vals.contains_key("tip_product_id") && !flag(&vals, "tip_product_id") && flag(&vals, "iface_tipproduct") {
                match xmlid(env, "point_of_sale", "product_product_tip") {
                    Some(p) => { vals.insert("tip_product_id".into(), p.into()); }
                    None => return user_err("The default tip product is missing. Please manually specify the tip product. (See Tips field.)"),
                }
            }
            Ok(vals)
        })
        // sequences created with the config in Python are made lazily (see pos_order_methods::config_sequence)
        .action("pos.config", "_check_before_creating_new_session", |env, ids_, _| { for i in ids_ { check_before_new_session(env, *i)?; } Ok(Value::Bool(true)) })
        .action("pos.config", "_check_payment_method_ids", |env, ids_, _| { for i in ids_ { check_payment_method_ids(env, *i)?; } Ok(Value::Bool(true)) })
        .action("pos.config", "_check_company_has_template", |env, ids_, _| { for i in ids_ { check_company_has_template(env, *i)?; } Ok(Value::Bool(true)) })
        .action("pos.config", "_check_profit_loss_cash_journal", |env, ids_, _| { for i in ids_ { check_profit_loss(env, *i)?; } Ok(Value::Bool(true)) })
        .action("pos.config", "_check_pricelists", |env, ids_, _| { for i in ids_ { check_pricelists(env, &rec(env, "pos.config", *i)?)?; } Ok(Value::Bool(true)) })
        .action("pos.config", "_check_company_payment", |env, ids_, _| { for i in ids_ { check_company_payment(env, &rec(env, "pos.config", *i)?)?; } Ok(Value::Bool(true)) })
        .action("pos.config", "_check_currencies", |env, ids_, _| { for i in ids_ { check_currencies(env, &rec(env, "pos.config", *i)?)?; } Ok(Value::Bool(true)) })
        .action("pos.config", "open_existing_session_cb", |env, ids_, _| {
            let cfg = *ids_.first().ok_or_else(|| OdooError::User("Select a point of sale".into()))?;
            let sid = current_session(env, cfg)?.and_then(|s| s["id"].as_i64());
            open_session_action(env, cfg, sid)
        })
        .action("pos.config", "_open_session", |env, ids_, kw| {
            let cfg = *ids_.first().ok_or_else(|| OdooError::User("Select a point of sale".into()))?;
            open_session_action(env, cfg, kw.get("session_id").and_then(|v| v.as_i64()))
        })
        .action("pos.config", "open_opened_rescue_session_form", |env, ids_, _| {
            let cfg = *ids_.first().ok_or_else(|| OdooError::User("Select a point of sale".into()))?;
            let rescue: Vec<i64> = open_sessions(env, cfg)?.into_iter().filter(|s| flag(s, "rescue")).filter_map(|s| s["id"].as_i64()).collect();
            Ok(if rescue.len() == 1 {
                Value::Map(row(&[("res_model", "pos.session".into()), ("view_mode", "form".into()), ("res_id", rescue[0].into()), ("type", "ir.actions.act_window".into())]))
            } else {
                Value::Map(row(&[("name", "Rescue Sessions".into()), ("res_model", "pos.session".into()), ("view_mode", "list,form".into()), ("domain", Value::List(vec![Value::List(vec!["id".into(), "in".into(), id_list(&rescue)])])), ("type", "ir.actions.act_window".into())]))
            })
        })
        .action("pos.config", "_get_available_categories", |env, ids_, _| Ok(id_list(&available_categories(env, &rec(env, "pos.config", *ids_.first().unwrap_or(&0))?)?)))
        .action("pos.config", "_get_available_product_domain", |env, ids_, _| {
            let cfg = rec(env, "pos.config", *ids_.first().unwrap_or(&0))?;
            let t = |f: &str, v: Value| Value::List(vec![f.into(), "=".into(), v]);
            let mut d = vec![t("active", true.into()), t("available_in_pos", true.into()), t("sale_ok", true.into())];
            if flag(&cfg, "limit_categories") && !ids(&cfg, "iface_available_categ_ids").is_empty() { d.push(Value::List(vec!["pos_categ_ids".into(), "in".into(), id_list(&available_categories(env, &cfg)?)])); }
            Ok(Value::List(d))
        })
        .action("pos.config", "_get_payment_method", |env, ids_, kw| {
            let cfg = rec(env, "pos.config", *ids_.first().unwrap_or(&0))?; let want = text(kw, "payment_type").unwrap_or_default();
            for pm in many(env, "pos.payment.method", &ids(&cfg, "payment_method_ids"))? { if pm_type(env, &pm) == want { return Ok(Value::Int(pm["id"].as_i64().unwrap())); } }
            Ok(Value::Bool(false))
        })
        .action("pos.config", "_get_available_pricelists", |env, ids_, _| {
            let cfg = rec(env, "pos.config", *ids_.first().unwrap_or(&0))?;
            Ok(if flag(&cfg, "use_pricelist") { id_list(&ids(&cfg, "available_pricelist_ids")) } else { id_of(&cfg, "pricelist_id").map_or(id_list(&[]), |p| id_list(&[p])) })
        })
        .action("pos.config", "_link_same_non_cash_payment_methods", |env, ids_, kw| {
            let src = rec(env, "pos.config", kw.get("source_config_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("source_config_id required".into()))?)?;
            let pms: Vec<i64> = many(env, "pos.payment.method", &ids(&src, "payment_method_ids"))?.into_iter().filter(|p| !flag(p, "is_cash_count")).filter_map(|p| p["id"].as_i64()).collect();
            if !pms.is_empty() { orm::write(env, "pos.config", ids_, row(&[("payment_method_ids", Value::List(pms.iter().map(|p| Value::List(vec![4.into(), (*p).into()])).collect()))]))?; }
            Ok(Value::Bool(true))
        })
        .action("pos.config", "_is_journal_exist", |env, _, kw| {
            let (code, name, company) = (text(kw, "journal_code").unwrap_or_default(), text(kw, "name").unwrap_or_default(), kw.get("company_id").and_then(|v| v.as_i64()).unwrap_or(1));
            let dom = Domain::And(vec![term("name", "=", name.as_str()), term("code", "=", code.as_str()), term("company_id", "=", company)]);
            Ok(Value::Int(find_or_create(env, "account.journal", dom, row(&[("name", name.as_str().into()), ("code", code.as_str().into()), ("type", "cash".into()), ("company_id", company.into())]))?))
        })
        .action("pos.config", "_is_pos_pm_exist", |env, _, kw| {
            let (name, journal, company) = (text(kw, "name").unwrap_or_default(), kw.get("journal_id").and_then(|v| v.as_i64()).unwrap_or(0), kw.get("company_id").and_then(|v| v.as_i64()).unwrap_or(1));
            let dom = Domain::And(vec![term("name", "=", name.as_str()), term("journal_id", "=", journal), term("company_id", "=", company)]);
            Ok(Value::Int(find_or_create(env, "pos.payment.method", dom, row(&[("name", name.as_str().into()), ("journal_id", journal.into()), ("company_id", company.into())]))?))
        })
        .action("pos.config", "_create_cash_payment_method", |env, _, kw| Ok(Value::Int(create_cash_payment_method(env, kw)?)))
        .action("pos.config", "_create_journal_and_payment_methods", |env, _, kw| {
            let (journal, pms) = create_journal_and_payment_methods(env, kw)?;
            Ok(Value::List(vec![Value::Int(journal), id_list(&pms)]))
        })
        .action("pos.config", "_get_special_products", |env, _, _| Ok(id_list(&xmlid(env, "point_of_sale", "product_product_tip").into_iter().collect::<Vec<_>>())))
        .action("pos.config", "_get_customer_display_data", |env, ids_, _| {
            let c = rec(env, "pos.config", *ids_.first().unwrap_or(&0))?; let kind = text(&c, "customer_display_type").unwrap_or_else(|| "none".into());
            let mut out = row(&[("config_id", c["id"].clone()), ("access_token", c.get("access_token").cloned().unwrap_or(Value::Bool(false))), ("type", kind.as_str().into()), ("has_bg_img", flag(&c, "customer_display_bg_img").into()), ("company_id", id_of(&c, "company_id").map_or(Value::Bool(false), Value::Int))]);
            if kind != "none" { out.insert("proxy_ip".into(), c.get("proxy_ip").cloned().unwrap_or(Value::Bool(false))); }
            Ok(Value::Map(out))
        })
        .action("pos.config", "get_pos_kanban_view_state", |env, _, _| {
            let e = env.sudo(); let company = company_of(&e, &Row::new());
            let has_cfg = match company { Some(c) => !orm::search(&e, "pos.config", &term("company_id", "=", c), None, Some(1), 0)?.is_empty(), None => false };
            let chart = company.and_then(|c| rec(&e, "res.company", c).ok()).map_or(false, |c| flag(&c, "chart_template"));
            let restaurant = has_model(&e, "ir.module.module") && find_one(&e, "ir.module.module", Domain::And(vec![term("name", "=", "pos_restaurant"), term("state", "=", "installed")]))?.is_some();
            Ok(Value::Map(row(&[("has_pos_config", has_cfg.into()), ("has_chart_template", chart.into()), ("is_restaurant_installed", restaurant.into()), ("is_main_company", (company == Some(1)).into())])))
        })
        .action("pos.config", "_set_default_pos_load_limit", |env, _, _| {
            let e = env.sudo();
            for (k, v) in [("point_of_sale.limited_product_count", "20000"), ("point_of_sale.limited_customer_count", "100")] {
                match find_one(&e, "ir.config_parameter", term("key", "=", k))? {
                    Some(p) if text(&rec(&e, "ir.config_parameter", p)?, "value").map_or(false, |v| !v.is_empty()) => {}
                    Some(p) => orm::write(&e, "ir.config_parameter", &[p], row(&[("value", v.into())]))?,
                    None => { orm::create(&e, "ir.config_parameter", row(&[("key", k.into()), ("value", v.into())]))?; }
                }
            }
            Ok(Value::Bool(true))
        })
}

fn last_closed(env: &Env, cfg: i64) -> Result<Option<Row>> {
    let e = env.sudo();
    let ids_ = orm::search(&e, "pos.session", &Domain::And(vec![term("config_id", "=", cfg), term("state", "=", "closed")]), Some("stop_at desc, id desc"), Some(1), 0)?;
    ids_.first().map(|i| rec(&e, "pos.session", *i)).transpose()
}
fn session_user(env: &Env, r: &Row) -> Result<Option<Row>> {
    Ok(open_sessions(env, cfg_id(r))?.into_iter().find(|s| !flag(s, "rescue") && matches!(text(s, "state").as_deref(), Some("opening_control" | "opened" | "closing_control"))))
}

/// `_get_descendants` over pos.category.parent_id.
fn category_descendants(env: &Env, roots: &[i64]) -> Result<Vec<i64>> {
    let mut all: Vec<i64> = roots.to_vec(); let mut frontier: Vec<i64> = roots.to_vec();
    while !frontier.is_empty() {
        let kids = orm::search(&env.sudo(), "pos.category", &term("parent_id", "in", id_list(&frontier)), None, None, 0)?;
        frontier = kids.into_iter().filter(|k| !all.contains(k)).collect(); all.extend(frontier.iter().copied());
    }
    Ok(all)
}
fn available_categories(env: &Env, cfg: &Row) -> Result<Vec<i64>> {
    let dom = if flag(cfg, "limit_categories") && !ids(cfg, "iface_available_categ_ids").is_empty() { term("id", "in", id_list(&category_descendants(env, &ids(cfg, "iface_available_categ_ids"))?)) } else { Domain::True };
    orm::search(&env.sudo(), "pos.category", &dom, Some("sequence"), None, 0)
}

fn open_session_action(env: &Env, cfg: i64, session: Option<i64>) -> Result<Value> {
    check_pricelists(env, &rec(env, "pos.config", cfg)?)?;   // the pricelist company might have changed after the first opening
    Ok(Value::Map(row(&[("name", "Session".into()), ("view_mode", "form,list".into()), ("res_model", "pos.session".into()), ("res_id", session.map_or(Value::Bool(false), Value::Int)), ("view_id", false.into()), ("type", "ir.actions.act_window".into())])))
}

fn check_before_new_session(env: &Env, cfg: i64) -> Result<()> {
    check_company_has_template(env, cfg)?;
    let r = rec(env, "pos.config", cfg)?;
    check_pricelists(env, &r)?; check_company_payment(env, &r)?; check_currencies(env, &r)?;
    check_profit_loss(env, cfg)?; check_payment_method_ids(env, cfg)
}
fn check_payment_method_ids(env: &Env, cfg: i64) -> Result<()> {
    if ids(&rec(env, "pos.config", cfg)?, "payment_method_ids").is_empty() { return invalid("You must have at least one payment method configured to launch a session."); }
    Ok(())
}
fn check_company_has_template(env: &Env, cfg: i64) -> Result<()> {
    let r = rec(env, "pos.config", cfg)?;
    let c = orm::read(env, "pos.config", &[cfg], &["company_has_template".into()])?;
    let _ = r;
    if !c.first().map_or(false, |x| flag(x, "company_has_template")) { return invalid("No chart of account configured, go to the \"configuration / settings\" menu, and install one from the Invoicing tab."); }
    Ok(())
}
fn check_profit_loss(env: &Env, cfg: i64) -> Result<()> {
    let r = rec(env, "pos.config", cfg)?;
    if cash_control_of(env, &r)? {
        for m in many(env, "pos.payment.method", &ids(&r, "payment_method_ids"))? {
            if flag(&m, "is_cash_count") {
                let j = id_of(&m, "journal_id").and_then(|j| rec(env, "account.journal", j).ok()).unwrap_or_default();
                if id_of(&j, "loss_account_id").is_none() || id_of(&j, "profit_account_id").is_none() { return invalid("You need a loss and profit account on your cash journal."); }
            }
        }
    }
    Ok(())
}
fn check_pricelists(env: &Env, cfg: &Row) -> Result<()> {
    check_companies(env, cfg)?;
    if let Some(pl) = id_of(cfg, "pricelist_id").and_then(|p| rec(env, "product.pricelist", p).ok()) {
        if let Some(c) = id_of(&pl, "company_id") { if Some(c) != id_of(cfg, "company_id") { return invalid("The default pricelist must belong to no company or the company of the point of sale."); } }
    }
    Ok(())
}
fn check_companies(env: &Env, cfg: &Row) -> Result<()> {
    for pl in many(env, "product.pricelist", &ids(cfg, "available_pricelist_ids"))? {
        if let Some(c) = id_of(&pl, "company_id") { if Some(c) != id_of(cfg, "company_id") { return invalid("The selected pricelists must belong to no company or the company of the point of sale."); } }
    }
    Ok(())
}
fn check_company_payment(env: &Env, cfg: &Row) -> Result<()> {
    for pm in many(env, "pos.payment.method", &ids(cfg, "payment_method_ids"))? {
        if id_of(&pm, "company_id") != id_of(cfg, "company_id") { return invalid(format!("The payment methods for the point of sale {} must belong to its company.", text(cfg, "name").unwrap_or_default())); }
    }
    Ok(())
}
fn check_currencies(env: &Env, cfg: &Row) -> Result<()> {
    if flag(cfg, "use_pricelist") {
        if let Some(p) = id_of(cfg, "pricelist_id") { if !ids(cfg, "available_pricelist_ids").contains(&p) { return invalid("The default pricelist must be included in the available pricelists."); } }
    }
    let cur = config_currency(env, cfg);
    for pm in many(env, "pos.payment.method", &ids(cfg, "payment_method_ids"))? {
        if let Some(jc) = id_of(&pm, "journal_id").and_then(|j| rec(env, "account.journal", j).ok()).and_then(|j| id_of(&j, "currency_id")) {
            if Some(jc) != cur { return invalid("All payment methods must be in the same currency as the Sales Journal or the company currency if that is not set."); }
        }
    }
    if let Some(ij) = id_of(cfg, "invoice_journal_id").and_then(|j| rec(env, "account.journal", j).ok()) {
        if let Some(c) = id_of(&ij, "currency_id") { if Some(c) != cur { return invalid("The invoice journal must be in the same currency as the Sales Journal or the company currency if that is not set."); } }
    }
    Ok(())
}
/// The constraints Odoo evaluates after every create/write of a config.
fn check_config(env: &Env, cfg: i64, vals: Option<&Row>) -> Result<()> {
    let r = rec(env, "pos.config", cfg)?;
    let touched = |ks: &[&str]| vals.map_or(true, |v| ks.iter().any(|k| v.contains_key(*k)));
    if touched(&["rounding_method"]) && flag(&r, "cash_rounding") {
        let strategy = id_of(&r, "rounding_method").and_then(|m| rec(env, "account.cash.rounding", m).ok()).and_then(|m| text(&m, "strategy"));
        if strategy.as_deref() != Some("add_invoice_line") { return invalid(format!("The cash rounding strategy of the point of sale {} must be: 'Add a rounding line'", text(&r, "name").unwrap_or_default())); }
    }
    if touched(&["company_id", "payment_method_ids"]) { check_company_payment(env, &r)?; }
    if touched(&["pricelist_id", "use_pricelist", "available_pricelist_ids", "journal_id", "invoice_journal_id", "payment_method_ids"]) { check_currencies(env, &r)?; }
    if touched(&["pricelist_id", "available_pricelist_ids", "company_id"]) { check_pricelists(env, &r)?; }
    if touched(&["payment_method_ids"]) {
        for pm in many(env, "pos.payment.method", &ids(&r, "payment_method_ids"))? {
            let Some(j) = id_of(&pm, "journal_id").and_then(|j| rec(env, "account.journal", j).ok()) else { continue };
            if text(&j, "type").as_deref() != Some("cash") { continue; }
            let pm_id = pm["id"].as_i64().unwrap();
            if !orm::search(&env.sudo(), "pos.config", &Domain::And(vec![term("id", "!=", cfg), term("payment_method_ids", "in", id_list(&[pm_id]))]), None, Some(1), 0)?.is_empty() {
                return invalid("This cash payment method is already used in another Point of Sale.\nA new cash payment method should be created for this Point of Sale.");
            }
            if orm::search(&env.sudo(), "pos.payment.method", &term("journal_id", "=", j["id"].as_i64().unwrap()), None, Some(2), 0)?.len() > 1 { return invalid("You cannot use the same journal on multiples cash payment methods."); }
        }
    }
    if touched(&["trusted_config_ids"]) {
        let cur = config_currency(env, &r);
        for t in many(env, "pos.config", &ids(&r, "trusted_config_ids"))? { if config_currency(env, &t) != cur { return invalid("You cannot share open orders with configuration that does not use the same currency."); } }
    }
    if touched(&["customer_display_type", "proxy_ip", "is_posbox"]) && text(&r, "customer_display_type").as_deref() == Some("proxy") && (!flag(&r, "is_posbox") || !flag(&r, "proxy_ip")) {
        return user_err("You must set the iot box's IP address to use an IoT-connected screen. You'll find the field under the 'IoT Box' option.");
    }
    Ok(())
}
/// `write`: while a session is open the structural settings of the config are frozen.
fn forbid_changes_while_open(env: &Env, ids_: &[i64], vals: &Row) -> Result<()> {
    const FORBIDDEN: [&str; 9] = ["module_pos_hr", "module_pos_restaurant", "available_pricelist_ids", "limit_categories", "iface_available_categ_ids", "use_pricelist", "module_pos_discount", "payment_method_ids", "iface_tipproduc"];
    let mut any_open = false;
    for i in ids_ { if !open_sessions(env, *i)?.is_empty() { any_open = true; } }
    if !any_open { return Ok(()); }
    let bypass_cat = flag_ctx(env, "bypass_categories_forbidden_change"); let bypass_pm = flag_ctx(env, "bypass_payment_method_ids_forbidden_change");
    let mut hit = vec![];
    for k in FORBIDDEN {
        if !vals.contains_key(k) { continue; }
        if bypass_cat && matches!(k, "limit_categories" | "iface_available_categ_ids") { continue; }
        if bypass_pm && k == "payment_method_ids" { continue; }
        if k == "use_pricelist" && flag(vals, k) { continue; }
        if k == "available_pricelist_ids" { continue; }   // whether a pricelist is being removed cannot be told after the write
        hit.push(env.reg.field("pos.config", k).map(|f| f.label(k)).unwrap_or_else(|_| k.to_string()));
    }
    if hit.is_empty() { return Ok(()); }
    user_err(format!("Unable to modify this PoS Configuration because you can't modify {} while a session is open.", hit.join(", ")))
}
fn flag_ctx(env: &Env, k: &str) -> bool { env.ctx.get(k).map_or(false, |v| v.truthy()) }
/// `_set_fiscal_position`: the default fiscal position is always among the selectable ones, and none without the tax regime option.
fn set_fiscal_position(env: &Env, ids_: &[i64]) -> Result<()> {
    let e = env.sudo();
    for i in ids_ {
        let c = rec(&e, "pos.config", *i)?; let fp = ids(&c, "fiscal_position_ids");
        if flag(&c, "tax_regime_selection") {
            if let Some(d) = id_of(&c, "default_fiscal_position_id") { if !fp.contains(&d) { orm::write(&e, "pos.config", &[*i], row(&[("fiscal_position_ids", Value::List(vec![Value::List(vec![4.into(), d.into()])]))]))?; } }
        } else if !fp.is_empty() { orm::write(&e, "pos.config", &[*i], row(&[("fiscal_position_ids", Value::List(vec![Value::List(vec![5.into()])]))]))?; }
    }
    Ok(())
}

fn create_cash_payment_method(env: &Env, kw: &Row) -> Result<i64> {
    let e = env.sudo(); let company = company_of(&e, &Row::new()).unwrap_or(1);
    let mut jv = row(&[("name", "Cash".into()), ("type", "cash".into()), ("company_id", company.into())]);
    if let Some(Value::Map(extra)) = kw.get("cash_journal_vals") { for (k, v) in extra { jv.insert(k.clone(), v.clone()); } }
    if let Some(a) = find_one(&e, "account.account", Domain::And(vec![term("account_type", "=", "asset_cash"), term("name", "=", "Cash")]))? { jv.entry("default_account_id".into()).or_insert(a.into()); }
    let journal = orm::create(&e, "account.journal", jv)?;
    orm::create(&e, "pos.payment.method", row(&[("name", "Cash".into()), ("journal_id", journal.into()), ("company_id", company.into())]))
}
/// A cash method of its own, plus the company's shared Card (bank) and Customer Account methods.
fn create_journal_and_payment_methods(env: &Env, kw: &Row) -> Result<(i64, Vec<i64>)> {
    let e = env.sudo(); let company = company_of(&e, &Row::new()).unwrap_or(1);
    let journal = find_or_create(&e, "account.journal", Domain::And(vec![term("code", "=", "POSS"), term("company_id", "=", company)]), row(&[("name", "Point of Sale".into()), ("code", "POSS".into()), ("type", "general".into()), ("company_id", company.into())]))?;
    let mut pms = vec![create_cash_payment_method(&e, kw)?];
    let bank_pm = orm::search(&e, "pos.payment.method", &Domain::And(vec![term("journal_id.type", "=", "bank"), term("company_id", "=", company)]), None, None, 0)?;
    if bank_pm.is_empty() {
        let bj = find_one(&e, "account.journal", Domain::And(vec![term("type", "=", "bank"), term("company_id", "=", company)]))?.ok_or_else(|| OdooError::User("Ensure that there is an existing bank journal. Check if chart of accounts is installed in your company.".into()))?;
        pms.push(orm::create(&e, "pos.payment.method", row(&[("name", "Card".into()), ("journal_id", bj.into()), ("company_id", company.into()), ("sequence", 1.into())]))?);
    } else { pms.extend(bank_pm); }
    let later = orm::search(&e, "pos.payment.method", &Domain::And(vec![term("journal_id", "=", Value::Bool(false)), term("company_id", "=", company), term("split_transactions", "=", true)]), None, None, 0)?;
    if later.is_empty() { pms.push(orm::create(&e, "pos.payment.method", row(&[("name", "Customer Account".into()), ("company_id", company.into()), ("split_transactions", true.into()), ("sequence", 2.into())]))?); } else { pms.extend(later); }
    Ok((journal, pms))
}

// ---------------------------------------------------------------------------------------------------------------------
// pos.payment.method
// ---------------------------------------------------------------------------------------------------------------------
/// `_compute_type`: the journal's type for cash/bank journals, otherwise pay later.
pub(crate) fn pm_type(env: &Env, r: &Row) -> String {
    match id_of(r, "journal_id").and_then(|j| rec(env, "account.journal", j).ok()).and_then(|j| text(&j, "type")) {
        Some(t) if t == "cash" || t == "bank" => t,
        Some(_) => "pay_later".into(),
        None => if flag(r, "is_cash_count") { "cash".into() } else { "pay_later".into() },   // journal-less method: honour the explicit cash flag
    }
}
pub(crate) fn pm_open_sessions(env: &Env, pm: &Row) -> Result<Vec<Row>> {
    let cfgs = ids(pm, "config_ids"); if cfgs.is_empty() { return Ok(vec![]); }
    let e = env.sudo();
    orm::search(&e, "pos.session", &Domain::And(vec![term("config_id", "in", id_list(&cfgs)), term("state", "!=", "closed")]), None, None, 0)?.into_iter().map(|i| rec(&e, "pos.session", i)).collect()
}
fn force_type_values(vals: &mut Row, ptype: &str, if_present: bool) {
    let disabled: &[&str] = match ptype { "terminal" => &["qr_code_method"], "qr_code" => &["use_payment_terminal"], _ => &["use_payment_terminal", "qr_code_method"] };
    for k in disabled { if !if_present || vals.contains_key(*k) { vals.insert((*k).into(), Value::Bool(false)); } }
}

fn payment_method_rules() -> Rules {
    Rules::default()
        .compute("pos.payment.method", "type", |env, r| Ok(Value::Text(pm_type(env, r))))
        .compute("pos.payment.method", "is_cash_count", |env, r| Ok((pm_type(env, r) == "cash").into()))
        .compute("pos.payment.method", "hide_qr_code_method", |env, r| {
            // only one QR format available means nothing to choose; this port has no QR format registry, so the selection decides
            let n = env.reg.field("pos.payment.method", "qr_code_method").map(|f| f.selection_values().len()).unwrap_or(0);
            Ok((text(r, "payment_method_type").as_deref() != Some("qr_code") || n == 1).into())
        })
        .compute("pos.payment.method", "open_session_ids", |env, r| Ok(id_list(&pm_open_sessions(env, r)?.iter().filter_map(|s| s["id"].as_i64()).collect::<Vec<_>>())))
        .before_create("pos.payment.method", |_, mut v| {
            // the ORM fills the default type ('none') before hooks run, so only an explicit terminal/QR type can be told apart
            if let Some(t) = text(&v, "payment_method_type").filter(|t| t == "terminal" || t == "qr_code") { force_type_values(&mut v, &t, false); }
            Ok(v)
        })
        .before_write("pos.payment.method", |_, mut v| {
            if let Some(t) = text(&v, "payment_method_type") { force_type_values(&mut v, &t, false); }
            Ok(v)
        })
        .after_write("pos.payment.method", |env, ids_, vals| {
            let e = env.sudo();
            // _is_write_forbidden: only the sequence may change while a session using the method is open
            if vals.keys().any(|k| k != "sequence") {
                let mut open = vec![];
                for i in ids_ { for s in pm_open_sessions(&e, &rec(&e, "pos.payment.method", *i)?)? { if let Some(n) = text(&s, "name") { if !open.contains(&n) { open.push(n); } } } }
                if !open.is_empty() { return user_err(format!("Please close and validate the following open PoS Sessions before modifying this payment method.\nOpen sessions: {}", open.join(" "))); }
            }
            // per-record forced values: a terminal method has no QR format, a QR one no terminal
            if !vals.contains_key("payment_method_type") {
                for i in ids_ {
                    let pm = rec(&e, "pos.payment.method", *i)?; let t = text(&pm, "payment_method_type").unwrap_or_default();
                    let mut fix = Row::new();
                    if t == "terminal" && flag(vals, "qr_code_method") { fix.insert("qr_code_method".into(), false.into()); }
                    if t == "qr_code" && flag(vals, "use_payment_terminal") { fix.insert("use_payment_terminal".into(), false.into()); }
                    if !fix.is_empty() { orm::write(&e, "pos.payment.method", &[*i], fix)?; }
                }
            }
            Ok(())
        })
        .after_create("pos.payment.method", |env, ids_, _| { for i in ids_ { check_payment_method(env, *i)?; } Ok(()) })
        .action("pos.payment.method", "_is_write_forbidden", |env, ids_, kw| {
            let names: Vec<String> = match kw.get("fields") { Some(Value::List(l)) => l.iter().filter_map(|v| v.as_str().map(String::from)).collect(), _ => vec![] };
            if !names.iter().any(|n| n != "sequence") { return Ok(false.into()); }
            for i in ids_ { if !pm_open_sessions(env, &rec(env, "pos.payment.method", *i)?)?.is_empty() { return Ok(true.into()); } }
            Ok(false.into())
        })
        .action("pos.payment.method", "_check_payment_method", |env, ids_, _| { for i in ids_ { check_payment_method(env, *i)?; } Ok(Value::Bool(true)) })
}
fn check_payment_method(env: &Env, id: i64) -> Result<()> {
    let r = rec(env, "pos.payment.method", id)?;
    if text(&r, "payment_method_type").as_deref() == Some("qr_code") {
        let j = id_of(&r, "journal_id").and_then(|j| rec(env, "account.journal", j).ok()).unwrap_or_default();
        if text(&j, "type").as_deref() != Some("bank") || id_of(&j, "bank_account_id").is_none() { return invalid("At least one bank account must be defined on the journal to allow registering QR code payments with Bank apps."); }
        if !flag(&r, "qr_code_method") { return invalid("You must select a QR-code method to generate QR-codes for this payment method."); }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------------------
// pos.payment
// ---------------------------------------------------------------------------------------------------------------------
pub(crate) fn check_payment_method_allowed(env: &Env, pay: &Row) -> Result<()> {
    let Some(order) = id_of(pay, "pos_order_id").and_then(|o| rec(env, "pos.order", o).ok()) else { return Ok(()) };
    let Some(cfg) = id_of(&order, "session_id").and_then(|s| rec(env, "pos.session", s).ok()).and_then(|s| id_of(&s, "config_id")).and_then(|c| rec(env, "pos.config", c).ok()) else { return Ok(()) };
    if let Some(m) = id_of(pay, "payment_method_id") { if !ids(&cfg, "payment_method_ids").contains(&m) { return invalid("The payment method selected is not allowed in the config of the POS session."); } }
    Ok(())
}
fn payment_rules() -> Rules {
    Rules::default()
        .after_write("pos.payment", |env, ids_, vals| {
            for i in ids_ {
                let p = rec(env, "pos.payment", *i)?;
                if vals.contains_key("amount") {
                    let st = id_of(&p, "pos_order_id").and_then(|o| rec(env, "pos.order", o).ok()).and_then(|o| text(&o, "state"));
                    if matches!(st.as_deref(), Some("invoiced" | "done")) { return invalid("You cannot edit a payment for a posted order."); }
                }
                if vals.contains_key("payment_method_id") { check_payment_method_allowed(env, &p)?; }
            }
            Ok(())
        })
        .action("pos.payment", "_check_amount", |env, ids_, _| {
            for i in ids_ { let st = id_of(&rec(env, "pos.payment", *i)?, "pos_order_id").and_then(|o| rec(env, "pos.order", o).ok()).and_then(|o| text(&o, "state")); if matches!(st.as_deref(), Some("invoiced" | "done")) { return invalid("You cannot edit a payment for a posted order."); } }
            Ok(Value::Bool(true))
        })
        .action("pos.payment", "_check_payment_method_id", |env, ids_, _| { for i in ids_ { check_payment_method_allowed(env, &rec(env, "pos.payment", *i)?)?; } Ok(Value::Bool(true)) })
}
