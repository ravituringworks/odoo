//! pos_self_order and pos_discount back-office rules (`addons/pos_self_order/models`, `addons/pos_discount/models`):
//! self-order URLs and status, mode consistency on write, table security tokens, custom links, category availability hours,
//! the kiosk/mobile defaults, and the discount product of a register.
//!
//! The constraint "the self-order default user must be a POS user" is available as `_check_default_user` but is not run on
//! every write (the terminal runtime creates self-order registers without a default user). QR code images are not generated.
use crate::pos_methods::{flag, id_list, invalid, many, user_err, xmlid};
use crate::util::*;
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, OdooError, Result, Row, Rules, Value};

fn has_self_order(env: &Env) -> bool { env.reg.field("pos.config", "self_ordering_mode").is_ok() }
fn first(ids_: &[i64]) -> Result<i64> { ids_.first().copied().ok_or_else(|| OdooError::User("Select a record".into())) }
fn identifier() -> String { crate::pos_order_methods::new_uuid().replace('-', "")[..8].to_string() }
fn base_url(env: &Env) -> String {
    let e = env.sudo();
    find_one(&e, "ir.config_parameter", term("key", "=", "web.base.url")).ok().flatten().and_then(|p| rec(&e, "ir.config_parameter", p).ok()).and_then(|p| text(&p, "value")).filter(|u| !u.is_empty()).unwrap_or_else(|| "http://localhost:8069".into())
}
/// werkzeug's `url_quote` with its default safe characters (`/` and `:`).
fn url_quote(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() { match b { b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => out.push(b as char), _ => out.push_str(&format!("%{b:02X}")) } }
    out
}
/// `_get_self_order_route`.
fn route(env: &Env, cfg: &Row, table: Option<i64>) -> Result<String> {
    let base = format!("/pos-self/{}", cfg["id"].as_i64().unwrap());
    match text(cfg, "self_ordering_mode").as_deref() {
        Some("consultation") => return Ok(base),
        Some("mobile") => {
            if let Some(t) = table.and_then(|t| rec(env, "restaurant.table", t).ok()).filter(|t| t.get("active").map_or(true, |a| a.truthy())) {
                return Ok(format!("{base}?access_token={}&table_identifier={}", text(cfg, "access_token").unwrap_or_default(), text(&t, "identifier").unwrap_or_default()));
            }
        }
        _ => {}
    }
    Ok(format!("{base}?access_token={}", text(cfg, "access_token").unwrap_or_default()))
}
fn self_order_url(env: &Env, cfg: &Row, table: Option<i64>) -> Result<String> { Ok(url_quote(&(base_url(env) + &route(env, cfg, table)?))) }
fn check_default_user(env: &Env, cfg: &Row) -> Result<()> {
    if text(cfg, "self_ordering_mode").as_deref() == Some("nothing") { return Ok(()); }
    let e = env.sudo();
    let ok = id_of(cfg, "self_ordering_default_user_id").and_then(|u| rec(&e, "res.users", u).ok()).map_or(false, |u| {
        let groups = ids(&u, "groups_id");
        ["group_pos_user", "group_pos_manager"].iter().filter_map(|g| xmlid(&e, "point_of_sale", g)).any(|g| groups.contains(&g))
    });
    if ok { Ok(()) } else { user_err("The Self-Order default user must be a POS user") }
}
fn check_kiosk_payment_methods(env: &Env, cfg: &Row) -> Result<()> {
    if text(cfg, "self_ordering_mode").as_deref() == Some("kiosk") && many(env, "pos.payment.method", &ids(cfg, "payment_method_ids"))?.iter().any(|p| flag(p, "is_cash_count")) { return invalid("You cannot add cash payment methods in kiosk mode."); }
    Ok(())
}
/// The "Order Now" link every self-order register shows.
fn custom_button(env: &Env, cfg: i64) -> Result<()> {
    if env.reg.model("pos_self_order.custom_link").is_err() { return Ok(()); }
    let e = env.sudo(); let url = format!("/pos-self/{cfg}/products");
    if orm::search(&e, "pos_self_order.custom_link", &Domain::And(vec![term("pos_config_ids", "in", id_list(&[cfg])), term("url", "=", url.as_str())]), None, Some(1), 0)?.is_empty() {
        orm::create(&e, "pos_self_order.custom_link", row(&[("name", "Order Now".into()), ("url", url.as_str().into()), ("pos_config_ids", Value::List(vec![Value::List(vec![4.into(), cfg.into()])]))]))?;
    }
    Ok(())
}

pub fn rules() -> Rules {
    Rules::default()
        .compute("pos.config", "status", |env, r| {
            let active = !orm::search(&env.sudo(), "pos.session", &Domain::And(vec![term("config_id", "=", r["id"].as_i64().unwrap_or(0)), term("state", "!=", "closed")]), None, Some(1), 0)?.is_empty();
            Ok(Value::Text(if active { "active" } else { "inactive" }.into()))
        })
        .compute("pos.config", "self_ordering_url", |env, r| Ok(Value::Text(base_url(env) + &route(env, r, None)?)))
        .after_create("pos.config", |env, ids_, _| {
            if !has_self_order(env) { return Ok(()); }
            for i in ids_ { let c = rec(env, "pos.config", *i)?; check_kiosk_payment_methods(env, &c)?; if text(&c, "self_ordering_mode").as_deref().map_or(false, |m| m != "nothing") { custom_button(env, *i)?; } }
            Ok(())
        })
        .after_write("pos.config", |env, ids_, vals| {
            if !has_self_order(env) { return Ok(()); }
            let e = env.sudo();
            for i in ids_ {
                let c = rec(&e, "pos.config", *i)?;
                if vals.contains_key("payment_method_ids") || vals.contains_key("self_ordering_mode") { check_kiosk_payment_methods(&e, &c)?; }
                // consistency of the ordering options, in the order Odoo applies them
                let mode = text(vals, "self_ordering_mode"); let mut pay_after = text(vals, "self_ordering_pay_after"); let mut service = text(vals, "self_ordering_service_mode");
                let restaurant = flag(&c, "module_pos_restaurant");
                if mode.as_deref() == Some("kiosk") { pay_after = Some("each".into()); }
                if !flag(vals, "module_pos_restaurant") && !restaurant && mode.as_deref() == Some("mobile") { pay_after = Some("each".into()); }
                let counter = service.as_deref().map_or(text(&c, "self_ordering_service_mode").as_deref() == Some("counter"), |s| s == "counter");
                if counter && mode.as_deref() == Some("mobile") { pay_after = Some("each".into()); }
                if mode.as_deref() == Some("mobile") && pay_after.as_deref() == Some("meal") { service = Some("table".into()); }
                let mut fix = Row::new();
                if let Some(p) = pay_after { if text(&c, "self_ordering_pay_after").as_deref() != Some(p.as_str()) { fix.insert("self_ordering_pay_after".into(), p.into()); } }
                if let Some(s) = service { if text(&c, "self_ordering_service_mode").as_deref() != Some(s.as_str()) { fix.insert("self_ordering_service_mode".into(), s.into()); } }
                if !fix.is_empty() { orm::write(&e, "pos.config", &[*i], fix)?; }
                if text(&c, "self_ordering_mode").as_deref().map_or(false, |m| m != "nothing") { custom_button(&e, *i)?; }
            }
            Ok(())
        })
        .action("pos.config", "_check_default_user", |env, ids_, _| { for i in ids_ { check_default_user(env, &rec(env, "pos.config", *i)?)?; } Ok(Value::Bool(true)) })
        .action("pos.config", "_get_self_order_route", |env, ids_, kw| Ok(Value::Text(route(env, &rec(env, "pos.config", first(ids_)?)?, kw.get("table_id").and_then(|v| v.as_i64()))?)))
        .action("pos.config", "_get_self_order_url", |env, ids_, kw| Ok(Value::Text(self_order_url(env, &rec(env, "pos.config", first(ids_)?)?, kw.get("table_id").and_then(|v| v.as_i64()))?)))
        .action("pos.config", "get_kiosk_url", |env, ids_, _| Ok(Value::Text(base_url(env) + &route(env, &rec(env, "pos.config", first(ids_)?)?, None)?)))
        .action("pos.config", "preview_self_order_app", |env, ids_, _| Ok(Value::Map(row(&[("type", "ir.actions.act_url".into()), ("url", route(env, &rec(env, "pos.config", first(ids_)?)?, None)?.into()), ("target", "new".into())]))))
        .action("pos.config", "_prepare_self_order_custom_btn", |env, ids_, _| { for i in ids_ { custom_button(env, *i)?; } Ok(Value::Null) })
        // new token for the register and for every table (so printed QR codes stop working)
        .action("pos.config", "_update_access_token", |env, ids_, _| {
            let e = env.sudo();
            for i in ids_ {
                orm::write(&e, "pos.config", &[*i], row(&[("access_token", crate::pos_order_methods::new_uuid().replace('-', "")[..16].into())]))?;
                if e.reg.model("restaurant.table").is_ok() { orm::call(&e, "restaurant.table", "_update_identifier", &[], &Row::new())?; }
            }
            Ok(Value::Null)
        })
        .action("pos.config", "_get_qr_code_data", |env, ids_, _| {
            let e = env.sudo(); let cfg = rec(&e, "pos.config", first(ids_)?)?; let mut out = vec![];
            if text(&cfg, "self_ordering_mode").as_deref() == Some("mobile") && flag(&cfg, "module_pos_restaurant") && text(&cfg, "self_ordering_service_mode").as_deref() == Some("table") {
                for f in many(&e, "restaurant.floor", &ids(&cfg, "floor_ids"))? {
                    let mut tables = vec![];
                    for t in children(&e, "restaurant.table", "floor_id", f["id"].as_i64().unwrap())? {
                        if !t.get("active").map_or(true, |a| a.truthy()) { continue; }
                        tables.push(Value::Map(row(&[("identifier", t.get("identifier").cloned().unwrap_or(Value::Null)), ("id", t["id"].clone()), ("name", t.get("table_number").cloned().unwrap_or(Value::Null)), ("url", self_order_url(&e, &cfg, t["id"].as_i64())?.into())])));
                    }
                    out.push(Value::Map(row(&[("name", f.get("name").cloned().unwrap_or(Value::Null)), ("type", "table".into()), ("tables", Value::List(tables))])));
                }
            } else {
                // a generic code, printed six times on a page
                let url = self_order_url(&e, &cfg, None)?;
                out.push(Value::Map(row(&[("name", "Generic".into()), ("type", "default".into()), ("tables", Value::List((0..6).map(|i| Value::Map(row(&[("id", (i as i64).into()), ("url", url.as_str().into())]))).collect()))])));
            }
            Ok(Value::List(out))
        })
        .action("pos.config", "_split_qr_codes_list", |_, _, kw| {
            let cols = kw.get("cols").and_then(|v| v.as_i64()).unwrap_or(1).max(1) as usize;
            let floors = match kw.get("floors") { Some(Value::List(l)) => l.clone(), _ => vec![] };
            Ok(Value::List(floors.into_iter().map(|f| { let f = match f { Value::Map(m) => m, _ => Row::new() }; let tables = match f.get("tables") { Some(Value::List(t)) => t.clone(), _ => vec![] };
                Value::Map(row(&[("name", f.get("name").cloned().unwrap_or(Value::Null)), ("rows_of_tables", Value::List(tables.chunks(cols).map(|c| Value::List(c.to_vec())).collect()))])) }).collect()))
        })
        // opening the kiosk starts an (empty) session; closing it drops unpaid orders and starts the closing control
        .action("pos.config", "action_open_wizard", |env, ids_, _| {
            let e = env.sudo(); let cfg = first(ids_)?;
            if crate::pos_methods::current_session(&e, cfg)?.is_none() {
                orm::call(&e, "pos.config", "_check_before_creating_new_session", &[cfg], &Row::new())?;
                let s = orm::create(&e, "pos.session", row(&[("user_id", env.uid.into()), ("config_id", cfg.into())]))?;
                orm::call(&e, "pos.session", "set_opening_control", &[s], &row(&[("cashbox_value", 0.0.into()), ("notes", "".into())]))?;
            }
            Ok(Value::Map(row(&[("res_model", "pos.config".into()), ("type", "ir.actions.client".into()), ("tag", "install_kiosk_pwa".into()), ("target", "new".into())])))
        })
        .action("pos.config", "action_close_kiosk_session", |env, ids_, _| {
            let e = env.sudo(); let cfg = first(ids_)?;
            let Some(s) = crate::pos_methods::current_session(&e, cfg)? else { return user_err("There is no open session to close") };
            let sid = s["id"].as_i64().unwrap();
            for o in children(&e, "pos.order", "session_id", sid)? { if !matches!(text(&o, "state").as_deref(), Some("paid" | "invoiced")) { orm::unlink(&e, "pos.order", &[o["id"].as_i64().unwrap()])?; } }
            orm::call(&e, "pos.session", "action_pos_session_closing_control", &[sid], &Row::new())
        })
        // ---- tables ----
        .before_create("restaurant.table", |env, mut v| { if env.reg.field("restaurant.table", "identifier").is_ok() && !flag(&v, "identifier") { v.insert("identifier".into(), identifier().into()); } Ok(v) })
        .action("restaurant.table", "_update_identifier", |env, _, _| {
            let e = env.sudo();
            for t in orm::search(&e.with_ctx("active_test", Value::Bool(false)), "restaurant.table", &Domain::True, None, None, 0)? { orm::write(&e, "restaurant.table", &[t], row(&[("identifier", identifier().into())]))?; }
            Ok(Value::Null)
        })
        // ---- order lines: a combo child can name its parent by uuid ----
        .before_create("pos.order.line", |env, mut v| { resolve_combo_parent(env, &mut v)?; Ok(v) })
        .before_write("pos.order.line", |env, mut v| { resolve_combo_parent(env, &mut v)?; Ok(v) })
        // ---- custom links and availability hours ----
        .compute("pos_self_order.custom_link", "link_html", |_, r| {
            let Some(name) = text(r, "name").filter(|n| !n.is_empty()) else { return Ok(Value::Null) };
            let esc = name.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&#34;").replace('\'', "&#39;");
            Ok(Value::Text(format!("<a class=\"btn btn-{} w-100\">{}</a>", text(r, "style").unwrap_or_else(|| "primary".into()), esc)))
        })
        .after_create("pos.category", |env, ids_, _| { for i in ids_ { check_hours(env, *i)?; } Ok(()) })
        .after_write("pos.category", |env, ids_, vals| { if vals.contains_key("hour_until") || vals.contains_key("hour_after") { for i in ids_ { check_hours(env, *i)?; } } Ok(()) })
        .action("pos.category", "_check_hour", |env, ids_, _| { for i in ids_ { check_hours(env, *i)?; } Ok(Value::Bool(true)) })
        .action("pos.session", "_create_pos_self_sessions_sequence", |env, ids_, _| { for i in ids_ { session_sequence(env, *i)?; } Ok(Value::Null) })
        .after_create("pos.session", |env, ids_, _| { if has_self_order(env) { for i in ids_ { session_sequence(env, *i)?; } } Ok(()) })
        // ---- pos_discount ----
        // the global discount needs its product before a first session opens; everything else is the terminal runtime's open_ui
        .action("pos.config", "open_ui", |env, ids_, kw| {
            let e = env.sudo();
            if e.reg.field("pos.config", "discount_product_id").is_ok() {
                for i in ids_ {
                    let c = rec(&e, "pos.config", *i)?;
                    if crate::pos_methods::current_session(&e, *i)?.is_none() && flag(&c, "module_pos_discount") && id_of(&c, "discount_product_id").is_none() {
                        return user_err("A discount product is needed to use the Global Discount feature. Go to Point of Sale > Configuration > Settings to set it.");
                    }
                }
            }
            let rt = crate::pos::runtime_rules();
            match rt.actions.get(&("pos.config".to_string(), "open_ui".to_string())) { Some(f) => f(env, ids_, kw), None => user_err("open_ui is not available") }
        })
        .action("pos.config", "_default_discount_value_on_module_install", |env, _, _| {
            // registers without an open session get the default discount product when the discount module is on
            let e = env.sudo(); if e.reg.field("pos.config", "discount_product_id").is_err() { return Ok(Value::Null); }
            let product = xmlid(&e, "pos_discount", "product_product_consumable");
            let open: Vec<i64> = orm::search_read(&e, "pos.session", &Domain::Or(vec![term("state", "!=", "closed"), term("rescue", "=", true)]), &["config_id".into()], None, None, 0)?.iter().filter_map(|s| id_of(s, "config_id")).collect();
            for c in orm::search(&e, "pos.config", &Domain::True, None, None, 0)? {
                if open.contains(&c) { continue; }
                let cfg = rec(&e, "pos.config", c)?;
                let usable = product.filter(|p| flag(&cfg, "module_pos_discount") && rec(&e, "product.product", *p).map_or(false, |r| id_of(&r, "company_id").map_or(true, |co| Some(co) == id_of(&cfg, "company_id"))));
                orm::write(&e, "pos.config", &[c], row(&[("discount_product_id", usable.map_or(Value::Null, Value::Int))]))?;
            }
            Ok(Value::Null)
        })
        .compute("res.config.settings", "pos_discount_product_id", |env, r| {
            let e = env.sudo(); let default = xmlid(&e, "pos_discount", "product_product_consumable");
            let cfg = id_of(r, "pos_config_id").and_then(|c| rec(&e, "pos.config", c).ok());
            let discount = cfg.as_ref().and_then(|c| id_of(c, "discount_product_id")).or(default);
            let on = cfg.as_ref().map_or(false, |c| flag(c, "module_pos_discount"));
            let company = cfg.as_ref().and_then(|c| id_of(c, "company_id"));
            let ok = discount.filter(|d| on && rec(&e, "product.product", *d).map_or(false, |p| id_of(&p, "company_id").map_or(true, |co| Some(co) == company)));
            Ok(ok.map_or(Value::Null, Value::Int))
        })
}
fn session_sequence(env: &Env, sid: i64) -> Result<()> {
    let e = env.sudo(); let code = format!("pos.order_{sid}");
    if find_one(&e, "ir.sequence", term("code", "=", code.as_str()))?.is_none() {
        orm::create(&e, "ir.sequence", row(&[("name", "PoS Order by Session".into()), ("padding", 4.into()), ("code", code.as_str().into()), ("number_next", 1.into()), ("number_increment", 1.into())]))?;
    }
    Ok(())
}
fn resolve_combo_parent(env: &Env, v: &mut Row) -> Result<()> {
    if let Some(u) = v.remove("combo_parent_uuid") {
        if let Some(u) = u.as_str().filter(|u| !u.is_empty()) {
            if let Some(p) = find_one(&env.sudo(), "pos.order.line", term("uuid", "=", u))? { v.insert("combo_parent_id".into(), p.into()); }
        }
    }
    Ok(())
}
fn check_hours(env: &Env, id: i64) -> Result<()> {
    let c = rec(env, "pos.category", id)?; let (until, after) = (num(&c, "hour_until"), num(&c, "hour_after"));
    if until != 0.0 && !(0.0..=24.0).contains(&until) { return invalid("The Availability Until must be set between 00:00 and 24:00"); }
    if after != 0.0 && !(0.0..=24.0).contains(&after) { return invalid("The Availability After must be set between 00:00 and 24:00"); }
    if until != 0.0 && after != 0.0 && until < after { return invalid("The Availability Until must be greater than Availability After."); }
    Ok(())
}
