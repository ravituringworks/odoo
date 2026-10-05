//! Online shop on top of the self-order engine: public catalogue with images, customer details, pickup or delivery.
//! With `sale` installed an order becomes a confirmed `sale.order` (and its delivery follows the normal sale flow); otherwise it is a
//! pickup order on the register. Same token gate and hostile-input rules as `pos_kiosk`.
use crate::{pos_kiosk::gate, util::*};
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, OdooError, Result, Row, Rules, Value};

const PREFIX: &str = "shop-";
fn flag(r: &Row, k: &str) -> bool { r.get(k).map_or(false, |v| v.truthy()) }
fn map(pairs: Vec<(&str, Value)>) -> Value { Value::Map(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()) }
fn clean(s: Option<String>, max: usize) -> String { s.unwrap_or_default().chars().filter(|c| !c.is_control()).take(max).collect::<String>().trim().to_string() }
fn email_ok(e: &str) -> bool { let Some((l, d)) = e.split_once('@') else { return false }; !l.is_empty() && d.contains('.') && !d.starts_with('.') && !d.ends_with('.') && e.len() <= 120 && !e.contains(' ') }

/// At most 120 shop orders per register per hour (the counter lives in `ir.config_parameter`).
fn rate_limit(env: &Env, cfg: i64) -> Result<()> {
    let hour = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() / 3600).unwrap_or(0) as i64;
    let key = format!("pos.shop.rate.{cfg}"); let slot = find_one(env, "ir.config_parameter", term("key", "=", key.as_str()))?;
    let (h, n) = slot.and_then(|p| rec(env, "ir.config_parameter", p).ok()).and_then(|p| text(&p, "value")).and_then(|v| { let (a, b) = v.split_once('|')?; Some((a.parse::<i64>().ok()?, b.parse::<i64>().ok()?)) }).unwrap_or((hour, 0));
    let n = if h == hour { n } else { 0 };
    if n >= 120 { return Err(OdooError::User("Too many orders right now — please try again later".into())); }
    let v = format!("{hour}|{}", n + 1);
    match slot { Some(p) => orm::write(env, "ir.config_parameter", &[p], row(&[("value", v.into())])), None => orm::create(env, "ir.config_parameter", row(&[("key", key.into()), ("value", v.into())])).map(|_| ()) }
}

pub fn rules() -> Rules {
    Rules::default()
        // small product pictures for the catalogue page (data URLs), a few at a time
        .action("pos.config", "kiosk_images", |env, ids, kw| {
            let e = env.sudo(); gate(&e, *ids.first().unwrap_or(&0), kw)?;
            let want: Vec<i64> = match kw.get("ids") { Some(Value::List(l)) => l.iter().filter_map(|v| v.as_i64()).take(24).collect(), _ => vec![] };
            let mut out = std::collections::BTreeMap::new();
            if e.reg.field("product.template", "image_128").is_err() { return Ok(Value::Map(out)); }
            for pid in want {
                let Some(p) = rec(&e, "product.product", pid).ok() else { continue };
                let Some(t) = id_of(&p, "product_tmpl_id").and_then(|t| rec(&e, "product.template", t).ok()) else { continue };
                if !flag(&t, "available_in_pos") { continue; }
                if let Some(b) = text(&t, "image_128").filter(|b| !b.is_empty() && b.len() < 400_000 && b.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '='))) {
                    let mime = if b.starts_with("/9j/") { "jpeg" } else if b.starts_with("iVBOR") { "png" } else if b.starts_with("UklG") { "webp" } else if b.starts_with("R0lG") { "gif" } else { continue };   // no SVG: it can carry script
                    out.insert(pid.to_string(), Value::Text(format!("data:image/{mime};base64,{b}")));
                }
            }
            Ok(Value::Map(out))
        })
        .action("pos.config", "kiosk_checkout", |env, ids, kw| {
            let e = env.sudo(); let cfg_id = *ids.first().unwrap_or(&0); let cfg = gate(&e, cfg_id, kw)?;
            if text(&cfg, "self_ordering_mode").as_deref() == Some("consultation") { return Err(OdooError::User("This shop is view-only".into())); }
            let uuid = text(kw, "uuid").unwrap_or_default();
            if !uuid.starts_with(PREFIX) || uuid.len() < 12 || uuid.len() > 64 || !uuid.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') { return Err(OdooError::User("Invalid order".into())); }
            let c = match kw.get("customer") { Some(Value::Map(m)) => m.clone(), _ => return Err(OdooError::User("Enter your details".into())) };
            let (name, email, phone) = (clean(text(&c, "name"), 80), clean(text(&c, "email"), 120).to_lowercase(), clean(text(&c, "phone"), 30));
            if name.is_empty() { return Err(OdooError::User("Please enter your name".into())); }
            if !email_ok(&email) { return Err(OdooError::User("Please enter a valid email address".into())); }
            let delivery = text(kw, "delivery").as_deref() == Some("delivery");
            let (street, city, zip) = (clean(text(&c, "street"), 120), clean(text(&c, "city"), 60), clean(text(&c, "zip"), 16));
            if delivery && (street.is_empty() || city.is_empty() || zip.is_empty()) { return Err(OdooError::User("Please enter your delivery address".into())); }
            let lines: Vec<Value> = match kw.get("lines") { Some(Value::List(l)) => l.clone(), _ => vec![] };
            if lines.is_empty() || lines.len() > 40 { return Err(OdooError::User("Your cart is empty".into())); }
            let mut picked: Vec<(i64, f64)> = vec![];
            for l in &lines {
                let l = if let Value::Map(m) = l { m } else { continue };
                let pid = l.get("product_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("Invalid product".into()))?; let qty = num(l, "qty");
                if !(1.0..=50.0).contains(&qty) || qty.fract() != 0.0 { return Err(OdooError::User("Invalid quantity".into())); }
                let p = rec(&e, "product.product", pid).map_err(|_| OdooError::User("Unknown product".into()))?;
                let t = id_of(&p, "product_tmpl_id").and_then(|t| rec(&e, "product.template", t).ok()).unwrap_or_default();
                if !flag(&t, "available_in_pos") { return Err(OdooError::User("That product is not available".into())); }
                picked.push((pid, qty));
            }
            let note = format!("Online order · {} · {}{}", if delivery { "delivery" } else { "pickup" }, clean(text(kw, "note"), 200), if delivery { format!(" · {street}, {zip} {city}") } else { String::new() });
            let sale = e.reg.model("sale.order").is_ok() && e.reg.field("sale.order", "client_order_ref").is_ok();
            // idempotent retries
            if sale { if let Some(o) = find_one(&e, "sale.order", term("client_order_ref", "=", uuid.as_str()))? { let r = rec(&e, "sale.order", o)?; return Ok(map(vec![("uuid", uuid.into()), ("number", text(&r, "name").unwrap_or_default().into()), ("total", num(&r, "amount_total").into()), ("path", "sale".into())])); } }
            rate_limit(&e, cfg_id)?;
            // the customer: an existing contact with this email is reused as-is (public input never edits existing records)
            let partner = match find_one(&e, "res.partner", term("email", "=", email.as_str()))? {
                Some(p) => p,
                None => { let mut v = row(&[("name", name.clone().into()), ("email", email.clone().into())]); if !phone.is_empty() { v.insert("phone".into(), phone.clone().into()); } if delivery { v.insert("street".into(), street.into()); v.insert("city".into(), city.into()); v.insert("zip".into(), zip.into()); } v.retain(|k, _| e.reg.field("res.partner", k).is_ok()); orm::create(&e, "res.partner", v)? }
            };
            if sale {
                let lines: Vec<Value> = picked.iter().map(|(p, q)| Value::List(vec![0.into(), 0.into(), Value::Map(row(&[("product_id", (*p).into()), ("product_uom_qty", (*q).into())]))])).collect();
                let oid = orm::create(&e, "sale.order", row(&[("partner_id", partner.into()), ("client_order_ref", uuid.clone().into()), ("order_line", Value::List(lines)), ("note", note.into())]))?;
                orm::call(&e, "sale.order", "action_confirm", &[oid], &Row::new())?;
                let r = rec(&e, "sale.order", oid)?;
                Ok(map(vec![("uuid", uuid.into()), ("number", text(&r, "name").unwrap_or_default().into()), ("total", num(&r, "amount_total").into()), ("path", "sale".into())]))
            } else {
                let sid = crate::pos_kiosk::open_session(&e, cfg_id)?;
                let pos_uuid = format!("kiosk-{uuid}");
                let mut k = row(&[("session_id", sid.into()), ("uuid", pos_uuid.clone().into()), ("partner_id", partner.into()), ("customer_count", 1.into()), ("note", note.into()),
                    ("lines", Value::List(picked.iter().enumerate().map(|(i, (p, q))| Value::Map(row(&[("uuid", format!("{pos_uuid}-{i}").into()), ("product_id", (*p).into()), ("qty", (*q).into())]))).collect()))]);
                if let Some(at) = text(kw, "pickup_at").filter(|a| a.len() <= 5 && a.chars().all(|c| c.is_ascii_digit() || c == ':')) { k.insert("shipping_date".into(), format!("{} {}", orm::today(), at).into()); }
                let f = env.rules.actions.get(&("pos.order".to_string(), "send_to_kitchen".to_string())).ok_or_else(|| OdooError::User("Orders are not available".into()))?;
                let v = f(&e, &[], &k)?;
                let total = if let Value::Map(m) = &v { num(m, "amount_total") } else { 0.0 };
                Ok(map(vec![("uuid", pos_uuid.clone().into()), ("number", format!("#{}", pos_uuid.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect::<String>().to_uppercase()).into()), ("total", total.into()), ("path", "pos".into())]))
            }
        })
        // status of a shop order placed through the sale flow
        .action("pos.config", "kiosk_shop_status", |env, ids, kw| {
            let e = env.sudo(); gate(&e, *ids.first().unwrap_or(&0), kw)?;
            let uuid = text(kw, "uuid").unwrap_or_default(); if !uuid.starts_with(PREFIX) { return Err(OdooError::User("Unknown order".into())); }
            let o = find_one(&e, "sale.order", term("client_order_ref", "=", uuid.as_str()))?.ok_or_else(|| OdooError::User("Unknown order".into()))?;
            let r = rec(&e, "sale.order", o)?; let name = text(&r, "name").unwrap_or_default();
            let delivered = e.reg.model("stock.picking").is_ok() && { let ps = orm::search(&e, "stock.picking", &term("origin", "=", name.as_str()), None, None, 0)?; !ps.is_empty() && ps.iter().all(|p| rec(&e, "stock.picking", *p).map_or(false, |x| text(&x, "state").as_deref() == Some("done"))) };
            Ok(map(vec![("uuid", uuid.into()), ("number", name.into()), ("total", num(&r, "amount_total").into()), ("stage", (if delivered { "ready" } else if text(&r, "state").as_deref() == Some("cancel") { "cancelled" } else { "received" }).into()), ("paid", Value::Bool(false))]))
        })
}
