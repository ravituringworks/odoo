//! pos_restaurant: floors & tables, open table orders (drafts shared by every terminal), kitchen tickets.
use crate::util::*;
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, OdooError, Result, Row, Rules, Value};
use std::collections::BTreeMap;

fn list(rows: Vec<Row>) -> Value { Value::List(rows.into_iter().map(Value::Map).collect()) }
fn pick(r: &Row, keys: &[&str]) -> Row { keys.iter().filter_map(|k| r.get(*k).map(|v| (k.to_string(), v.clone()))).collect() }
fn as_map(v: &Value) -> Row { if let Value::Map(m) = v { m.clone() } else { Row::new() } }
fn flag(r: &Row, k: &str) -> bool { r.get(k).map_or(false, |v| v.truthy()) }
fn kw_list<'a>(kw: &'a Row, k: &str) -> &'a [Value] { match kw.get(k) { Some(Value::List(l)) => l, _ => &[] } }

pub fn has_restaurant(env: &Env) -> bool { env.reg.model("restaurant.table").is_ok() && env.reg.field("pos.order", "table_id").is_ok() }

/// Floors (of this register) with their tables and how much is open on each.
fn floor_plan(env: &Env, cfg: &Row) -> Result<Value> {
    if !has_restaurant(env) { return Ok(Value::List(vec![])); }
    let cid = cfg["id"].as_i64().unwrap_or(0);
    let floors = ids(cfg, "floor_ids");
    let open = open_orders(env, cid, None)?;
    let mut out = vec![];
    for f in floors {
        let fl = rec(env, "restaurant.floor", f)?; if !flag(&fl, "active") && fl.contains_key("active") { continue; }
        let tables = children(env, "restaurant.table", "floor_id", f)?.into_iter().filter(|t| !t.contains_key("active") || flag(t, "active")).map(|t| {
            let tid = t["id"].as_i64().unwrap();
            let mine: Vec<&Row> = open.iter().filter(|o| id_of(o, "table_id") == Some(tid)).collect();
            let mut r = pick(&t, &["id", "table_number", "seats", "shape", "position_h", "position_v", "width", "height", "color"]);
            if let Some(n) = t.get("table_number") { r.insert("name".into(), Value::Text(format!("{}", n.as_i64().map(|i| i.to_string()).unwrap_or_default()))); }
            r.insert("orders".into(), Value::Int(mine.len() as i64)); r.insert("total".into(), Value::Float(r2(mine.iter().map(|o| num(o, "amount_total")).sum())));
            r
        }).collect();
        let mut fr = pick(&fl, &["id", "name", "background_color"]); fr.insert("tables".into(), list(tables)); out.push(Value::Map(fr));
    }
    Ok(Value::List(out))
}
pub fn plan(env: &Env, cfg: &Row) -> Result<Value> { floor_plan(env, cfg) }

/// Draft orders of the register, optionally for one table.
fn open_orders(env: &Env, config: i64, table: Option<i64>) -> Result<Vec<Row>> {
    let mut dom = vec![term("config_id", "=", config), term("state", "=", "draft")];
    if let Some(t) = table { dom.push(term("table_id", "=", t)); }
    let ids = orm::search(env, "pos.order", &Domain::And(dom), Some("id"), None, 0)?;
    ids.into_iter().map(|i| rec(env, "pos.order", i)).collect()
}
fn draft_view(env: &Env, o: &Row) -> Result<Value> {
    let lines = children(env, "pos.order.line", "order_id", o["id"].as_i64().unwrap())?.into_iter().map(|l| pick(&l, &["uuid", "product_id", "qty", "price_unit", "discount", "customer_note"])).collect();
    let mut v = pick(o, &["id", "uuid", "table_id", "customer_count", "takeaway", "partner_id", "amount_total", "general_note", "shipping_date", "name", "last_order_preparation_change"]);
    // many2one as [id, label] like `read` does, so the terminal can show the customer and table name
    if let Some(p) = id_of(o, "partner_id") { if let Ok(r) = rec(env, "res.partner", p) { v.insert("partner_id".into(), Value::List(vec![p.into(), text(&r, "name").unwrap_or_default().into()])); } }
    if let Some(t) = id_of(o, "table_id") { if let Ok(r) = rec(env, "restaurant.table", t) { v.insert("table_id".into(), Value::List(vec![t.into(), format!("Table {}", num(&r, "table_number") as i64).into()])); } }
    v.insert("lines".into(), list(lines)); Ok(Value::Map(v))
}

/// Parse `{line_uuid: {qty, name, note, product_id}}` stored in `last_order_preparation_change`.
fn sent_map(o: &Row) -> BTreeMap<String, Row> {
    text(o, "last_order_preparation_change").and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()).and_then(|j| if let serde_json::Value::Object(m) = j { Some(m) } else { None })
        .map(|m| m.into_iter().map(|(k, v)| (k, match Value::from_json(&v) { Value::Map(m) => m, _ => Row::new() })).collect()).unwrap_or_default()
}

pub fn rules() -> Rules {
    Rules::default()
        .action("pos.config", "floor_plan", |env, ids, _| { let cfg = rec(env, "pos.config", *ids.first().unwrap_or(&0))?; floor_plan(env, &cfg) })
        .action("pos.config", "open_orders", |env, ids, kw| {
            let cid = *ids.first().unwrap_or(&0);
            Ok(list(open_orders(env, cid, kw.get("table_id").and_then(|v| v.as_i64()))?.iter().map(|o| as_map(&draft_view(env, o).unwrap_or_default())).collect()))
        })
        // Persist a table order so every terminal (and a page refresh) sees it. Idempotent by uuid.
        .action("pos.order", "save_draft", |env, _, kw| save_draft(env, kw).map(|(_, v)| v))
        // Send the new/changed/cancelled items to the kitchen as one ticket.
        .action("pos.order", "send_to_kitchen", |env, _, kw| {
            let (oid, view) = save_draft(env, kw)?; let e = env.sudo();
            let o = rec(&e, "pos.order", oid)?; let before = sent_map(&o);
            let lines = children(&e, "pos.order.line", "order_id", oid)?;
            let mut now: BTreeMap<String, Value> = BTreeMap::new(); let mut tl: Vec<Row> = vec![];
            for l in &lines {
                let key = text(l, "uuid").unwrap_or_default(); if key.is_empty() { continue; }
                let (q, was) = (num(l, "qty"), before.get(&key).map_or(0.0, |b| num(b, "qty")));
                let name = text(l, "full_product_name").unwrap_or_default();
                now.insert(key.clone(), Value::Map(row(&[("qty", q.into()), ("name", name.clone().into()), ("note", text(l, "customer_note").unwrap_or_default().into())])));
                if (q - was).abs() > 1e-9 { tl.push(row(&[("product_id", id_of(l, "product_id").map_or(Value::Null, Value::Int)), ("name", name.into()), ("qty", (q - was).abs().into()), ("cancelled", (q < was).into()), ("note", text(l, "customer_note").unwrap_or_default().into())])); }
            }
            for (k, b) in &before { if !now.contains_key(k) { tl.push(row(&[("product_id", Value::Null), ("name", text(b, "name").unwrap_or_default().into()), ("qty", num(b, "qty").into()), ("cancelled", true.into()), ("note", "".into())])); now.insert(k.clone(), Value::Map(row(&[("qty", 0.0.into()), ("name", text(b, "name").unwrap_or_default().into())]))); } }
            let mut ticket = Value::Null;
            if !tl.is_empty() && e.reg.model("pos_preparation_display.ticket").is_ok() {
                let table = id_of(&o, "table_id");
                let label = table.and_then(|t| rec(&e, "restaurant.table", t).ok()).map(|t| format!("Table {}", num(&t, "table_number") as i64)).unwrap_or_else(|| if flag(&o, "takeaway") { "Takeaway".into() } else { text(&o, "name").unwrap_or_default() });
                let mut tv = row(&[("name", label.into()), ("config_id", id_of(&o, "config_id").unwrap_or(0).into()), ("order_id", oid.into()), ("takeaway", flag(&o, "takeaway").into()), ("note", text(&o, "general_note").unwrap_or_default().into())]);
                if let Some(t) = table { tv.insert("table_id".into(), t.into()); }
                let tid = orm::create(&e, "pos_preparation_display.ticket", tv)?;
                for mut l in tl { l.insert("ticket_id".into(), tid.into()); orm::create(&e, "pos_preparation_display.line", l)?; }
                ticket = Value::Int(tid);
            }
            let json = serde_json::to_string(&Value::Map(now.into_iter().collect()).to_json()).unwrap_or_default();
            orm::write(&e, "pos.order", &[oid], row(&[("last_order_preparation_change", json.into())]))?;
            let mut v = as_map(&view); v.insert("ticket_id".into(), ticket); Ok(Value::Map(v))
        })
        .action("pos.order", "transfer_table", |env, _, kw| {
            let e = env.sudo(); let uuid = text(kw, "uuid").unwrap_or_default(); let table = kw.get("table_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("Select a table".into()))?;
            let oid = find_one(&e, "pos.order", term("uuid", "=", uuid.as_str()))?.ok_or_else(|| OdooError::User("Order not found".into()))?;
            orm::write(&e, "pos.order", &[oid], row(&[("table_id", table.into())]))?; Ok(Value::Bool(true))
        })
        .action("pos.order", "discard_draft", |env, _, kw| {
            let e = env.sudo(); let uuid = text(kw, "uuid").unwrap_or_default();
            if let Some(oid) = find_one(&e, "pos.order", term("uuid", "=", uuid.as_str()))? { if text(&rec(&e, "pos.order", oid)?, "state").as_deref() == Some("draft") { orm::unlink(&e, "pos.order", &[oid])?; } }
            Ok(Value::Bool(true))
        })
        // Kitchen display feed: everything not yet served, oldest first.
        .action("pos.config", "kitchen_tickets", |env, ids, kw| {
            let e = env.sudo(); let cid = *ids.first().unwrap_or(&0);
            if e.reg.model("pos_preparation_display.ticket").is_err() { return Ok(Value::List(vec![])); }
            let cats: Vec<i64> = match kw.get("category_ids") { Some(Value::List(l)) => l.iter().filter_map(|v| v.as_i64()).collect(), _ => vec![] };
            let tids = orm::search(&e, "pos_preparation_display.ticket", &Domain::And(vec![term("config_id", "=", cid), term("state", "!=", "done")]), Some("id"), None, 0)?;
            let mut out = vec![];
            for t in tids { let tk = rec(&e, "pos_preparation_display.ticket", t)?;
                let mut ls = children(&e, "pos_preparation_display.line", "ticket_id", t)?;
                if !cats.is_empty() { ls.retain(|l| { let lc = ids_of(l, "category_ids"); lc.is_empty() || lc.iter().any(|c| cats.contains(c)) }); }
                if ls.is_empty() { continue; }
                let mut v = pick(&tk, &["id", "name", "state", "takeaway", "note", "create_date"]);
                v.insert("lines".into(), list(ls.iter().map(|l| pick(l, &["id", "name", "qty", "note", "state", "cancelled"])).collect())); out.push(Value::Map(v)); }
            Ok(Value::List(out))
        })
        // advance one line (new → cooking → done); the ticket is served when every line is done
        .action("pos_preparation_display.line", "advance", |env, ids, kw| {
            let e = env.sudo(); let to = text(kw, "state").unwrap_or_default();
            if !matches!(to.as_str(), "new" | "cooking" | "done") { return Err(OdooError::User("Unknown state".into())); }
            for id in ids { orm::write(&e, "pos_preparation_display.line", &[*id], row(&[("state", to.as_str().into())]))?;
                let l = rec(&e, "pos_preparation_display.line", *id)?; if let Some(t) = id_of(&l, "ticket_id") {
                    let all = children(&e, "pos_preparation_display.line", "ticket_id", t)?;
                    let st = if all.iter().all(|x| text(x, "state").as_deref() == Some("done") || flag(x, "cancelled")) { "done" } else if all.iter().any(|x| text(x, "state").as_deref() != Some("new")) { "cooking" } else { "new" };
                    orm::write(&e, "pos_preparation_display.ticket", &[t], row(&[("state", st.into())]))?; } }
            Ok(Value::Bool(true))
        })
        .action("pos_preparation_display.ticket", "bump", |env, ids, _| {
            let e = env.sudo();
            for t in ids { for l in children(&e, "pos_preparation_display.line", "ticket_id", *t)? { orm::write(&e, "pos_preparation_display.line", &[l["id"].as_i64().unwrap()], row(&[("state", "done".into())]))?; }
                orm::write(&e, "pos_preparation_display.ticket", &[*t], row(&[("state", "done".into())]))?; }
            Ok(Value::Bool(true))
        })
}
fn ids_of(r: &Row, k: &str) -> Vec<i64> { ids(r, k) }

fn save_draft(env: &Env, kw: &Row) -> Result<(i64, Value)> {
    let e = env.sudo();
    let sid = kw.get("session_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("Missing POS session".into()))?;
    let sess = rec(&e, "pos.session", sid)?;
    if text(&sess, "state").as_deref() != Some("opened") { return Err(OdooError::User("This POS session is closed.".into())); }
    let uuid = text(kw, "uuid").filter(|u| !u.is_empty()).ok_or_else(|| OdooError::User("Missing order uuid".into()))?;
    let cfg = rec(&e, "pos.config", id_of(&sess, "config_id").unwrap_or(0))?;
    let built = crate::pos::draft_lines(&e, &cfg, kw)?;
    let mut ov = row(&[("session_id", sid.into()), ("config_id", id_of(&sess, "config_id").unwrap_or(0).into()), ("state", "draft".into()), ("uuid", uuid.clone().into()), ("amount_total", built.2.into()), ("amount_tax", built.1.into()), ("amount_paid", 0.0.into()), ("amount_return", 0.0.into()), ("date_order", orm::now().into()), ("user_id", env.uid.into()),
        ("customer_count", kw.get("customer_count").cloned().unwrap_or(Value::Int(1))), ("takeaway", Value::Bool(flag(kw, "takeaway"))), ("general_note", text(kw, "note").unwrap_or_default().into())]);
    if let Some(t) = kw.get("table_id").and_then(|v| v.as_i64()) { ov.insert("table_id".into(), t.into()); }
    if let Some(d) = text(kw, "shipping_date").filter(|d| !d.is_empty()) { ov.insert("shipping_date".into(), d.into()); }
    if let Some(p) = kw.get("partner_id").and_then(|v| v.as_i64()) { ov.insert("partner_id".into(), p.into()); }
    if let Some(c) = id_of(&sess, "company_id").or_else(|| id_of(&cfg, "company_id")) { ov.insert("company_id".into(), c.into()); }
    let existing = find_one(&e, "pos.order", term("uuid", "=", uuid.as_str()))?;
    if let Some(o) = existing { if text(&rec(&e, "pos.order", o)?, "state").as_deref() != Some("draft") { return Err(OdooError::User("This order is already paid".into())); } }
    ov.retain(|k, _| e.reg.field("pos.order", k).is_ok());
    let oid = match existing {
        Some(o) => { for l in children(&e, "pos.order.line", "order_id", o)? { orm::unlink(&e, "pos.order.line", &[l["id"].as_i64().unwrap()])?; } orm::write(&e, "pos.order", &[o], ov)?; o }
        None => { ov.insert("name".into(), format!("Draft {}", &uuid[..uuid.len().min(8)]).into()); orm::create(&e, "pos.order", ov)? }
    };
    crate::pos::insert_lines(&e, oid, built.0)?;
    let o = rec(&e, "pos.order", oid)?; Ok((oid, draft_view(&e, &o)?))
}
