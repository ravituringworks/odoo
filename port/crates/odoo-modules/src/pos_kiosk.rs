//! pos_self_order: public QR-menu / mobile ordering / kiosk. Every action checks the register's access token itself,
//! because these run for anonymous visitors. Orders arrive as open orders (drafts) + kitchen tickets for the staff.
use crate::{pos_restaurant, tax, util::*};
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, OdooError, Result, Row, Rules, Value};

const PREFIX: &str = "kiosk-";
fn flag(r: &Row, k: &str) -> bool { r.get(k).map_or(false, |v| v.truthy()) }
fn map(pairs: Vec<(&str, Value)>) -> Value { Value::Map(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()) }
fn fresh_token() -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new(); std::time::SystemTime::now().hash(&mut h); std::process::id().hash(&mut h);
    let a = h.finish(); a.hash(&mut h); format!("{:016x}{:016x}", a, h.finish())
}
fn ct_eq(a: &str, b: &str) -> bool { a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0 }

/// The register, if the token is right and self-ordering is switched on.
fn gate(env: &Env, config: i64, kw: &Row) -> Result<Row> {
    let cfg = rec(env, "pos.config", config).map_err(|_| OdooError::User("Unknown register".into()))?;
    let mode = text(&cfg, "self_ordering_mode").unwrap_or_default();
    let tok = text(&cfg, "access_token").unwrap_or_default();
    if tok.is_empty() || !ct_eq(&tok, &text(kw, "access_token").unwrap_or_default()) || !matches!(mode.as_str(), "mobile" | "kiosk" | "consultation") { return Err(OdooError::User("Self-ordering is not available".into())); }
    Ok(cfg)
}
fn open_session(env: &Env, config: i64) -> Result<i64> {
    find_one(env, "pos.session", Domain::And(vec![term("config_id", "=", config), term("state", "=", "opened")]))?.ok_or_else(|| OdooError::User("We are closed right now — please see the staff.".into()))
}

pub fn rules() -> Rules {
    Rules::default()
        // Odoo's `default=lambda: uuid4()/secrets` tokens
        .default_for("restaurant.table", "identifier", |_| Ok(Value::Text(fresh_token()[..16].to_string())))
        .default_for("pos.config", "access_token", |_| Ok(Value::Text(fresh_token())))
        // staff: make sure the register has a token and return the public link parts
        .action("pos.config", "kiosk_link", |env, ids, _| {
            let e = env.sudo(); let id = *ids.first().ok_or_else(|| OdooError::User("Select a register".into()))?;
            let cfg = rec(&e, "pos.config", id)?;
            let tok = match text(&cfg, "access_token").filter(|t| !t.is_empty()) { Some(t) => t, None => { let t = fresh_token(); orm::write(&e, "pos.config", &[id], row(&[("access_token", t.clone().into())]))?; t } };
            Ok(map(vec![("config", id.into()), ("token", tok.into()), ("mode", text(&cfg, "self_ordering_mode").unwrap_or_default().into())]))
        })
        .action("pos.config", "kiosk_menu", |env, ids, kw| {
            let e = env.sudo(); let id = *ids.first().unwrap_or(&0); let cfg = gate(&e, id, kw)?;
            let sess = open_session(&e, id).ok();
            let mut dom = vec![term("available_in_pos", "=", true)];
            let limit = ids_of(&cfg, "iface_available_categ_ids");
            if flag(&cfg, "limit_categories") && !limit.is_empty() { dom.push(term("pos_categ_ids", "in", Value::List(limit.iter().map(|i| Value::Int(*i)).collect()))); }
            let tmpl = orm::search_read(&e, "product.template", &Domain::And(dom), &["id".into(), "name".into(), "list_price".into(), "taxes_id".into(), "pos_categ_ids".into(), "public_description".into(), "description_sale".into()], Some("name"), Some(300), 0)?;
            let tids: Vec<i64> = tmpl.iter().filter_map(|t| t["id"].as_i64()).collect();
            let prods = if tids.is_empty() { vec![] } else { orm::search_read(&e, "product.product", &term("product_tmpl_id", "in", Value::List(tids.iter().map(|i| Value::Int(*i)).collect())), &["id".into(), "product_tmpl_id".into()], None, None, 0)? };
            let products: Vec<Value> = tmpl.iter().filter_map(|t| {
                let p = prods.iter().find(|p| id_of(p, "product_tmpl_id") == t["id"].as_i64())?;
                let price = tax::load(&e, &ids_of(t, "taxes_id")).ok().map(|tx| tax::compute(1.0, num(t, "list_price"), 0.0, &tx).2).unwrap_or(num(t, "list_price"));
                Some(map(vec![("id", p["id"].clone()), ("name", t["name"].clone()), ("price", price.into()), ("description", text(t, "public_description").or_else(|| text(t, "description_sale")).unwrap_or_default().into()), ("category_ids", t.get("pos_categ_ids").cloned().unwrap_or(Value::List(vec![])))]))
            }).collect();
            let cats = orm::search_read(&e, "pos.category", &Domain::True, &["id".into(), "name".into(), "parent_id".into()], Some("sequence, id"), None, 0).unwrap_or_default().into_iter().map(Value::Map).collect();
            let sym = id_of(&cfg, "company_id").and_then(|c| rec(&e, "res.company", c).ok()).and_then(|c| id_of(&c, "currency_id")).and_then(|c| rec(&e, "res.currency", c).ok()).and_then(|c| text(&c, "symbol")).unwrap_or_else(|| "$".into());
            let table = kw.get("table").and_then(|v| v.as_i64()).and_then(|n| find_one(&e, "restaurant.table", term("table_number", "=", n)).ok().flatten());
            Ok(map(vec![("name", text(&cfg, "name").unwrap_or_default().into()), ("mode", text(&cfg, "self_ordering_mode").unwrap_or_default().into()), ("service_mode", text(&cfg, "self_ordering_service_mode").unwrap_or_default().into()),
                ("takeaway", Value::Bool(flag(&cfg, "self_ordering_takeaway"))), ("pay_after", text(&cfg, "self_ordering_pay_after").unwrap_or_default().into()), ("open", Value::Bool(sess.is_some())), ("currency", sym.into()),
                ("table", table.map_or(Value::Null, Value::Int)), ("categories", Value::List(cats)), ("products", Value::List(products))]))
        })
        .action("pos.config", "kiosk_order", |env, ids, kw| {
            let e = env.sudo(); let id = *ids.first().unwrap_or(&0); let cfg = gate(&e, id, kw)?;
            if text(&cfg, "self_ordering_mode").as_deref() == Some("consultation") { return Err(OdooError::User("This menu is view-only — please order at the counter.".into())); }
            let sid = open_session(&e, id)?;
            let uuid = text(kw, "uuid").unwrap_or_default();
            if !uuid.starts_with(PREFIX) || uuid.len() < 12 || uuid.len() > 64 { return Err(OdooError::User("Invalid order".into())); }
            let lines: Vec<Value> = match kw.get("lines") { Some(Value::List(l)) => l.clone(), _ => vec![] };
            if lines.is_empty() || lines.len() > 40 { return Err(OdooError::User("Your order is empty".into())); }
            // visitors choose products and quantities only — prices, discounts and extra fields are dropped
            let mut clean = vec![];
            for l in &lines {
                let l = if let Value::Map(m) = l { m } else { continue };
                let pid = l.get("product_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("Invalid product".into()))?;
                let qty = num(l, "qty"); if !(1.0..=50.0).contains(&qty) || qty.fract() != 0.0 { return Err(OdooError::User("Invalid quantity".into())); }
                let p = rec(&e, "product.product", pid).map_err(|_| OdooError::User("Unknown product".into()))?;
                let t = id_of(&p, "product_tmpl_id").map(|t| rec(&e, "product.template", t)).transpose()?.unwrap_or_default();
                if !flag(&t, "available_in_pos") { return Err(OdooError::User("That product is not available".into())); }
                clean.push(Value::Map(row(&[("uuid", format!("{}-{}", uuid, clean.len()).into()), ("product_id", pid.into()), ("qty", qty.into()), ("note", text(l, "note").unwrap_or_default().chars().take(120).collect::<String>().into())])));
            }
            if let Some(o) = find_one(&e, "pos.order", term("uuid", "=", uuid.as_str()))? { if text(&rec(&e, "pos.order", o)?, "state").as_deref() != Some("draft") { return status_of(&e, &uuid); } }
            let mut k = row(&[("session_id", sid.into()), ("uuid", uuid.clone().into()), ("lines", Value::List(clean)), ("takeaway", Value::Bool(flag(kw, "takeaway"))), ("customer_count", 1.into()),
                ("note", { let mut n = String::from("Self-order"); if let Some(s) = text(kw, "stand").filter(|s| !s.is_empty()) { n.push_str(&format!(" · stand {}", s.chars().take(12).collect::<String>())); } n.into() })]);
            if let Some(n) = kw.get("table").and_then(|v| v.as_i64()) { if let Some(t) = find_one(&e, "restaurant.table", term("table_number", "=", n))? { k.insert("table_id".into(), t.into()); } }
            let user = id_of(&cfg, "self_ordering_default_user_id").unwrap_or(1);
            let acting = env.sudo(); let _ = user;
            let f = env.rules.actions.get(&("pos.order".to_string(), "send_to_kitchen".to_string())).ok_or_else(|| OdooError::User("Kitchen not available".into()))?;
            let v = f(&acting, &[], &k)?;
            let number = short(&uuid);
            let mut out = match v { Value::Map(m) => m, _ => Row::new() }; out.insert("number".into(), number.into());
            Ok(Value::Map(pick_keys(&out, &["uuid", "amount_total", "number"])))
        })
        .action("pos.config", "kiosk_status", |env, ids, kw| { let e = env.sudo(); gate(&e, *ids.first().unwrap_or(&0), kw)?; status_of(&e, &text(kw, "uuid").unwrap_or_default()) })
}
fn ids_of(r: &Row, k: &str) -> Vec<i64> { ids(r, k) }
fn pick_keys(r: &Row, keys: &[&str]) -> Row { keys.iter().filter_map(|k| r.get(*k).map(|v| (k.to_string(), v.clone()))).collect() }

/// received → preparing → ready, or paid once the staff took the payment.
fn status_of(env: &Env, uuid: &str) -> Result<Value> {
    if !uuid.starts_with(PREFIX) { return Err(OdooError::User("Unknown order".into())); }
    let oid = find_one(env, "pos.order", term("uuid", "=", uuid))?.ok_or_else(|| OdooError::User("Unknown order".into()))?;
    let o = rec(env, "pos.order", oid)?;
    let state = text(&o, "state").unwrap_or_default();
    let mut stage = if state == "draft" { "received" } else { "paid" };
    if env.reg.model("pos_preparation_display.ticket").is_ok() {
        let ts = orm::search(env, "pos_preparation_display.ticket", &term("order_id", "=", oid), None, None, 0)?;
        let ls: Vec<Row> = ts.iter().flat_map(|t| children(env, "pos_preparation_display.line", "ticket_id", *t).unwrap_or_default()).filter(|l| !flag(l, "cancelled")).collect();
        if !ls.is_empty() { if ls.iter().all(|l| text(l, "state").as_deref() == Some("done")) { stage = "ready"; } else if ls.iter().any(|l| text(l, "state").as_deref() != Some("new")) { stage = "preparing"; } }
    }
    let _ = pos_restaurant::has_restaurant;
    Ok(map(vec![("uuid", uuid.into()), ("number", short(uuid).into()), ("total", num(&o, "amount_total").into()), ("stage", stage.into()), ("paid", Value::Bool(state != "draft"))]))
}

/// Guest-facing order number: the last four characters of the order uuid, e.g. `#A3F2`.
fn short(uuid: &str) -> String { format!("#{}", uuid.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect::<String>().to_uppercase()) }
